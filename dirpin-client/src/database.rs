use crate::domain::Pin;
use crate::settings::Settings;
use crate::utils::get_host_user;
use dirpin_common::utils;
use eyre::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use tracing::debug;

#[derive(Debug)]
pub struct Context {
    pub cwd: String,
    pub hostname: String,
    pub cgd: Option<String>,
    pub host_id: String,
}

pub fn current_context() -> Context {
    let hostname = get_host_user();
    let cwd = utils::get_current_dir();
    let cgd = utils::get_git_parent_dir(&cwd);
    let host_id = Settings::host_id().to_string();
    Context {
        cwd,
        hostname,
        cgd,
        host_id,
    }
}

pub struct Database {
    pub pool: SqlitePool,
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

    async fn save_raw(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, v: &Pin) -> Result<()> {
        sqlx::query(
            r#"
            insert or ignore into pins(
                id, data, hostname, cwd, cgd, created_at, updated_at, deleted_at
            ) values(
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8
            ) "#,
        )
        .bind(v.id.to_string())
        .bind(v.data.as_str())
        .bind(v.hostname.as_str())
        .bind(v.cwd.as_str())
        .bind(v.cgd.as_ref().map(|x| x.as_str()))
        .bind(v.created_at)
        .bind(v.created_at)
        .bind(v.deleted_at)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn save(&self, item: &Pin) -> Result<()> {
        debug!("Saving pin to database");
        let mut tx = self.pool.begin().await?;
        Self::save_raw(&mut tx, item).await?;
        tx.commit().await?;
        Ok(())
    }
}
