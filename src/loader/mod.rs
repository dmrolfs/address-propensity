pub mod domain;
pub mod propensity_loader;
pub mod property_loader;
pub mod settings;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LoaderError {
    #[error("{0}")]
    SettingsError(#[from] settings_loader::SettingsError),

    #[error("{0}")]
    CoreError(#[from] crate::core::CoreError),

    #[error("{0}")]
    ValidationErrors(#[from] validator::ValidationErrors),

    #[error("{0}")]
    IOError(#[from] std::io::Error),

    #[error("{0}")]
    CsvError(#[from] csv::Error),

    #[error("{0}")]
    RepositoryError(#[from] sqlx::Error),

    #[error("Unrecognized land use type: {0}")]
    UnrecognizedLandUseType(String),

    #[error("{0}")]
    LoaderError(#[from] anyhow::Error),
}
