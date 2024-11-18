use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use dirpin_common::api::ErrorMessage;
use tracing::error;

// TODO figure out what interface to implement so that I can do "map_err(ServerError::Validation)
#[derive(thiserror::Error, Debug)]
pub enum ServerError {
    #[error("Database error: {0}")]
    DatabaseError(&'static str),

    #[error("Bad request: {0}")]
    BadRequest(&'static str),

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

    #[error("Conflict: {0}")]
    Conflict(&'static str),
}

impl ServerError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ServerError::NotFound(_) => StatusCode::NOT_FOUND,
            ServerError::Validation(_) => StatusCode::BAD_REQUEST,
            ServerError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ServerError::InvalidCredentials => StatusCode::BAD_REQUEST,
            ServerError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ServerError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::Conflict(_) => StatusCode::CONFLICT,
        }
    }

    pub fn message(&self) -> String {
        match self {
            ServerError::NotFound(v) => v.to_string(),
            ServerError::Validation(v) => v.to_string(),
            ServerError::BadRequest(v) => v.to_string(),
            ServerError::Conflict(v) => v.to_string(),
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

        (
            status,
            Json(ErrorMessage {
                value: value.clone(),
            }),
        )
            .into_response()
    }
}
