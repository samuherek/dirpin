use crate::models::{NewPin, NewSession, NewUser, Pin, User};
use eyre::Result;
use futures_util::TryStreamExt;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use sqlx::FromRow;
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use time::OffsetDateTime;
use tracing::debug;

pub struct DbEntry(pub Pin);
pub struct DbUser(pub User);

impl<'r> FromRow<'r, SqliteRow> for DbEntry {
    fn from_row(row: &'r SqliteRow) -> sqlx::Result<Self> {
        Ok(Self(Pin {
            id: row.try_get("id")?,
            client_id: row.try_get("client_id")?,
            user_id: row.try_get("user_id")?,
            timestamp: row
                .try_get("timestamp")
                .map(|x: i64| OffsetDateTime::from_unix_timestamp_nanos(x as i128).unwrap())?,
            version: row.try_get("version")?,
            data: row.try_get("data")?,
        }))
    }
}

impl<'r> FromRow<'r, SqliteRow> for DbUser {
    fn from_row(row: &'r SqliteRow) -> sqlx::Result<Self> {
        Ok(Self(User {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
            email: row.try_get("email")?,
            password: row.try_get("password")?,
            verified_at: row.try_get("verified_at")?,
            created_at: row.try_get("created_at")?,
        }))
    }
}

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

#[derive(Debug)]
pub enum DbError {
    NotFound,
    // TODO: Not sure if the eyre::Error or eyre::Report is the right thing to have here
    // as it no longer does a nice formatting in the serer axum logging as it addes
    // empty lines and breaks the log flow?
    Other(eyre::Error),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

fn db_error(error: sqlx::Error) -> DbError {
    match error {
        sqlx::Error::RowNotFound => DbError::NotFound,
        error => DbError::Other(error.into()),
    }
}

impl Database {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        debug!("opening database at {:?}", path);
        if !path.exists() {
            if let Some(dir) = path.parent() {
                fs_err::create_dir_all(dir)?;
            }
        }

        let options =
            SqliteConnectOptions::from_str(path.to_str().unwrap())?.create_if_missing(true);
        let pool = SqlitePoolOptions::new().connect_with(options).await?;

        Self::setup_db(&pool).await?;

        Ok(Self { pool })
    }

    async fn setup_db(pool: &SqlitePool) -> Result<()> {
        debug!("setting up database");
        sqlx::migrate!("./migrations").run(pool).await?;

        Ok(())
    }

    pub async fn list_pins(&self) -> Result<Vec<Pin>, DbError> {
        sqlx::query_as("select * from pins")
            .fetch(&self.pool)
            .map_ok(|DbEntry(entry)| entry)
            .try_collect()
            .await
            .map_err(db_error)
    }

    pub async fn add_pins(&self, pins: &[NewPin]) -> Result<(), DbError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;

        for el in pins {
            let created_at = OffsetDateTime::now_utc().to_string();
            sqlx::query(
                r#"
                insert into pins(
                    client_id, user_id, timestamp, version, data, created_at
                ) 
                values(
                    ?1, ?2, ?3, ?4, ?5, ?6
                )
                on conflict(client_id) do update set
                    client_id = ?1,
                    user_id = ?2,
                    timestamp = ?3, 
                    version = ?4, 
                    data = ?5,
                    created_at = ?6
            "#,
            )
            .bind(el.client_id.as_str())
            .bind(el.user_id)
            .bind(el.timestamp.unix_timestamp_nanos() as i64)
            .bind(el.version)
            .bind(el.data.as_str())
            .bind(created_at)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        }

        tx.commit().await.map_err(db_error)?;

        Ok(())
    }

    pub async fn add_user(&self, user: NewUser) -> Result<u32, DbError> {
        let created_at = OffsetDateTime::now_utc().to_string();
        sqlx::query_as(
            r#"
            insert into users(username, email, password, created_at)
            values(?1, ?2, ?3, ?4)
            returning id
            "#,
        )
        .bind(user.username)
        .bind(user.email)
        .bind(user.password)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)
        .map(|(count,)| count)
    }

    pub async fn add_session(&self, session: NewSession) -> Result<(), DbError> {
        sqlx::query(
            r#"
            insert into sessions(user_id, token)
            values(?1, ?2)
            "#,
        )
        .bind(session.user_id)
        .bind(session.token)
        .execute(&self.pool)
        .await
        .map_err(db_error)
        .map(|_| ())
    }

    pub async fn get_user(&self, username: &str) -> Result<User, DbError> {
        sqlx::query_as("select * from users where username = ?1")
            .bind(username)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)
            .map(|DbUser(user)| user)
    }
}
