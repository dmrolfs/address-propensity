use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationSettings {
    #[serde_as(as = "DisplayFromStr")]
    pub port: u16,
    pub host: String,
    // pub base_url: String,
}
