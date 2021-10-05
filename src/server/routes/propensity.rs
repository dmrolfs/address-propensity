use crate::core::domain::{
    Address, AssessorParcelNumber, DomainError, PropensityScore, PropertyPropensityScoreRepository,
};
use crate::server::routes::error_chain_fmt;
// use crate::server::ApplicationBaseUrl;
use actix_web::http::StatusCode;
use actix_web::{web, ResponseError};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::convert::TryInto;

#[derive(Debug, PartialEq, Deserialize)]
pub struct PropensityScoresParameters {
    #[serde(default)]
    pub limit: Option<u16>,

    #[serde(alias = "zip")]
    #[serde(alias = "zipcode")]
    pub zip_code: String,
}

#[derive(thiserror::Error)]
pub enum PropensityRouteError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),

    #[error("User supplied invalid zip code: {0}")]
    InvalidZipCode(#[from] DomainError),
}

impl std::fmt::Debug for PropensityRouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PropensityRouteError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidZipCode(_) => StatusCode::BAD_REQUEST,
        }
    }
}

const LIMIT_DEFAULT: u16 = 10;

#[derive(Debug, Serialize)]
pub struct PropensitySearchItem {
    pub apn: AssessorParcelNumber,
    #[serde(flatten)]
    pub propensity_score: PropensityScore,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<Address>,
}

#[tracing::instrument(level = "info")]
pub async fn propensity_search(
    parameters: web::Query<PropensityScoresParameters>, pool: web::Data<PgPool>,
) -> Result<web::Json<Vec<PropensitySearchItem>>, PropensityRouteError> {
    let zip_code = parameters.zip_code.clone().try_into()?;
    let limit = parameters.limit.unwrap_or(LIMIT_DEFAULT);
    let top_propensity_addresses =
        PropertyPropensityScoreRepository::find_address_scores_for_zip_code(&zip_code, limit, &pool)
            .await
            .context(format!(
                "Failed to find addresses with top propensity scores in zip code, {}",
                zip_code
            ));
    if let Err(ref error) = top_propensity_addresses {
        tracing::error!(
            ?error,
            "failed to search repository for top {} propensity scores.",
            limit
        );
    }

    let report = top_propensity_addresses?
        .into_iter()
        .map(|(score, address)| PropensitySearchItem {
            apn: score.apn,
            propensity_score: score.score,
            address,
        })
        .collect();
    Ok(web::Json(report))
}
