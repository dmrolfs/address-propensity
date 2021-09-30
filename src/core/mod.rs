use std::fmt::Debug;

use settings_loader::common::database::DatabaseSettings;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use thiserror::Error;

pub mod domain;

#[tracing::instrument(level = "info")]
pub async fn get_connection_pool(settings: &DatabaseSettings) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect_with(settings.with_db())
        .await
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CoreError {
    #[error("Unrecognized land use type: {0}")]
    UnrecognizedLandUseType(String),

    #[error("Failed validation: {0}")]
    ValidationError(#[from] validator::ValidationErrors),

    #[error("{0}")]
    DatabaseExecutionError(#[from] sqlx::Error),

    #[error("{0}")]
    CoreError(#[from] anyhow::Error),
}
