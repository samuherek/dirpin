use crate::error::ServerError;
use axum::response::{IntoResponse, Json};
use dirpin_common::api::HealthCheckResponse;

pub mod entry;
pub mod user;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn index() -> Result<Json<HealthCheckResponse>, ServerError> {
    let version = VERSION.to_string();

    Ok(Json(HealthCheckResponse {
        status: "Ok".to_string(),
        version,
    }))
}
