use crate::models::NewPin;
use eyre::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
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

    pub async fn add_pins(&self, pins: &[NewPin]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for el in pins {
            let created_at = OffsetDateTime::now_utc().to_string();
            sqlx::query(
                r#"
                insert into pins(client_id, user_id, timestamp, version, data, created_at) 
                values(?1, ?2, ?3, ?4, ?5, ?6)
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
}
