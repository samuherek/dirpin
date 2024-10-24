use axum::response::Json;
use dirpin_common::api::HealthCheckResponse;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn index() -> Json<HealthCheckResponse> {
    let version = VERSION.to_string();

    Json(HealthCheckResponse {
        status: "Ok".to_string(),
        version,
    })
}
