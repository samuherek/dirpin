use crate::domain::Pin;
use crate::utils::get_host_user;
use dirpin_common::utils;
use eyre::Result;
use sql_builder::{quote, SqlBuilder};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use time::OffsetDateTime;
use tracing::debug;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum FilterMode {
    All,
    Directory,
    Workspace,
}

impl FilterMode {
    pub fn as_str(&self) -> &str {
        match self {
            FilterMode::All => "all",
            FilterMode::Directory => "directory",
            FilterMode::Workspace => "workspace",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Context {
    pub cwd: String,
    pub hostname: String,
    pub cgd: Option<String>,
}

/// We assume that the global context is the root of the computer
/// and we assume there is no "git repo" in the root of the computer
/// TODO: Please check the "get_root_dir" impl for comment about the
/// widnows root dir.
pub fn global_context() -> Context {
    let hostname = get_host_user();
    let cwd = utils::get_rooot_dir();
    Context {
        cwd,
        hostname,
        cgd: None,
    }
}

/// Get the current entry context basd on the current directory path
pub fn current_context() -> Context {
    let hostname = get_host_user();
    let cwd = utils::get_current_dir();
    let cgd = utils::get_git_parent_dir(&cwd);
    Context { cwd, hostname, cgd }
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

    fn map_query_pins(row: SqliteRow) -> Pin {
        let cgd: Option<String> = row.get("cgd");
        let created_at =
            OffsetDateTime::from_unix_timestamp_nanos(row.get::<i64, _>("created_at") as i128)
                .unwrap();
        let updated_at =
            OffsetDateTime::from_unix_timestamp_nanos(row.get::<i64, _>("updated_at") as i128)
                .unwrap();
        let deleted_at: Option<i64> = row.get("deleted_at");
        let deleted_at =
            deleted_at.and_then(|x| OffsetDateTime::from_unix_timestamp_nanos(x as i128).ok());

        Pin {
            id: Uuid::parse_str(row.get("id")).unwrap(),
            data: row.get("data"),
            hostname: row.get("hostname"),
            cwd: row.get("cwd"),
            cgd,
            created_at,
            updated_at,
            deleted_at,
            version: row.get("version"),
        }
    }

    async fn save_raw(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, v: &Pin) -> Result<()> {
        // TODO: Think about using the query! for static checks
        sqlx::query(
            r#"
            insert into pins(
                id, data, hostname, cwd, cgd, created_at, updated_at, deleted_at, version
            ) values(
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9
            )
            on conflict(id) do update set
                data = ?2,
                hostname = ?3,
                cwd = ?4,
                cgd = ?5,
                created_at = ?6,
                updated_at = ?7,
                deleted_at = ?8,
                version = ?9
            "#,
        )
        .bind(v.id.to_string())
        .bind(v.data.as_str())
        .bind(v.hostname.as_str())
        .bind(v.cwd.as_str())
        .bind(v.cgd.as_ref().map(|x| x.as_str()))
        .bind(v.created_at.unix_timestamp_nanos() as i64)
        .bind(v.updated_at.unix_timestamp_nanos() as i64)
        .bind(v.deleted_at.map(|x| x.unix_timestamp_nanos() as i64))
        .bind(v.version)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn save(&self, item: &Pin) -> Result<()> {
        debug!("Saving pin to database");
        let mut tx = self.pool.begin().await?;
        // TODO: if transaction fails, it does not throw error?
        Self::save_raw(&mut tx, item).await?;
        tx.commit().await?;

        Ok(())
    }

    pub async fn save_bulk(&self, items: &[Pin]) -> Result<()> {
        debug!("Saving pins in bulk to database");
        let mut tx = self.pool.begin().await?;
        for el in items {
            Self::save_raw(&mut tx, &el).await?;
        }
        tx.commit().await?;

        Ok(())
    }

    pub async fn after(&self, timestamp: OffsetDateTime) -> Result<Vec<Pin>> {
        debug!("Query pins before from datbase");
        let res = sqlx::query("select * from pins where updated_at > ?1")
            .bind(timestamp.unix_timestamp_nanos() as i64)
            .map(Self::map_query_pins)
            .fetch_all(&self.pool)
            .await?;

        Ok(res)
    }

    pub async fn list(
        &self,
        filters: &[FilterMode],
        context: &Context,
        search: &str,
    ) -> Result<Vec<Pin>> {
        let mut query = SqlBuilder::select_from("pins");
        query.field("*").order_desc("updated_at");
        for filter in filters {
            match filter {
                FilterMode::All => &mut query,
                FilterMode::Directory => query.and_where_eq("cwd", quote(&context.cwd)),
                FilterMode::Workspace => query.and_where_eq("cwd", quote(&context.cwd)),
            };
        }
        if !search.is_empty() {
            query.and_where_like_any("data", search);
        }

        let query = query.sql().expect("Failed to parse query");
        let res = sqlx::query(&query)
            .map(Self::map_query_pins)
            .fetch_all(&self.pool)
            .await?;

        Ok(res)
    }
}
