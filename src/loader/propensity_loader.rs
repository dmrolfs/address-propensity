use crate::core::domain::property::PropertyRecordRepository;
use crate::core::domain::{PropertyPropensityScore, PropertyPropensityScoreRepository};
use crate::loader::domain::CsvPropertyPropensityScore;
use crate::loader::settings::Settings;
use crate::loader::LoaderError;
use sqlx::PgPool;
use std::convert::TryInto;
use std::fmt;
use std::path::PathBuf;
use validator::{Validate, ValidationErrors};

#[derive(Default)]
struct QualityMeasure {
    pub deserialization_failures: Vec<(usize, anyhow::Error)>,
    pub validation_failures: Vec<(usize, ValidationErrors)>,
    pub save_failures: Vec<(CsvPropertyPropensityScore, anyhow::Error)>,
    pub missing_scores: Vec<usize>,
    pub not_in_core_properties: Vec<PropertyPropensityScore>,
}

impl fmt::Debug for QualityMeasure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nr_issues = self.deserialization_failures.len()
            + self.validation_failures.len()
            + self.save_failures.len()
            + self.missing_scores.len();

        let mut result =
            f.write_str(format!("{} issues found{}", nr_issues, if 0 < nr_issues { ":" } else { "" }).as_str());

        if result.is_ok() && 0 < nr_issues {
            if !self.deserialization_failures.is_empty() {
                result = f.write_str(
                    format!("\n\t{} deserialization failures", self.deserialization_failures.len()).as_str(),
                );
            }
            if !self.validation_failures.is_empty() {
                result = f.write_str(format!("\n\t{} validation failures", self.validation_failures.len()).as_str());
            }
            if !self.save_failures.is_empty() {
                result = f.write_str(format!("\n\t{} save failures", self.save_failures.len()).as_str());
            }
            if !self.missing_scores.is_empty() {
                result = f.write_str(format!("\n\t{} missing scores", self.missing_scores.len()).as_str());
            }
            if !self.not_in_core_properties.is_empty() {
                result =
                    f.write_str(format!("\n\t{} not in core properties", self.not_in_core_properties.len()).as_str());
            }

            result
        } else {
            result
        }
    }
}

//todo: there's likely similarity between property loading and propensity loading. Future work to unify.
#[tracing::instrument(level = "info")]
pub async fn load_propensity_data(
    file: PathBuf, distribution: Option<PathBuf>, settings: Settings,
) -> Result<(), LoaderError> {
    let mut reader = csv::Reader::from_path(&file)?;
    let mut quality = QualityMeasure::default();
    let mut skipped_records = vec![];

    let connection_pool = crate::core::get_connection_pool(&settings.database)
        .await
        .expect("Failed to connect to Postgres database.");

    let mut nr_valid_records: usize = 0;
    let mut nr_saved_records: usize = 0;

    tracing::info!("loading propensity records from source file: {:?}", file);
    for (pos, record) in reader.deserialize().enumerate() {
        let idx = pos + 1;
        let record: Result<CsvPropertyPropensityScore, csv::Error> = record;
        let ingress = match record {
            Ok(ref property_propensity) => {
                if property_propensity.propensity_score.is_none() {
                    tracing::warn!(
                        ?record,
                        "propensity record[{}] does not have a propensity score - skipping.",
                        idx
                    );
                    quality.missing_scores.push(idx);
                    skipped_records.push(idx);
                    continue;
                }
                property_propensity
            }
            Err(err) => {
                tracing::error!(error=?err, record_index=%idx, "failed to load propensity record[{}]", idx);
                quality.deserialization_failures.push((idx, err.into()));
                skipped_records.push(idx);
                continue;
            }
        };
        tracing::debug!(?ingress, "deserialized record[{}]", idx);

        if let Err(err) = ingress.validate() {
            tracing::error!(error=?err, "propensity record[{}] failed initial validation.", idx);
            quality.validation_failures.push((idx, err));
            skipped_records.push(idx);
            continue;
        }
        tracing::debug!(?ingress, "record[{}] validated", idx);

        // should never be None due to above check; however this is easy and resilient to future modification.
        let propensity_record: Result<Option<PropertyPropensityScore>, LoaderError> = ingress.clone().try_into();
        let propensity_record = match propensity_record {
            Ok(rec) => rec,
            Err(err) => {
                tracing::error!(error=?err, "failed to convert csv record into domain -- skipped");
                quality.deserialization_failures.push((idx, err.into()));
                skipped_records.push(idx);
                continue;
            }
        };

        let propensity_record = match propensity_record {
            Some(score) => score,
            None => {
                tracing::error!(
                    ?propensity_record,
                    "redundant propensity score failed - data load okay but check code"
                );
                tracing::warn!("Propensity record[{}] did not contain a score - skipping.", idx);
                quality.missing_scores.push(idx);
                skipped_records.push(idx);
                continue;
            }
        };

        nr_valid_records += 1;
        tracing::debug!(?propensity_record, %nr_valid_records, "csv record[{}] converted to save to database.", idx);

        let saved = save_record(
            &propensity_record,
            &connection_pool,
            idx,
            &ingress,
            &mut quality,
            &mut skipped_records,
        )
        .await;
        if saved {
            nr_saved_records += 1;
        }
    }

    tracing::warn!(
        "Saved {} records from {:?} ({} skipped) with {:?}",
        nr_saved_records,
        file,
        skipped_records.len(),
        quality
    );

    Ok(())
}

