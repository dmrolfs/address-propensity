use crate::core::domain::{
    Address, AddressLine, AssessorParcelNumber, City, GeoCoordinate, LandUseType, SecondaryAddressLine, StateOrRegion,
    StreetDirection, ZipOrPostalCode,
};
use crate::core::CoreError;
use anyhow::Context;
use bigdecimal::BigDecimal;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use std::convert::TryInto;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    #[serde(default, alias = "property_id")]
    pub id: Option<i32>,

    pub apn: AssessorParcelNumber,

    pub address: Address,

    #[serde(alias = "county_name")]
    pub admin_division: String,

    #[serde(default)]
    pub geo_coordinate: Option<GeoCoordinate>,

    #[serde(alias = "standardized_land_use_type")]
    pub land_use_type: LandUseType,

    #[serde(default)]
    pub area_sq_ft: Option<u32>,

    #[serde(default, alias = "beds_count")]
    pub nr_bedrooms: Option<u8>,

    #[serde(default, alias = "baths")]
    pub nr_bathrooms: Option<BigDecimal>,

    #[serde(default)]
    pub total_area_sq_ft: Option<u32>,
}

pub struct PropertyRecordRepository;

impl PropertyRecordRepository {
    // pub fn new<'a>(connection: &'a PgConnection) -> Self where 'a: 'c { Self(connection) }

    #[tracing::instrument(level = "info", skip(pool))]
    pub async fn find(apn: &AssessorParcelNumber, pool: &PgPool) -> Result<Option<Property>, CoreError> {
        sqlx::query!(
            r##"
            SELECT
                id,
                apn,
                street_number,
                street_pre_direction,
                street_name,
                street_suffix,
                street_post_direction,
                secondary_designator,
                secondary_number,
                city,
                state_or_region,
                zip_or_postal_code,
                latitude,
                longitude,
                admin_division,
                land_use_type,
                area_sq_ft,
                nr_bedrooms,
                nr_bathrooms,
                total_area_sq_ft,
                created_on,
                last_updated_on
            FROM properties
            WHERE apn = $1
            LIMIT 1
            "##,
            apn.as_ref()
        )
        .fetch_optional(pool)
        .await
        .context("Failed to perform a query to retrieve stored property record.")?
        .map(|row| {
            let secondary: Option<SecondaryAddressLine> = row
                .secondary_designator
                .zip(row.secondary_number)
                .map(|(d, n)| SecondaryAddressLine::new(d, n));

            let geo_coordinate: Option<GeoCoordinate> =
                row.latitude.zip(row.longitude).map(|(d, n)| GeoCoordinate::new(d, n));

            Ok(Property {
                id: Some(row.id),
                apn: AssessorParcelNumber::new(row.apn)?,
                address: Address::new_in_usa(
                    AddressLine::new(
                        row.street_number,
                        row.street_name,
                        row.street_suffix,
                        StreetDirection::new(row.street_pre_direction, row.street_post_direction),
                    ),
                    secondary,
                    City::new(row.city),
                    StateOrRegion::new(row.state_or_region),
                    ZipOrPostalCode::new(row.zip_or_postal_code)?,
                ),
                admin_division: row.admin_division,
                geo_coordinate,
                land_use_type: row.land_use_type.as_str().try_into()?,
                area_sq_ft: row.area_sq_ft.map(|v| v as u32),
                nr_bedrooms: row.nr_bedrooms.map(|v| v as u8),
                nr_bathrooms: row.nr_bathrooms,
                total_area_sq_ft: row.total_area_sq_ft.map(|v| v as u32),
            })
        })
        .transpose()
    }

    #[tracing::instrument(level = "info", skip(transaction))]
    pub async fn save(transaction: &mut Transaction<'_, Postgres>, record: &Property) -> Result<Property, CoreError> {
        let now = Utc::now();

        let land_use: String = record.land_use_type.clone().into();
        let address = &record.address;
        let secondary_designator = address.secondary_address_line.as_ref().map(|s| s.designator.clone());
        let secondary_number = address.secondary_address_line.as_ref().map(|s| s.number.clone());
        let dir_prefix = address.address_line.street_direction.prefix();
        let dir_suffix = address.address_line.street_direction.suffix();
        let geo_lat = record.geo_coordinate.as_ref().map(|g| g.latitude.clone());
        let geo_long = record.geo_coordinate.as_ref().map(|g| g.longitude.clone());

        let result = sqlx::query!(
            r##"
            INSERT INTO Properties (
                id,
                apn,
                street_number,
                street_pre_direction,
                street_name,
                street_suffix,
                street_post_direction,
                secondary_designator,
                secondary_number,
                city,
                state_or_region,
                zip_or_postal_code,
                latitude,
                longitude,
                admin_division,
                land_use_type,
                area_sq_ft,
                nr_bedrooms,
                nr_bathrooms,
                total_area_sq_ft,
                created_on,
                last_updated_on
            )
            VALUES(DEFAULT, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21 )
            RETURNING id
            "##,
            record.apn.as_ref(),
            &record.address.address_line.street_number,
            dir_prefix.as_deref(),
            &record.address.address_line.street_name,
            &record.address.address_line.street_suffix,
            dir_suffix.as_deref(),
            secondary_designator.as_deref(),
            secondary_number.as_deref(),
            record.address.city.as_ref(),
            record.address.state_or_region.as_ref(),
            record.address.zip_or_postal_code.as_ref(),
            geo_lat,
            geo_long,
            &record.admin_division,
            &land_use,
            record.area_sq_ft.map(|v| v as i32),
            record.nr_bedrooms.map(|v| v as i16),
            record.nr_bathrooms.as_ref(),
            record.total_area_sq_ft.map(|v| v as i32),
            now.into(),
            now.into(),
        )
            .fetch_one(transaction)
            .await?;

        Ok(Property { id: Some(result.id as i32), ..record.clone() })
    }
}
