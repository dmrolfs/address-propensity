use crate::core::domain::property::{Property, PropertyRecordRepository};
use crate::loader::domain::CsvProperty;
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
pub async fn load_property_data(file: PathBuf, settings: Settings) -> Result<(), LoaderError> {
    let mut reader = csv::Reader::from_path(&file)?;
    let mut quality = QualityMeasure::default();
    let mut skipped_records = vec![];

    let connection_pool = crate::core::get_connection_pool(&settings.database)
        .await
        .expect("Failed to connect to Postgres database.");

    let mut nr_valid_records: usize = 0;
    let mut nr_saved_records: usize = 0;

    tracing::info!("loading property records from source file: {:?}", file);
    for (pos, record) in reader.deserialize().enumerate() {
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

    tracing::warn!(
        "Saved {} records from {:?} ({} skipped) with {:?}",
        nr_saved_records,
        file,
        skipped_records.len(),
        quality
    );
    if !skipped_records.is_empty() {
        let first_25: Vec<usize> = skipped_records.iter().take(25).copied().collect();
        tracing::warn!("indexes of first 25 skipped csv records: {:?}", first_25);
    }

    Ok(())
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

// #[tracing::instrument(level = "info")]
// async fn do_load_property_data(file: PathBuf, settings: Settings) -> Result<(), LoaderError> {
//     let mut reader = csv::Reader::from_path(&file)?;
//     let mut deserialization_failures = vec![];
//     let mut validation_failures = vec![];
//
//     let connection_pool = crate::core::get_connection_pool(&settings.database)
//         .await
//         .expect("Failed to connect to Postgres database.");
//
//     let mut nr_valid_records: usize = 0;
//     tracing::info!("loading property records from source file: {:?}", file);
//     for (pos, record) in reader.deserialize().enumerate() {
//         let idx = pos + 1;
//         let ingress: CsvProperty = match record {
//             Ok(v) => v,
//             Err(err) => {
//                 tracing::error!(error=?err, record_index=%idx, "failed to load property record[{}]", idx);
//                 deserialization_failures.push((idx, err));
//                 continue;
//             }
//         };
//         tracing::debug!(?ingress, "deserialized record[{}]", idx);
//
//         if let Err(err) = ingress.validate() {
//             tracing::error!(error=?err, "property record[{}] failed initial validation", idx);
//             validation_failures.push((idx, err));
//             continue;
//         }
//         tracing::debug!(?ingress, "record[{}] validated", idx);
//
//         let property_record: Property = ingress.clone().try_into()?;
//         nr_valid_records += 1;
//         tracing::debug!(?property_record, %nr_valid_records, "csv record[{}] converted to save to database", idx);
//
//         match PropertyRecordRepository::find(&property_record.apn, &connection_pool).await {
//             Err(err) => {
//                 tracing::error!(error=?err, apn=?property_record.apn, "error looking while checking if property record[{}] was previously loaded.", idx);
//                 continue;
//             }
//
//             Ok(Some(record)) => {
//                 tracing::info!(apn=?record.apn, "property record[{}] previously loaded - skipping", idx);
//                 continue;
//             }
//
//             Ok(None) => {
//                 let mut transaction = connection_pool
//                     .begin()
//                     .await
//                     .expect("Failed to acquire Postgres connection from the pool.");
//                 let nr_updated = PropertyRecordRepository::save(&mut transaction, &property_record).await?;
//                 transaction
//                     .commit()
//                     .await
//                     .expect("Failed to commit SQL transaction to store loaded property records.");
//                 tracing::info!("Saved RECORD[{}]: => {:?}", idx, nr_updated,);
//             }
//         }
//     }
//
//     // transaction.commit().await.expect("Failed to commit SQL transaction to store loaded property records.");
//     tracing::info!(
//         "Saved {} records from {:?} with {} deserialization problems and {} validation issues",
//         nr_valid_records,
//         file,
//         deserialization_failures.len(),
//         validation_failures.len()
//     );
//
//     Ok(())
// }