#[tracing::instrument(level = "info", skip(pool, csv_record, quality, skipped_records,))]
async fn save_record(
    record: &PropertyPropensityScore, pool: &PgPool, index: usize, csv_record: &CsvPropertyPropensityScore,
    quality: &mut QualityMeasure, skipped_records: &mut Vec<usize>,
) -> bool {
    match PropertyPropensityScoreRepository::find_for_apn(&record.apn, &pool).await {
        Err(err) => {
            tracing::error!(error=?err, apn=?record.apn, "error while checking if propensity record[{}] was previously loaded - skipping", index);
            skipped_records.push(index);
            false
        }

        Ok(Some(record)) => {
            tracing::info!(apn=?record.apn, "propensity record[{}] previously loaded - skipping", index);
            skipped_records.push(index);
            false
        }

        Ok(None) => {
            let matched = do_assess_for_property(pool, record).await.unwrap_or(false);
            if !matched {
                quality.not_in_core_properties.push(record.clone());
            }
            let save_span = tracing::info_span!("save", apn=%record.apn, %index,);
            let _save_span_guardian = save_span.enter();

            match do_save(pool, record, index).await {
                Ok(_score) => {
                    tracing::info!("saved property propensity score");
                    true
                }

                Err(err) => {
                    tracing::error!(error=?err, "failed to save property propensity score - skipping.");
                    quality.save_failures.push((csv_record.clone(), err.into()));
                    skipped_records.push(index);
                    false
                }
            }
        }
    }
}

#[tracing::instrument(level = "info", skip(pool))]
async fn do_save(
    pool: &PgPool, record: &PropertyPropensityScore, index: usize,
) -> Result<PropertyPropensityScore, LoaderError> {
    let mut transaction = pool
        .begin()
        .await
        .expect("Failed to acquire Postgres connection from the pool.");

    let nr_updated = PropertyPropensityScoreRepository::save(&mut transaction, record).await?;

    transaction
        .commit()
        .await
        .expect("Failed to commit SQL transaction to store loaded propensity record.");

    tracing::info!("Saved RECORD[{}]: => {:?}", index, nr_updated);
    Ok(nr_updated)
}

#[tracing::instrument(level = "info", skip(pool))]
async fn do_assess_for_property(pool: &PgPool, record: &PropertyPropensityScore) -> Result<bool, LoaderError> {
    let result = PropertyRecordRepository::find(&record.apn, pool).await?;
    Ok(result.is_some())
}
