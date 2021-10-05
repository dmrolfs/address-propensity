use super::RE_APN;
use crate::core::domain::{AssessorParcelNumber, PropensityScore, PropertyPropensityScore, ZipOrPostalCode};
use crate::loader::errors::LoaderError;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use validator::Validate;

#[derive(Debug, Validate, Clone, PartialEq, Serialize, Deserialize)]
pub struct CsvPropertyPropensityScore {
    #[validate(regex = "RE_APN")]
    pub apn: String,

    #[serde(default, alias = "SitusHouseNbr")]
    pub street_number: Option<String>,

    #[serde(default, alias = "SitusHouseNbrSuffix")]
    pub street_number_suffix: Option<String>,

    #[serde(default, alias = "SitusDirectionLeft")]
    pub street_pre_direction: Option<String>,

    #[serde(default, alias = "SitusStreet")]
    pub street_name: Option<String>,

    #[serde(default, alias = "SitusMode")]
    pub street_suffix: Option<String>,

    #[serde(default, alias = "SitusDirectionRight")]
    pub street_post_direction: Option<String>,

    #[serde(default, alias = "SitusUnitType")]
    pub secondary_designator: Option<String>,

    #[serde(default, alias = "SitusUnitNbr")]
    pub secondary_number: Option<String>,

    #[serde(default, alias = "SitusCity")]
    pub city: Option<String>,

    #[serde(default, alias = "SitusState")]
    pub state_or_region: Option<String>,

    #[serde(default, alias = "SitusZIP5")]
    pub zip_or_postal_code: Option<String>,

    #[serde(default, alias = "HomeEquityIntelScore_LineofCredit")]
    #[validate(range(min = 0))]
    pub propensity_score: Option<u16>,
}

impl TryInto<Option<PropertyPropensityScore>> for CsvPropertyPropensityScore {
    type Error = LoaderError;

    fn try_into(self) -> Result<Option<PropertyPropensityScore>, Self::Error> {
        self.propensity_score
            .map(|score| {
                Ok(PropertyPropensityScore {
                    id: None,
                    apn: self.extract_apn()?,
                    zip_or_postal_code: self.extract_zip_or_postal_code()?,
                    score: PropensityScore::new(score)?,
                })
            })
            .transpose()
    }
}

impl CsvPropertyPropensityScore {
    fn extract_apn(&self) -> Result<AssessorParcelNumber, LoaderError> {
        AssessorParcelNumber::new(&self.apn).map_err(|err| err.into())
    }

    fn extract_zip_or_postal_code(&self) -> Result<Option<ZipOrPostalCode>, LoaderError> {
        self.zip_or_postal_code
            .as_deref()
            .map(ZipOrPostalCode::new)
            .transpose()
            .map_err(|err| err.into())
    }
}
