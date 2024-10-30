use axum::response::Json;
use dirpin_common::api::{HealthCheckResponse, SyncRequest, SyncResponse};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn index() -> Json<HealthCheckResponse> {
    let version = VERSION.to_string();

    Json(HealthCheckResponse {
        status: "Ok".to_string(),
        version,
    })
}

pub async fn sync(Json(sync): Json<SyncRequest>) -> Json<SyncResponse> {
    println!("sync:: {:?}", sync);
    Json(SyncResponse {
        ok: "Yes".to_string(),
    })
}
