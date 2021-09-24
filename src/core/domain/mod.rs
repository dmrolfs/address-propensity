use crate::core::CoreError;
pub use address::*;
use bigdecimal::BigDecimal;
pub use propensity::*;
use regex::{Regex, RegexSet, RegexSetBuilder};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::convert::TryFrom;
use std::fmt::{self, Display};
use thiserror::Error;
use validator::{ValidationError, ValidationErrors};

pub mod address;
pub mod propensity;
pub mod property;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssessorParcelNumber(String);

impl AssessorParcelNumber {
    pub fn new(apn: impl Into<String>) -> Result<Self, ValidationErrors> {
        let apn = apn.into();
        let apn = Self::check_apn(apn)?;
        Ok(Self(apn))
    }

    fn check_apn(apn: String) -> Result<String, ValidationErrors> {
        lazy_static::lazy_static! {
            static ref RE_NUMERIC_APN: Regex = Regex::new(r##"\d[\d-]*\d"##).unwrap();
        }
        const APN_LENGTH: usize = 14;

        if !RE_NUMERIC_APN.is_match(&apn) {
            let mut errors = ValidationErrors::new();
            let mut error = ValidationError::new("format");
            error.message = Some(Cow::from("Only numeric-based APNs currently supported"));
            error.add_param(Cow::from("numeric"), &apn);
            errors.add("apn", error);
            return Err(errors);
        }

        let mut reduced_apn = apn.replace(&['-'][..], "");

        if APN_LENGTH < reduced_apn.len() {
            let mut errors = ValidationErrors::new();
            let mut error = ValidationError::new("length");
            error.message = Some(Cow::from(format!(
                "Up to {}-digit APNs (not including `-`'s) are currently supported",
                APN_LENGTH
            )));
            error.add_param(Cow::from("max"), &APN_LENGTH);
            errors.add("apn", error);
            return Err(errors);
        }

        if reduced_apn.len() < APN_LENGTH {
            //todo replace magic number in format string with APN_LENGTH
            reduced_apn = format!("{:0>14}", reduced_apn);
        }

        Ok(reduced_apn)
    }

    pub fn apn(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for AssessorParcelNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for AssessorParcelNumber {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoCoordinate {
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
}

impl GeoCoordinate {
    pub fn new(latitude: BigDecimal, longitude: BigDecimal) -> Self {
        Self { latitude, longitude }
    }
}

impl fmt::Display for GeoCoordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.latitude, self.longitude)
    }
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LandUseType {
    CondominiumUnit,
    Duplex,
    MobileOrManufacturedHome,
    MultiFamilyDwellings,
    PlannedUnitDevelopment,
    Quadruplex,
    RuralOrAgriculturalResidence,
    SingleFamilyResidential,
    Townhouse,
    Triplex,
    VacationResidence,
}

impl Into<String> for LandUseType {
    fn into(self) -> String {
        format!("{}", self)
    }
}

impl<'s> TryFrom<&'s str> for LandUseType {
    type Error = CoreError;

    #[tracing::instrument(level = "debug")]
    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        lazy_static::lazy_static! {
            static ref RE_CONDOMINIUM: RegexSet = RegexSetBuilder::new(&[
                r##"condominium\s*unit"##,
            ]).case_insensitive(true).build().unwrap();

            static ref RE_DUPLEX: RegexSet = RegexSetBuilder::new(&[
                r##"duplex"##,
            ]).case_insensitive(true).build().unwrap();

            static ref RE_MOBILE_OR_MANUFACTURED: RegexSet = RegexSetBuilder::new(&[
                r##"mobile\s*home"##,
                r##"manufactured\s*home"##,
                r##"mobile\s*or\s*manufactured\s*home"##,
                r##"manufactured\s*or\s*mobile\s*home"##,
            ]).case_insensitive(true).build().unwrap();

            static ref RE_MULTI_FAMILY: RegexSet = RegexSetBuilder::new(&[
                r##"multi\s*-?\s*family\s*dwellings?"##,
                r##"multi\s*-?\s*family\s*residential"##,
                r##"multi\s*residential"##,
            ]).case_insensitive(true).build().unwrap();

            static ref RE_PLANNED: RegexSet = RegexSetBuilder::new(&[
                r##"planned\s*unit\s*development"##,
                r##"planned\s*development"##,
            ]).case_insensitive(true).build().unwrap();

            static ref RE_QUADRUPLEX: RegexSet = RegexSetBuilder::new(&[
                r##"quadruplex"##,
            ]).case_insensitive(true).build().unwrap();

            static ref RE_RURAL_OR_AGRICULTURAL: RegexSet = RegexSetBuilder::new(&[
                r##"rural\s*or\s*agricultural\s*residence"##,
                r##"rural\s*residence"##,
                r##"agricultural\s*residence"##,
            ]).case_insensitive(true).build().unwrap();

            static ref RE_SINGLE_FAMILY: RegexSet = RegexSetBuilder::new(&[
                r##"single\s*family\s*residential"##,
                r##"single\s*residential"##,
            ]).case_insensitive(true).build().unwrap();

            static ref RE_TOWNHOUSE: RegexSet = RegexSetBuilder::new(&[
                r##"townhouse"##
            ]).case_insensitive(true).build().unwrap();

            static ref RE_TRIPLEX: RegexSet = RegexSetBuilder::new(&[
                r##"triplex"##,
            ]).case_insensitive(true).build().unwrap();

            static ref RE_VACATION: RegexSet = RegexSetBuilder::new(&[
                r##"vacation\s*residence"##,
            ]).case_insensitive(true).build().unwrap();

            static ref TYPE_PATTERNS: Vec<(String, &'static RegexSet)> = vec![
                (format!("{}", LandUseType::CondominiumUnit), &RE_CONDOMINIUM),
                (format!("{}", LandUseType::Duplex), &RE_DUPLEX),
                (format!("{}", LandUseType::MobileOrManufacturedHome), &RE_MOBILE_OR_MANUFACTURED),
                (format!("{}", LandUseType::MultiFamilyDwellings), &RE_MULTI_FAMILY),
                (format!("{}", LandUseType::PlannedUnitDevelopment), &RE_PLANNED),
                (format!("{}", LandUseType::Quadruplex), &RE_QUADRUPLEX),
                (format!("{}", LandUseType::RuralOrAgriculturalResidence), &RE_RURAL_OR_AGRICULTURAL),
                (format!("{}", LandUseType::SingleFamilyResidential), &RE_SINGLE_FAMILY),
                (format!("{}", LandUseType::Townhouse), &RE_TOWNHOUSE),
                (format!("{}", LandUseType::Triplex), &RE_TRIPLEX),
                (format!("{}", LandUseType::VacationResidence), &RE_VACATION),
            ];
        }

        let use_type = TYPE_PATTERNS.iter().find(|(_, regex)| regex.is_match(value));
        match use_type.map(|ut| ut.0.as_str()) {
            Some("CondominiumUnit") => Ok(Self::CondominiumUnit),
            Some("Duplex") => Ok(Self::Duplex),
            Some("MobileOrManufacturedHome") => Ok(Self::MobileOrManufacturedHome),
            Some("MultiFamilyDwellings") => Ok(Self::MultiFamilyDwellings),
            Some("PlannedUnitDevelopment") => Ok(Self::PlannedUnitDevelopment),
            Some("Quadruplex") => Ok(Self::Quadruplex),
            Some("RuralOrAgriculturalResidence") => Ok(Self::RuralOrAgriculturalResidence),
            Some("SingleFamilyResidential") => Ok(Self::SingleFamilyResidential),
            Some("Townhouse") => Ok(Self::Townhouse),
            Some("Triplex") => Ok(Self::Triplex),
            Some("VacationResidence") => Ok(Self::VacationResidence),
            _ => Err(CoreError::UnrecognizedLandUseType(value.to_string())),
        }
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DomainError {
    #[error("Invalid zip or postal code: {0}")]
    InvalidZipOrPostalCode(#[from] ValidationErrors),
}
