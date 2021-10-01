use std::fmt::Debug;
use thiserror::Error;
use metered::{metered, ErrorCount};

#[metered::error_count(name = PropensityErrorCount, visibility = pub)]
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PropensityError {
    #[error("{0}")]
    SettingsError(#[from] settings_loader::SettingsError),

    #[error("{0}")]
    IOError(#[from] std::io::Error),
}
