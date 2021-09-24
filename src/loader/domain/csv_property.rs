use super::RE_APN;
use crate::core::domain::property::Property;
use crate::core::domain::{
    Address, AddressLine, AssessorParcelNumber, City, GeoCoordinate, LandUseType, SecondaryAddressLine, StateOrRegion,
    StreetDirection, ZipOrPostalCode,
};
use crate::loader::LoaderError;
use bigdecimal::{BigDecimal, FromPrimitive};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use validator::Validate;

#[derive(Debug, Validate, Clone, PartialEq, Serialize, Deserialize)]
pub struct CsvProperty {
    #[serde(alias = "apn_unformatted")]
    #[validate(regex = "RE_APN")]
    pub apn: String,

    #[serde(alias = "primary_number")]
    #[serde(alias = "house_number")]
    pub street_number: String,

    #[serde(default)]
    pub street_pre_direction: Option<String>,

    pub street_name: String,

    pub street_suffix: String,

    #[serde(default)]
    pub street_post_direction: Option<String>,

    #[serde(default)]
    pub secondary_designator: Option<String>,

    #[serde(default)]
    pub secondary_number: Option<String>,

    pub city: String,

    #[serde(alias = "state")]
    pub state_or_region: String,

    #[serde(alias = "zip_code")]
    pub zip_or_postal_code: String,

    #[serde(default)]
    #[validate(range(min=-90., max=90.))]
    pub latitude: Option<f64>,

    #[serde(default)]
    #[validate(range(min=-180., max=180.))]
    pub longitude: Option<f64>,

    #[serde(alias = "county_name")]
    pub admin_division: String,

    #[serde(alias = "standardized_land_use_type")]
    pub land_use_type: String,

    #[serde(default)]
    pub area_sq_ft: Option<u32>,

    #[serde(default, alias = "beds_count")]
    pub nr_bedrooms: Option<u8>,

    #[serde(default, alias = "baths")]
    pub nr_bathrooms: Option<f32>,

    #[serde(default)]
    pub total_area_sq_ft: Option<u32>,
}

impl TryInto<Property> for CsvProperty {
    type Error = LoaderError;

    fn try_into(self) -> Result<Property, Self::Error> {
        Ok(Property {
            id: None,
            apn: self.extract_apn()?,
            address: self.extract_address()?,
            admin_division: self.extract_admin_division()?,
            geo_coordinate: self.extract_geo_coordinate()?,
            land_use_type: self.extract_land_use_type()?,
            area_sq_ft: self.extract_area_sq_ft()?,
            nr_bedrooms: self.extract_nr_bedrooms()?,
            nr_bathrooms: self.extract_nr_bathrooms()?,
            total_area_sq_ft: self.extract_total_area_sq_ft()?,
        })
    }
}

impl CsvProperty {
    fn extract_apn(&self) -> Result<AssessorParcelNumber, LoaderError> {
        AssessorParcelNumber::new(&self.apn).map_err(|err| err.into())
    }

    fn extract_address(&self) -> Result<Address, LoaderError> {
        let address_line = AddressLine::new(
            &self.street_number,
            &self.street_name,
            &self.street_suffix,
            StreetDirection::new(self.street_pre_direction.as_ref(), self.street_post_direction.as_ref()),
        );

        let secondary_line = self
            .secondary_designator
            .as_ref()
            .zip(self.secondary_number.as_ref())
            .map(|(d, n)| SecondaryAddressLine::new(d, n));

        Ok(Address::new_in_usa(
            address_line,
            secondary_line,
            City::new(&self.city),
            StateOrRegion::new(&self.state_or_region),
            ZipOrPostalCode::new(&self.zip_or_postal_code)?,
        ))
    }

    fn extract_admin_division(&self) -> Result<String, LoaderError> {
        Ok(self.admin_division.clone())
    }

    fn extract_geo_coordinate(&self) -> Result<Option<GeoCoordinate>, LoaderError> {
        let latitude = self.latitude.and_then(BigDecimal::from_f64);
        let longitude = self.longitude.and_then(BigDecimal::from_f64);
        Ok(latitude
            .zip(longitude)
            .map(|(latitude, longitude)| GeoCoordinate { latitude, longitude }))
    }

    #[tracing::instrument(level = "debug")]
    fn extract_land_use_type(&self) -> Result<LandUseType, LoaderError> {
        let land_use_type = self.land_use_type.as_str().try_into()?;
        Ok(land_use_type)
    }

    fn extract_area_sq_ft(&self) -> Result<Option<u32>, LoaderError> {
        Ok(self.area_sq_ft)
    }

    fn extract_nr_bedrooms(&self) -> Result<Option<u8>, LoaderError> {
        Ok(self.nr_bedrooms)
    }

    fn extract_nr_bathrooms(&self) -> Result<Option<BigDecimal>, LoaderError> {
        Ok(self.nr_bathrooms.and_then(BigDecimal::from_f32))
    }

    fn extract_total_area_sq_ft(&self) -> Result<Option<u32>, LoaderError> {
        Ok(self.total_area_sq_ft)
    }
}
