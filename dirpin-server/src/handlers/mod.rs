use crate::models::NewPin;
use crate::router::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use dirpin_common::api::{AddPinRequest, HealthCheckResponse, SyncRequest, SyncResponse};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn index() -> Json<HealthCheckResponse> {
    let version = VERSION.to_string();

    Json(HealthCheckResponse {
        status: "Ok".to_string(),
        version,
    })
}

pub async fn sync(params: Query<SyncRequest>) -> Json<SyncResponse> {
    println!("sync:: {:?}", params);
    Json(SyncResponse {
        updated: vec![],
        deleted: vec![],
    })
}

pub async fn add(state: State<AppState>, Json(req): Json<Vec<AddPinRequest>>) -> impl IntoResponse {
    println!("adding:....");
    let pins = req
        .into_iter()
        .map(|x| NewPin {
            client_id: x.id,
            user_id: 1,
            timestamp: x.timestamp,
            version: x.version,
            data: x.data,
        })
        .collect::<Vec<_>>();

    match state.database.add_pins(&pins).await {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            println!("Error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
