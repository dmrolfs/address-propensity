pub mod csv_propensity;
pub mod csv_property;

pub use csv_propensity::*;
pub use csv_property::*;

use regex::Regex;

lazy_static::lazy_static! {
    static ref RE_APN: Regex = Regex::new(r##"[\d\w-]{7,}"##).unwrap();
}
