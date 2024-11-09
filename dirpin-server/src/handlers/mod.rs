use crate::models::NewPin;
use crate::router::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use dirpin_common::api::{AddPinRequest, HealthCheckResponse, SyncRequest, SyncResponse};
use tracing::error;

pub mod user;

const VERSION: &str = env!("CARGO_PKG_VERSION");

// TODO figure out what interface to implement so that I can do "map_err(ServerError::Validation)
#[derive(thiserror::Error, Debug)]
pub enum ServerError {
    #[error("Database error: {0}")]
    DatabaseError(&'static str),

    #[error("Not found: {0}")]
    NotFound(&'static str),

    #[error("Incorrect input: {0}")]
    Validation(&'static str),

    #[error("Invalid credentails")]
    InvalidCredentials,

    #[error("Unauthorized: {0}")]
    Unauthorized(&'static str),

    #[error("Unexpected error: {0}")]
    UnexpectedError(&'static str),
}

impl ServerError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ServerError::NotFound(_) => StatusCode::NOT_FOUND,
            ServerError::Validation(_) => StatusCode::BAD_REQUEST,
            ServerError::InvalidCredentials => StatusCode::BAD_REQUEST,
            ServerError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ServerError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn message(&self) -> String {
        match self {
            ServerError::NotFound(v) => v.to_string(),
            ServerError::Validation(v) => v.to_string(),
            ServerError::InvalidCredentials => "Invalid credentails".to_string(),
            ServerError::Unauthorized(v) => v.to_string(),
            ServerError::UnexpectedError(_) | ServerError::DatabaseError(_) => {
                "An unexpected error occured. Please try agian later".into()
            }
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status_code();
        let value = self.message();

        match self {
            ServerError::Validation(_) => {}
            ServerError::Unauthorized(_) => {}
            ServerError::NotFound(_) => {}
            e => {
                error!("Error: {e:?}");
            }
        }

        (
            status,
            Json(ErrorMessage {
                value: value.clone(),
            }),
        )
            .into_response()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ErrorMessage {
    pub value: String,
}

pub async fn index() -> Result<Json<HealthCheckResponse>, ServerError> {
    let version = VERSION.to_string();

    Ok(Json(HealthCheckResponse {
        status: "Ok".to_string(),
        version,
    }))
}

// TODO: make a propert error response types
pub async fn sync(
    state: State<AppState>,
    _params: Query<SyncRequest>,
) -> Result<Json<SyncResponse>, ServerError> {
    let res = state.database.list_pins().await.map_err(|err| {
        error!("Failed to list entries {err}");
        ServerError::DatabaseError("list entries")
    })?;

    Ok(Json(SyncResponse {
        updated: res.into_iter().map(|x| x.data).collect::<Vec<_>>(),
        deleted: vec![],
    }))
}

pub async fn add(
    state: State<AppState>,
    Json(req): Json<Vec<AddPinRequest>>,
) -> Result<impl IntoResponse, ServerError> {
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

    state.database.add_pins(&pins).await.map_err(|err| {
        error!("Failed to add entries {err}");
        ServerError::DatabaseError("add entries")
    })?;

    Ok(StatusCode::OK)
}
