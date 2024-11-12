use crate::authentication::{hash_password, session_expires_at, validate_credentials, UserSession};
use crate::database::DbError;
use crate::handlers::ServerError;
use crate::models::{HostSession, NewSession, NewUser, RenewSession};
use crate::router::AppState;
use axum::extract::State;
use axum::response::Json;
use dirpin_common::api::{
    LoginRequest, LoginResponse, LogoutResponse, RegisterRequest, RegisterResponse,
};
use dirpin_common::utils::crypto_random_string;
use tracing::error;

pub async fn register(
    state: State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, ServerError> {
    if !req
        .username
        .chars()
        .all(|x| x.is_ascii_alphanumeric() || x == '-' || x == '_')
    {
        return Err(ServerError::Validation(
            "Only alphanumeric, hypens and underscrores are allwoed in username",
        ));
    }

    // TODO: try to get the user from the db based on the username to make sure that the user
    // can not create a duplicate account. Or otherwise, the unique constraint will fail in the
    // database query.

    let hashed_password = hash_password(&req.password).map_err(|err| {
        error!("Failed to hash password {err}");
        ServerError::UnexpectedError("Failed to register user")
    })?;

    let new_user = NewUser {
        email: req.email,
        username: req.username,
        password: hashed_password,
    };
    let user_id = state.database.add_user(new_user).await.map_err(|err| {
        error!("Failed saving user: {err}");
        ServerError::UnexpectedError("Failed to register user")
    })?;

    let token = crypto_random_string::<24>();
    let expires_at = session_expires_at();

    let new_session = NewSession {
        user_id,
        host_id: req.host_id,
        token: (&token).into(),
        expires_at,
    };

    // TODO:: verification step for the user. don't return session yet. We first want to verify the
    // user.

    state
        .database
        .add_session(new_session)
        .await
        .map_err(|err| {
            error!("Failed to create session: {err}");
            ServerError::UnexpectedError("Failed to register user")
        })?;

    Ok(Json(RegisterResponse { session: token }))
}

pub async fn login(
    state: State<AppState>,
    req: Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ServerError> {
    // TODO: check if the user is verified. If not, return and ask the user to verify!

    let user = validate_credentials(&state.database, &req.username, &req.password).await?;

    let host_session = HostSession {
        user_id: user.id,
        host_id: req.host_id.to_owned(),
    };

    let session = match state.database.get_host_session(host_session).await {
        Ok(session) => session,
        Err(DbError::Other(err)) => {
            error!("Database error: {err}");
            return Err(ServerError::UnexpectedError("Database error"));
        }
        _ => None,
    };

    let next_token = crypto_random_string::<24>();
    let next_expires_at = session_expires_at();

    match session {
        Some(s) => {
            let renew_session = RenewSession {
                id: s.id,
                token: next_token.clone(),
                expires_at: next_expires_at,
            };
            state.database.renew_session(renew_session).await
        }
        None => {
            let new_session = NewSession {
                user_id: user.id,
                host_id: req.host_id.to_owned(),
                token: next_token.clone(),
                expires_at: next_expires_at,
            };
            state.database.add_session(new_session).await
        }
    }
    .map_err(|err| {
        error!("Database error: {err}");
        ServerError::UnexpectedError("Database error")
    })?;

    Ok(Json(LoginResponse {
        session: next_token,
    }))
}

// TODO:: we don't actually use this in the dirpin client. 
// But we might want to use it at some point. In the client
// we just remove the session from the host. 
pub async fn logout(
    session: UserSession,
    state: State<AppState>,
) -> Result<Json<LogoutResponse>, ServerError> {
    match state.database.get_session(session.token()).await {
        Err(DbError::Other(err)) => {
            error!("Database error: {err}");
            return Err(ServerError::UnexpectedError("Database error"));
        }
        Ok(Some(s)) => Some(s),
        // TODO: I really don't like this error handling. We need to split NOT FOUND from the
        // database error.
        _ => None,
    }
    .ok_or(ServerError::NotFound("session"))?;

    state
        .database
        .remove_session(session.token())
        .await
        .map_err(|err| {
            error!("Database error: {err}");
            ServerError::UnexpectedError("Database error")
        })?;

    Ok(Json(LogoutResponse { ok: true }))
}
