use crate::core::domain::DomainError;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::convert::TryFrom;
use std::fmt::{self, Display};
use validator::{Validate, ValidationErrors};

//todo: Since there broader address normalization is out of scope, formmating all address strings
// to upper case in order to avoid potential inconsistency from source data.

//todo: consider validate at struct level considering locale, via method okay, but via service too much.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Address {
    pub address_line: AddressLine,
    pub secondary_address_line: Option<SecondaryAddressLine>,
    pub city: City,
    pub state_or_region: StateOrRegion,
    pub zip_or_postal_code: ZipOrPostalCode,
    pub locale: CountryCode,
}

impl Address {
    pub fn new_in_usa(
        address_line: AddressLine, secondary_address_line: Option<SecondaryAddressLine>, city: City,
        state: StateOrRegion, zip_code: ZipOrPostalCode,
    ) -> Self {
        Self {
            address_line,
            secondary_address_line,
            city,
            state_or_region: state,
            zip_or_postal_code: zip_code,
            locale: USA.clone(),
        }
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //todo: via i18n crate, format address display string via Locale, but for now hard-code USA
        let address_lines = self
            .secondary_address_line
            .as_ref()
            .map(|second| format!("{} {}", self.address_line, second))
            .unwrap_or_else(|| format!("{}", self.address_line));

        write!(
            f,
            "{}, {}, {} {}, {}",
            address_lines, self.city, self.state_or_region, self.zip_or_postal_code, self.locale.iso_3166_alpha_3
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddressLine {
    pub street_number: String,
    pub street_name: String,
    pub street_suffix: String,
    pub street_direction: StreetDirection,
}

impl AddressLine {
    pub fn new(
        number: impl Into<String>, name: impl Into<String>, suffix: impl Into<String>, direction: StreetDirection,
    ) -> Self {
        Self {
            street_number: number.into(),
            street_name: name.into().to_uppercase(),
            street_suffix: suffix.into().to_uppercase(),
            street_direction: direction,
        }
    }
}

impl Display for AddressLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let street_name = format!("{} {}", self.street_name, self.street_suffix);
        let street_name = self.street_direction.decorate_street_name(street_name.as_str());
        write!(f, "{} {}", self.street_number, street_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecondaryAddressLine {
    pub designator: String,
    pub number: String,
}

impl SecondaryAddressLine {
    pub fn new(designator: impl Into<String>, number: impl Into<String>) -> Self {
        Self {
            designator: designator.into().to_uppercase(),
            number: number.into().to_uppercase(),
        }
    }
}

impl Display for SecondaryAddressLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.designator, self.number)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StreetDirection {
    None,
    Prefix(String),
    Suffix(String),
    Both(String, String),
}

impl StreetDirection {
    pub fn new(prefix_direction: Option<impl Into<String>>, suffix_direction: Option<impl Into<String>>) -> Self {
        match (prefix_direction.map(|s| s), suffix_direction) {
            (None, None) => Self::None,
            (Some(prefix), None) => Self::for_prefix(prefix),
            (None, Some(suffix)) => Self::for_suffix(suffix),
            (Some(prefix), Some(suffix)) => Self::Both(prefix.into().to_uppercase(), suffix.into().to_uppercase()),
        }
    }

    pub fn for_prefix(rep: impl Into<String>) -> Self {
        Self::Prefix(rep.into().to_uppercase())
    }

    pub fn for_suffix(rep: impl Into<String>) -> Self {
        Self::Suffix(rep.into().to_uppercase())
    }

    pub fn prefix(&self) -> Option<String> {
        match self {
            Self::None | Self::Suffix(_) => None,
            Self::Prefix(prefix) => Some(prefix.clone()),
            Self::Both(prefix, _) => Some(prefix.clone()),
        }
    }

    pub fn suffix(&self) -> Option<String> {
        match self {
            Self::None | Self::Prefix(_) => None,
            Self::Suffix(suffix) => Some(suffix.clone()),
            Self::Both(_, suffix) => Some(suffix.clone()),
        }
    }

    pub fn decorate_street_name<'s>(&self, street_name: &'s str) -> Cow<'s, str> {
        match self {
            Self::None => street_name.into(),
            Self::Prefix(pre) => format!("{} {}", pre, street_name).into(),
            Self::Suffix(post) => format!("{} {}", street_name, post).into(),
            Self::Both(pre, post) => format!("{} {} {}", pre, street_name, post).into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct City(String);

impl City {
    pub fn new(city: impl Into<String>) -> Self {
        Self(city.into().to_uppercase())
    }
}

impl Display for City {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for City {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
// impl Into<String> for City {
//     fn into(self) -> String {
//         self.0
//     }
// }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateOrRegion(String);

impl StateOrRegion {
    pub fn new(state_or_region: impl Into<String>) -> Self {
        Self(state_or_region.into().to_uppercase())
    }
}

impl Display for StateOrRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for StateOrRegion {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

lazy_static::lazy_static! {
    static ref RE_ZIP_CODE: Regex = Regex::new(r##"^\d{5}$"##).unwrap();
}

#[derive(Debug, Validate, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZipOrPostalCode {
    #[validate(regex(path = "RE_ZIP_CODE", message = "only 5 digit US zip codes are supported"))]
    code: String,
}

impl ZipOrPostalCode {
    pub fn new(zip_or_postal_code: impl Into<String>) -> Result<Self, ValidationErrors> {
        let code = Self { code: zip_or_postal_code.into() };
        code.validate()?;
        Ok(code)
    }
}

impl Display for ZipOrPostalCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code)
    }
}

impl AsRef<str> for ZipOrPostalCode {
    fn as_ref(&self) -> &str {
        self.code.as_str()
    }
}

impl TryFrom<String> for ZipOrPostalCode {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        //todo: add regex validation
        let code = Self::new(value)?;
        Ok(code)
    }
}

//todo: good use for a declarative macro to initialize const.
lazy_static::lazy_static! {
    pub static ref USA: CountryCode = CountryCode::new("USA", "United States of America").unwrap();
}

#[derive(Debug, Validate, Clone, PartialEq, Serialize, Deserialize)]
pub struct CountryCode {
    #[serde(alias = "code")]
    #[validate(length(equal = 3))]
    iso_3166_alpha_3: String,
    #[serde(alias = "name")]
    official_name: String,
}

impl CountryCode {
    pub fn new(iso_3166_alpha_3_code: impl Into<String>, name: impl Into<String>) -> Result<Self, ValidationErrors> {
        let iso_3166_alpha_3 = iso_3166_alpha_3_code.into().to_uppercase();
        let official_name = name.into().to_uppercase();
        let code = Self { iso_3166_alpha_3, official_name };
        code.validate()?;
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claim::assert_ok;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_address_display() -> anyhow::Result<()> {
        let address = Address::new_in_usa(
            AddressLine::new(3112.to_string(), "Bonnie Brook", "Ln", StreetDirection::for_prefix("n")),
            Some(SecondaryAddressLine::new("Unit", "7A")),
            City::new("Plano"),
            StateOrRegion::new("Tx"),
            assert_ok!(ZipOrPostalCode::new(75075.to_string())),
        );

        assert_eq!(
            format!("{}", address),
            "3112 N BONNIE BROOK LN UNIT 7A, PLANO, TX 75075, USA".to_string()
        );
        Ok(())
    }

    #[test]
    fn test_address_line_display() -> anyhow::Result<()> {
        let address = AddressLine {
            street_number: 3112.to_string(),
            street_direction: StreetDirection::None,
            street_name: "Bonnie Brook".to_string(),
            street_suffix: "Ln".to_string(),
        };

        assert_eq!(format!("{}", address), "3112 Bonnie Brook Ln".to_string());

        let address = AddressLine {
            street_direction: StreetDirection::for_prefix("NE"),
            ..address
        };
        assert_eq!(format!("{}", address), "3112 NE Bonnie Brook Ln".to_string());

        let address = AddressLine {
            street_direction: StreetDirection::for_suffix("SW"),
            ..address
        };
        assert_eq!(format!("{}", address), "3112 Bonnie Brook Ln SW".to_string());
        Ok(())
    }
}
