use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PropensityError {
    #[error("{0}")]
    SettingsError(#[from] super::settings::SettingsError),

    #[error("{0}")]
    IOError(#[from] std::io::Error),
}
