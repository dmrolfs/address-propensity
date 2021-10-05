use crate::core::domain::property::{Property, PropertyRecordRepository};
use crate::loader::domain::CsvProperty;
use crate::loader::errors::LoaderError;
use crate::loader::settings::Settings;
use console::style;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use sqlx::PgPool;
use std::convert::TryInto;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use validator::{Validate, ValidationErrors};

#[derive(Default)]
struct QualityMeasure {
    pub deserialization_failures: Vec<(usize, anyhow::Error)>,
    pub validation_failures: Vec<(usize, ValidationErrors)>,
    pub save_failures: Vec<(CsvProperty, anyhow::Error)>,
}

impl fmt::Debug for QualityMeasure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nr_issues = self.deserialization_failures.len() + self.validation_failures.len() + self.save_failures.len();

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

            result
        } else {
            result
        }
    }
}

#[tracing::instrument(level = "info")]
fn count_nr_records(path: &PathBuf) -> Result<usize, LoaderError> {
    let reader = BufReader::new(File::open(path)?);
    Ok(reader.lines().count() - 1) // subtract header line
}

#[tracing::instrument(level = "info", skip(settings))]
pub async fn load_property_data(file: PathBuf, settings: Settings) -> Result<(), LoaderError> {
    let mut reader = csv::Reader::from_path(&file)?;
    let mut quality = QualityMeasure::default();
    let mut skipped_records = vec![];
    let nr_records = count_nr_records(&file)?;

    let connection_pool = crate::core::get_connection_pool(&settings.database)
        .await
        .expect("Failed to connect to Postgres database.");

    let mut nr_valid_records: usize = 0;
    let mut nr_saved_records: usize = 0;

    tracing::info!("loading property records from source file: {:?}", file);
    eprintln!(" {}...", style("Loading property records").bold());
    let sty = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} eta:{eta} processing:{per_sec}")
        .progress_chars("##-");

    let progress = ProgressBar::new(nr_records as u64);
    progress.set_style(sty);

    for (pos, record) in reader.deserialize().enumerate().progress_with(progress) {
        let idx = pos + 1;

        let ingress: CsvProperty = match record {
            Ok(property) => property,
            Err(err) => {
                tracing::error!(error=?err, record_index=%idx, "failed to load property record[{}]", idx);
                quality.deserialization_failures.push((idx, err.into()));
                skipped_records.push(idx);
                continue;
            }
        };
        tracing::debug!(?ingress, "deserialized record[{}]", idx);

        if let Err(err) = ingress.validate() {
            tracing::error!(error=?err, "property record[{}] failed initial validation", idx);
            quality.validation_failures.push((idx, err));
            skipped_records.push(idx);
            continue;
        }
        tracing::debug!(?ingress, "record[{}] validated", idx);

        let property_record: Result<Property, LoaderError> = ingress.clone().try_into();
        let property_record = match property_record {
            Ok(rec) => rec,
            Err(err) => {
                tracing::error!(error=?err, "failed to convert csv record into domain -- skipped");
                quality.deserialization_failures.push((idx, err.into()));
                skipped_records.push(idx);
                continue;
            }
        };

        nr_valid_records += 1;
        tracing::debug!(?property_record, %nr_valid_records, "csv record[{}] converted to save to database", idx);

        let saved = save_record(
            &property_record,
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

    summarize(nr_saved_records, &file, &skipped_records, &quality);
    Ok(())
}

#[tracing::instrument(level = "info", skip(nr_saved_records, file, skipped, quality))]
fn summarize(nr_saved_records: usize, file: &PathBuf, skipped: &[usize], quality: &QualityMeasure) {
    eprintln!(
        " {}",
        style(format!(
            "Saved {} records from {:?} ({} skipped) with {:?}",
            nr_saved_records,
            file,
            skipped.len(),
            quality
        ))
        .bold()
    );
    tracing::warn!(
        "Saved {} records from {:?} ({} skipped) with {:?}",
        nr_saved_records,
        file,
        skipped.len(),
        quality
    );

    if !skipped.is_empty() {
        let first: Vec<usize> = skipped.iter().take(10).copied().collect();
        eprintln!(
            " {}",
            style(format!(
                "indexes of first {} skipped csv records: {:?}",
                first.len(),
                first
            ))
            .bold()
        );
    }
}

#[tracing::instrument(level = "info", skip(pool, csv_record, quality, skipped_records))]
async fn save_record(
    record: &Property, pool: &PgPool, index: usize, csv_record: &CsvProperty, quality: &mut QualityMeasure,
    skipped_records: &mut Vec<usize>,
) -> bool {
    match PropertyRecordRepository::find(&record.apn, &pool).await {
        Err(err) => {
            tracing::error!(error=?err, apn=?record.apn, "error looking while checking if property record[{}] was previously loaded.", index);
            skipped_records.push(index);
            false
        }

        Ok(Some(record)) => {
            tracing::info!(apn=?record.apn, "property record[{}] previously loaded - skipping", index);
            skipped_records.push(index);
            false
        }

        Ok(None) => {
            let save_span = tracing::info_span!("save", apn=%record.apn, %index,);
            let _save_span_guardian = save_span.enter();

            match do_save(pool, record, index).await {
                Ok(_property) => {
                    tracing::info!("saved property record.");
                    true
                }

                Err(err) => {
                    tracing::error!(error=?err, "failed to save property record - skipping.");
                    quality.save_failures.push((csv_record.clone(), err.into()));
                    skipped_records.push(index);
                    false
                }
            }
        }
    }
}

#[tracing::instrument(level = "info", skip(pool))]
async fn do_save(pool: &PgPool, record: &Property, index: usize) -> Result<Property, LoaderError> {
    let mut transaction = pool
        .begin()
        .await
        .expect("Failed to acquire Postgres connection from the pool.");

    let nr_updated = PropertyRecordRepository::save(&mut transaction, &record).await?;

    transaction
        .commit()
        .await
        .expect("Failed to commit SQL transaction to store loaded property records.");

    tracing::info!("Saved RECORD[{}]: => {:?}", index, nr_updated,);
    Ok(nr_updated)
}
