use crate::core::domain::property::Property;
use crate::core::domain::{
    Address, AddressLine, AssessorParcelNumber, City, SecondaryAddressLine, StateOrRegion, StreetDirection,
    ZipOrPostalCode,
};
use crate::core::CoreError;
use anyhow::Context;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use validator::Validate;

#[derive(Debug, Validate, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropensityScore {
    #[validate(range(min = 0))]
    pub score: u16,
}

impl PropensityScore {
    pub fn new(propensity_score: u16) -> Result<Self, CoreError> {
        let score = Self { score: propensity_score };
        score.validate()?;
        Ok(score)
    }
}

#[derive(Debug, Validate, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyPropensityScore {
    #[serde(default, alias = "property_propensity_id")]
    pub id: Option<i32>,

    pub apn: AssessorParcelNumber,

    #[validate]
    pub zip_or_postal_code: Option<ZipOrPostalCode>,

    #[validate]
    pub score: PropensityScore,
}

impl PropertyPropensityScore {
    pub fn new(
        score: PropensityScore, apn: &AssessorParcelNumber, zip_or_postal_code: &Option<ZipOrPostalCode>,
    ) -> Result<Self, CoreError> {
        let property_score = Self {
            id: None,
            apn: apn.clone(),
            zip_or_postal_code: zip_or_postal_code.clone(),
            score,
        };
        property_score.validate()?;
        Ok(property_score)
    }

    pub fn for_property(property: &Property, score: PropensityScore) -> Result<Self, CoreError> {
        let property_score = Self {
            id: None,
            apn: property.apn.clone(),
            zip_or_postal_code: Some(property.address.zip_or_postal_code.clone()),
            score,
        };
        property_score.validate()?;
        Ok(property_score)
    }
}

pub struct PropertyPropensityScoreRepository;

type ScoreAddress = (PropertyPropensityScore, Option<Address>);

impl PropertyPropensityScoreRepository {
    #[tracing::instrument(level = "info", skip(pool))]
    pub async fn find_for_apn(
        apn: &AssessorParcelNumber, pool: &PgPool,
    ) -> Result<Option<PropertyPropensityScore>, CoreError> {
        sqlx::query!(
            r##"
            SELECT id, apn, score, zip_or_postal_code
            FROM propensities
            WHERE apn = $1
            LIMIT 1
            "##,
            apn.as_ref()
        )
        .fetch_optional(pool)
        .await
        .context("Failed to perform a query to retrieve stored propensity score for apn.")?
        .map(|row| {
            let zip_or_postal_code = row.zip_or_postal_code.map(|z| ZipOrPostalCode::new(z)).transpose()?;
            Ok(PropertyPropensityScore {
                id: Some(row.id),
                apn: AssessorParcelNumber::new(row.apn)?,
                zip_or_postal_code,
                score: PropensityScore::new(row.score as u16)?,
            })
        })
        .transpose()
    }

    #[tracing::instrument(level = "info", skip(pool))]
    pub async fn find_address_scores_for_zip_code(
        zip_code: &ZipOrPostalCode, limit: u16, pool: &PgPool,
    ) -> Result<Vec<ScoreAddress>, CoreError> {
        let records = sqlx::query!(
            r##"
            SELECT Propensities.id, Propensities.apn, Propensities.score, Propensities.zip_or_postal_code as p_zip_or_postal_code,
                Properties.street_number, Properties.street_pre_direction, Properties.street_name,
                Properties.street_suffix, Properties.street_post_direction, Properties.secondary_designator,
                Properties.secondary_number, Properties.city, Properties.state_or_region, Properties.zip_or_postal_code as a_zip_or_postal_code
            FROM Propensities
            INNER JOIN Properties
            ON Propensities.apn = Properties.apn
            WHERE Propensities.zip_or_postal_code = $1
            ORDER BY Propensities.score DESC
            LIMIT $2
            "##,
            zip_code.as_ref(),
            limit as i64,
        )
        .fetch_all(pool)
        .await
        .context("Failed to perform a query to retrieve top propensity scores for a zip code.")?;

        let result: Vec<Result<ScoreAddress, CoreError>> = records
            .into_iter()
            .map(|record| {
                let p_zip_or_postal_code = record.p_zip_or_postal_code.map(|z| ZipOrPostalCode::new(z)).transpose()?;
                let score = PropertyPropensityScore {
                    id: Some(record.id),
                    apn: AssessorParcelNumber::new(record.apn)?,
                    zip_or_postal_code: p_zip_or_postal_code,
                    score: PropensityScore::new(record.score as u16)?,
                };
                let secondary: Option<SecondaryAddressLine> = record
                    .secondary_designator
                    .zip(record.secondary_number)
                    .map(|(d, n)| SecondaryAddressLine::new(d, n));

                let address = Address::new_in_usa(
                    AddressLine::new(
                        record.street_number,
                        record.street_name,
                        record.street_suffix,
                        StreetDirection::new(record.street_pre_direction, record.street_post_direction),
                    ),
                    secondary,
                    City::new(record.city),
                    StateOrRegion::new(record.state_or_region),
                    ZipOrPostalCode::new(record.a_zip_or_postal_code)?,
                );
                Ok((score, Some(address)))
            })
            .collect();

        let result: Result<Vec<ScoreAddress>, CoreError> = result.into_iter().collect();
        result
    }

    #[tracing::instrument(level = "info", skip(transaction))]
    pub async fn save(
        transaction: &mut Transaction<'_, Postgres>, record: &PropertyPropensityScore,
    ) -> Result<PropertyPropensityScore, CoreError> {
        let now = Utc::now();

        let result = sqlx::query!(
            r##"
            INSERT INTO Propensities (id, apn, zip_or_postal_code, score, created_on, last_updated_on)
            VALUES(DEFAULT, $1, $2, $3, $4, $5)
            RETURNING id
            "##,
            record.apn.as_ref(),
            record.zip_or_postal_code.as_ref().map(|z| z.as_ref()),
            record.score.score as i16,
            now.into(),
            now.into()
        )
        .fetch_one(transaction)
        .await?;

        Ok(PropertyPropensityScore { id: Some(result.id as i32), ..record.clone() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claim::{assert_err, assert_ok};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_propensity_score_validation() -> anyhow::Result<()> {
        let _errors = assert_err!(PropensityScore::new(0));

        let actual = assert_ok!(PropensityScore::new(50));
        assert_eq!(actual, PropensityScore { score: 50 });

        let actual = assert_ok!(PropensityScore::new(950));
        assert_eq!(actual, PropensityScore { score: 950 });

        let _errors = assert_err!(PropensityScore::new(951));
        Ok(())
    }
}
