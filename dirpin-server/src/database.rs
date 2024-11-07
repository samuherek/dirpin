use crate::models::{DbPin, NewPin, NewSession, NewUser};
use eyre::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use time::OffsetDateTime;
use tracing::debug;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
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

    // TODO: Redo from NewPin to actual DbPin
    fn map_quert_pins(row: SqliteRow) -> DbPin {
        DbPin {
            id: row.get("id"),
            client_id: row.get("client_id"),
            user_id: row.get("user_id"),
            timestamp: OffsetDateTime::from_unix_timestamp_nanos(
                row.get::<i64, _>("timestamp") as i128
            )
            .unwrap(),
            version: row.get("version"),
            data: row.get("data"),
        }
    }

    pub async fn list_pins(&self) -> Result<Vec<DbPin>> {
        let res = sqlx::query("select * from pins")
            .map(Self::map_quert_pins)
            .fetch_all(&self.pool)
            .await?;

        Ok(res)
    }

    pub async fn add_pins(&self, pins: &[NewPin]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

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
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn add_user(&self, user: NewUser) -> Result<u32> {
        let created_at = OffsetDateTime::now_utc().to_string();
        let res: (u32,) = sqlx::query_as(
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
        .await?;

        Ok(res.0)
    }

    pub async fn add_session(&self, session: NewSession) -> Result<()> {
        sqlx::query(
            r#"
            insert into sessions(user_id, token)
            values(?1, ?2)
            "#,
        )
        .bind(session.user_id)
        .bind(session.token)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
