use crate::database::{Database, DbError};
use crate::handlers::ServerError;
use crate::models::User;
use crate::router::AppState;
use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordVerifier, Version};
use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::{self, request::Parts};
use eyre::Context;
use time::{Duration, OffsetDateTime};
use tracing::error;

pub struct UserSession {
    user: User,
    token: String,
}

impl UserSession {
    pub fn user(&self) -> &User {
        &self.user
    }

    pub fn token(&self) -> &str {
        self.token.as_ref()
    }
}

#[async_trait]
impl FromRequestParts<AppState> for UserSession {
    type Rejection = ServerError;

    async fn from_request_parts(
        req: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = req
            .headers
            .get(http::header::AUTHORIZATION)
            .ok_or_else(|| ServerError::BadRequest("missing autorization header"))?;

        let auth_header = auth_header
            .to_str()
            .map_err(|_| ServerError::BadRequest("invalid autorization header encoding"))?;

        let (typ, token) = auth_header
            .split_once(' ')
            .ok_or_else(|| ServerError::BadRequest("invalid autorization header encoding"))?;

        if typ != "Token" {
            return Err(ServerError::BadRequest(
                "invalid autorization header encoding",
            ));
        }

        let user = state
            .database
            .get_session_user(token)
            .await
            .map_err(|err| match err {
                DbError::NotFound => ServerError::NotFound("session not found"),
                DbError::Other(err) => {
                    error!("database error {err}");
                    ServerError::UnexpectedError("failed to get session")
                }
            })?;

        Ok(UserSession {
            user,
            token: token.into(),
        })
    }
}

pub(crate) fn hash_password(value: &str) -> eyre::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(value.as_bytes(), &salt)?
    .to_string();
    Ok(password_hash)
}

pub(crate) fn session_expires_at() -> OffsetDateTime {
    OffsetDateTime::now_utc().saturating_add(Duration::weeks(12))
}

pub(crate) fn verify_password_hash(
    expected: &str,
    provided: &str,
) -> eyre::Result<(), ServerError> {
    let expected_password_hash = PasswordHash::new(expected).map_err(|err| {
        error!("Failed to parse hash in PHC string format. {err}");
        ServerError::UnexpectedError("Failed to parse hash")
    })?;

    Argon2::default()
        .verify_password(provided.as_bytes(), &expected_password_hash)
        .context("Invalid password.")
        .map_err(|_| ServerError::InvalidCredentials)
}

pub(crate) async fn validate_credentials(
    db: &Database,
    username: &str,
    password: &str,
) -> Result<User, ServerError> {
    let mut user = None;
    let mut expected_password_hash = "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
        .to_string();

    match db.get_user(username).await {
        Ok(u) => {
            expected_password_hash = u.password.clone();
            user = Some(u);
        }
        Err(DbError::NotFound) => {}
        Err(DbError::Other(err)) => {
            error!("failed query {err}");
            return Err(ServerError::UnexpectedError("Failed to execute query"));
        }
    }

    verify_password_hash(&expected_password_hash, password)?;

    user.ok_or_else(|| eyre::eyre!("Unknown user."))
        .map_err(|_| ServerError::InvalidCredentials)
}
