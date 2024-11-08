use crate::authentication::hash_password;
use crate::models::{NewPin, NewSession, NewUser};
use crate::router::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use dirpin_common::api::{
    AddPinRequest, HealthCheckResponse, RegisterRequest, RegisterResponse, SyncRequest,
    SyncResponse,
};
use dirpin_common::utils::crypto_random_string;
use tracing::error;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ErrorMessage<'a> {
    pub value: &'a str,
}

pub struct ResponseError<'a> {
    pub error: ErrorMessage<'a>,
    pub status: StatusCode,
}

impl<'a> IntoResponse for ResponseError<'a> {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(self.error)).into_response()
    }
}

pub trait ResponseErrExt<'a> {
    fn with_status(self, status: StatusCode) -> ResponseError<'a>;
    fn value(value: &'a str) -> Self;
}

impl<'a> ResponseErrExt<'a> for ErrorMessage<'a> {
    fn with_status(self, status: StatusCode) -> ResponseError<'a> {
        ResponseError {
            error: self,
            status,
        }
    }

    fn value(value: &'a str) -> Self {
        ErrorMessage { value }
    }
}

pub async fn index() -> Result<Json<HealthCheckResponse>, ResponseError<'static>> {
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
) -> Result<Json<SyncResponse>, ResponseError<'static>> {
    let res = state.database.list_pins().await.map_err(|err| {
        error!("failed querying entries {err}");
        ErrorMessage::value("Server error").with_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    Ok(Json(SyncResponse {
        updated: res.into_iter().map(|x| x.data).collect::<Vec<_>>(),
        deleted: vec![],
    }))
}

pub async fn add(
    state: State<AppState>,
    Json(req): Json<Vec<AddPinRequest>>,
) -> Result<impl IntoResponse, ResponseError<'static>> {
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

    state.database.add_pins(&pins).await.map_err(|e| {
        error!("failed adding entries {e}");
        ErrorMessage::value("Server error").with_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    Ok(StatusCode::OK)
}

pub async fn register(
    state: State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, ResponseError<'static>> {
    if !req
        .username
        .chars()
        .all(|x| x.is_ascii_alphanumeric() || x == '-' || x == '_')
    {
        return Err(ErrorMessage::value(
            "Only alphanumeric, hypens and underscrores are allwoed in username",
        )
        .with_status(StatusCode::BAD_REQUEST));
    }

    let hashed_password = hash_password(&req.password).map_err(|_| {
        ErrorMessage::value("Failed to register user").with_status(StatusCode::BAD_REQUEST)
    })?;

    let new_user = NewUser {
        email: req.email,
        username: req.username,
        password: hashed_password,
    };
    let user_id = state.database.add_user(new_user).await.map_err(|err| {
        error!("failed saving user {err}");
        ErrorMessage::value("Failed to register user").with_status(StatusCode::BAD_REQUEST)
    })?;

    let token = crypto_random_string::<24>();
    let new_session = NewSession {
        user_id,
        token: (&token).into(),
    };

    // TODO:: verification step
    // TODO:: add created_at timestamp so we can have an expiration time on the session
    state
        .database
        .add_session(new_session)
        .await
        .map_err(|err| {
            error!("failed creating session {err}");
            ErrorMessage::value("Failed to register user").with_status(StatusCode::BAD_REQUEST)
        })?;

    Ok(Json(RegisterResponse { session: token }))
}
