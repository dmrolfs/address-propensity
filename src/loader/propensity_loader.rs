use crate::core::domain::property::PropertyRecordRepository;
use crate::core::domain::{
    PropensityScore, PropertyPropensityScore, PropertyPropensityScoreRepository, ZipOrPostalCode,
};
use crate::loader::domain::CsvPropertyPropensityScore;
use crate::loader::errors::LoaderError;
use crate::loader::settings::Settings;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use plotters::prelude::*;
use sqlx::PgPool;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use validator::{Validate, ValidationErrors};

#[derive(Default)]
struct QualityMeasure {
    pub propensity_zips: Vec<(PropensityScore, Option<ZipOrPostalCode>)>,
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
                result = f.write_str(
                    format!(
                        "\n\t{} not in core properties (but still loaded)",
                        self.not_in_core_properties.len()
                    )
                    .as_str(),
                );
            }

            result
        } else {
            result
        }
    }
}

//todo: there's likely similarity between property loading and propensity loading. Future work to unify.
#[tracing::instrument(level = "info", skip(settings))]
pub async fn load_propensity_data(file: PathBuf, settings: Settings) -> Result<(), LoaderError> {
    let mut reader = csv::Reader::from_path(&file)?;
    let mut quality = QualityMeasure::default();
    let mut skipped_records = vec![];
    let nr_records = count_nr_records(&file)?;

    let connection_pool = crate::core::get_connection_pool(&settings.database)
        .await
        .expect("Failed to connect to Postgres database.");

    let mut nr_valid_records: usize = 0;
    let mut nr_saved_records: usize = 0;

    tracing::info!("loading propensity records from source file: {:?}", file);
    eprintln!(" {}...", style("Loading propensity scores").bold());
    let sty = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .progress_chars("##-");

    let progress = ProgressBar::new(nr_records as u64);
    progress.set_style(sty);

    for (pos, record) in reader.deserialize().enumerate() {
        let idx = pos + 1;
        progress.set_message(format!("record #{}", idx));
        progress.inc(1);

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
    progress.finish_with_message("done");

    summarize(nr_saved_records, &file, &skipped_records, &quality);
    Ok(())
}

#[tracing::instrument(level = "info")]
fn count_nr_records(path: &PathBuf) -> Result<usize, LoaderError> {
    let reader = BufReader::new(File::open(path)?);
    Ok(reader.lines().count() - 1) // subtract header line
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
            quality
                .propensity_zips
                .push((record.score, record.zip_or_postal_code.clone()));
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
                    quality
                        .propensity_zips
                        .push((record.score, record.zip_or_postal_code.clone()));
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

    visualize_score_distribution(&quality.propensity_zips).expect("Failed to save propensity_score_distribution.png");
    visualize_zipcode_scores(&quality.propensity_zips).expect("Failed to save score_zipcode_distribution.png");
}

#[tracing::instrument(level = "info", skip(score_zips))]
fn visualize_score_distribution(score_zips: &Vec<(PropensityScore, Option<ZipOrPostalCode>)>) -> anyhow::Result<()> {
    let out_filename = "propensity_score_distribution.png";
    let scores: Vec<u32> = score_zips.iter().map(|(s, _)| s.score as u32).collect();

    let background = BitMapBackend::new(out_filename, (640, 480)).into_drawing_area();
    background.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&background)
        .x_label_area_size(35)
        .y_label_area_size(40)
        .margin(5)
        .caption("Propensity Score Distribution", ("sans-serif", 50.0))
        .build_cartesian_2d((0_u32..950).into_segmented(), 0_u32..100_u32)?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .bold_line_style(&WHITE.mix(0.3))
        .y_desc("Count")
        .x_desc("Score")
        .axis_desc_style(("sans-serif", 15))
        .draw()?;

    chart.draw_series(
        Histogram::vertical(&chart)
            .style(RED.mix(0.5).filled())
            .data(scores.iter().map(|x| (*x, 1))),
    )?;

    background.present().expect("Unable to write result to file");
    eprintln!(
        " {}.",
        style(format!("Propensity score visualization was saved to {}", out_filename)).bold()
    );
    Ok(())
}

#[tracing::instrument(level = "info", skip(score_zips))]
fn visualize_zipcode_scores(score_zips: &Vec<(PropensityScore, Option<ZipOrPostalCode>)>) -> anyhow::Result<()> {
    let out_filename = "score_zipcode_distribution.png";
    let mut zip_counts: HashMap<String, u32> = HashMap::default();
    for (_score, zip) in score_zips.iter() {
        if let Some(z) = zip {
            let count = zip_counts.get(z.as_ref()).copied().unwrap_or(0);
            zip_counts.insert(z.to_string(), count + 1);
        }
    }
    let mut x_counts = vec![];
    let mut y_counts: HashMap<u32, u32> = HashMap::default();
    for (_, zip_count) in zip_counts {
        x_counts.push(zip_count);
        let y_count = y_counts.get(&zip_count).copied().unwrap_or(0);
        y_counts.insert(zip_count, y_count + 1);
    }
    let x_max = x_counts.iter().max().copied().unwrap_or(0);
    let x_min = x_counts.iter().min().copied().unwrap_or(0);
    let y_max = y_counts.iter().map(|(_, count)| count).max().copied().unwrap_or(0);
    let y_min = y_counts.iter().map(|(_, count)| count).min().copied().unwrap_or(0);
    tracing::warn!(%x_max, %x_min, %y_min, %y_max, nr_counts=%x_counts.len(), "zipcode viz tallies");

    let background = BitMapBackend::new(out_filename, (640, 480)).into_drawing_area();
    background.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&background)
        .x_label_area_size(35)
        .y_label_area_size(40)
        .margin(5)
        .caption("Propensity Zipcode Distribution", ("sans-serif", 50.0))
        .build_cartesian_2d((0..x_max).into_segmented(), 0..y_max)?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .bold_line_style(&WHITE.mix(0.3))
        .y_desc("# of Zipcodes")
        .x_desc("# Scores in Zipcode")
        .axis_desc_style(("sans-serif", 15))
        .draw()?;

    chart.draw_series(
        Histogram::vertical(&chart)
            .style(RED.mix(0.5).filled())
            .data(x_counts.into_iter().map(|x| (x, 1))),
    )?;

    background.present().expect("Unable to write result ro file");
    eprintln!(
        " {}.",
        style(format!(
            "Zipcode propensity population visualization was save to {}",
            out_filename
        ))
            .bold()
    );
    Ok(())
}
