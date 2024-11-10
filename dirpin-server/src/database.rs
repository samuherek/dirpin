use crate::models::{
    Entry, HostSession, NewEntry, NewSession, NewUser, RenewSession, Session, User,
};
use eyre::Result;
use futures_util::TryStreamExt;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use sqlx::FromRow;
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use time::OffsetDateTime;
use tracing::debug;

// timestamp/updated_at -> unix timestamp with nanoseconds for precision
// expires_at/created_at/deleted_at -> unix timestamp

pub struct DbEntry(pub Entry);
pub struct DbUser(pub User);
pub struct DbSession(pub Session);

impl<'r> FromRow<'r, SqliteRow> for DbEntry {
    fn from_row(row: &'r SqliteRow) -> sqlx::Result<Self> {
        Ok(Self(Entry {
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
            created_at: row
                .try_get("created_at")
                .map(|x: i64| OffsetDateTime::from_unix_timestamp(x).unwrap())?,
        }))
    }
}

impl<'r> FromRow<'r, SqliteRow> for DbSession {
    fn from_row(row: &'r SqliteRow) -> sqlx::Result<Self> {
        Ok(Self(Session {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            host_id: row.try_get("host_id").ok(),
            token: row.try_get("token")?,
            expires_at: row
                .try_get("expires_at")
                .map(|x: i64| OffsetDateTime::from_unix_timestamp(x).unwrap())?,
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

impl std::error::Error for DbError {}

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

    pub async fn list_entries(&self) -> Result<Vec<Entry>, DbError> {
        sqlx::query_as("select * from entries")
            .fetch(&self.pool)
            .map_ok(|DbEntry(entry)| entry)
            .try_collect()
            .await
            .map_err(db_error)
    }

    pub async fn add_entries(&self, entries: &[NewEntry]) -> Result<(), DbError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;

        for el in entries {
            sqlx::query(
                r#"
                insert into entries(
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
            .bind(OffsetDateTime::now_utc().unix_timestamp())
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        }

        tx.commit().await.map_err(db_error)?;

        Ok(())
    }

    pub async fn add_user(&self, user: NewUser) -> Result<u32, DbError> {
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
        .bind(OffsetDateTime::now_utc().unix_timestamp())
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)
        .map(|(count,)| count)
    }

    pub async fn add_session(&self, session: NewSession) -> Result<(), DbError> {
        sqlx::query(
            r#"
            insert into sessions(user_id, host_id, token, expires_at)
            values(?1, ?2, ?3, ?4)
            "#,
        )
        .bind(session.user_id)
        .bind(session.host_id)
        .bind(session.token)
        .bind(session.expires_at.unix_timestamp())
        .execute(&self.pool)
        .await
        .map_err(db_error)
        .map(|_| ())
    }

    pub async fn get_host_session(&self, session: HostSession) -> Result<Option<Session>, DbError> {
        sqlx::query_as(
            r#"
            select * from sessions 
            where user_id = ?1 and host_id = ?2 and expires_at > strftime('%s', 'now')
        "#,
        )
        .bind(session.user_id)
        .bind(session.host_id)
        .fetch_optional(&self.pool)
        .await
        // TODO: here we don't have to cover all the database errors. Just the relevant one which
        // is failure. Please review this and see how to go about making the errors a bit better
        // for all the different queries.
        .map_err(db_error)
        .map(|x| x.map(|DbSession(session)| session))
    }

    pub async fn renew_session(&self, session: RenewSession) -> Result<(), DbError> {
        sqlx::query(
            r#"
               update sessions  
               set token = ?2, expires_at = ?3
               where id = ?1
            "#,
        )
        .bind(session.id)
        .bind(session.token)
        .bind(session.expires_at.unix_timestamp())
        .execute(&self.pool)
        .await
        .map_err(db_error)
        .map(|_| ())
    }

    pub async fn remove_session(&self, token: &str) -> Result<(), DbError> {
        sqlx::query(
            r#"
               delete from sessions  
               where token = ?1 and expires_at > strftime('%s', 'now')
            "#,
        )
        .bind(token)
        .execute(&self.pool)
        .await
        .map_err(db_error)
        .map(|_| ())
    }

    pub async fn get_session(&self, token: &str) -> Result<Option<Session>, DbError> {
        sqlx::query_as(
            r#"
            select * from sessions 
            where token = ?1 and expires_at > strftime('%s', 'now')
        "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)
        .map(|x| x.map(|DbSession(session)| session))
    }

    pub async fn get_session_user(&self, token: &str) -> Result<User, DbError> {
        sqlx::query_as(
            r#"
            select * from users 
            left join sessions on sessions.user_id = users.id
            where sessions.token = ?1 and sessions.expires_at > strftime('%s', 'now')
        "#,
        )
        .bind(token)
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)
        .map(|DbUser(user)| user)
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
