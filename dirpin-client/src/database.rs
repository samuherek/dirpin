use crate::domain::{Entry, EntryKind};
use crate::utils::get_host_user;
use dirpin_common::utils;
use eyre::Result;
use futures_util::TryStreamExt;
use sql_builder::{quote, SqlBuilder};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use sqlx::{FromRow, Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use time::OffsetDateTime;
use tracing::debug;
use uuid::Uuid;

// timestamp/updated_at -> unix timestamp with nanoseconds for precision
// expires_at/created_at/deleted_at -> unix timestamp

pub struct DbEntry(pub Entry);

impl<'r> FromRow<'r, SqliteRow> for DbEntry {
    fn from_row(row: &'r SqliteRow) -> sqlx::Result<Self> {
        Ok(Self(Entry {
            id: row
                .try_get("id")
                .map(|x: &str| Uuid::parse_str(x).unwrap())?,
            value: row.try_get("value")?,
            data: row.try_get("data")?,
            // TODO: fix this deserialization with the serde_json
            kind: row
                .try_get("kind")
                .map(|x: &str| EntryKind::from_str(x).unwrap())?,
            hostname: row.try_get("hostname")?,
            cwd: row.try_get("cwd")?,
            cgd: row.try_get("cgd")?,
            created_at: row
                .try_get("created_at")
                .map(|x: i64| OffsetDateTime::from_unix_timestamp(x).unwrap())?,
            updated_at: row
                .try_get("updated_at")
                .map(|x: i64| OffsetDateTime::from_unix_timestamp_nanos(x as i128).unwrap())?,
            deleted_at: row
                .try_get("deleted_at")
                .map(|x: Option<i64>| x.map(|y| OffsetDateTime::from_unix_timestamp(y).unwrap()))?,
            version: row.try_get("version")?,
        }))
    }
}

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

    async fn save_raw(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, v: &Entry) -> Result<()> {
        sqlx::query(
            r#"
            insert into entries(
                id, value, data, kind, hostname, cwd, cgd, created_at, updated_at, deleted_at, version
            ) values(
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
            )
            on conflict(id) do update set
                value = ?2,
                data = ?3,
                kind = ?4,
                hostname = ?5,
                cwd = ?6,
                cgd = ?7,
                created_at = ?8,
                updated_at = ?9,
                deleted_at = ?10,
                version = ?11
            "#,
        )
            .bind(v.id.to_string())
            .bind(v.value.as_str())
            .bind(v.data.to_owned())
            .bind(v.kind.as_str())
            .bind(v.hostname.as_str())
            .bind(v.cwd.as_str())
            .bind(v.cgd.to_owned())
            .bind(v.created_at.unix_timestamp())
            .bind(v.updated_at.unix_timestamp_nanos() as i64)
            .bind(v.deleted_at.map(|x| x.unix_timestamp()))
            .bind(v.version)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn save(&self, item: &Entry) -> Result<()> {
        debug!("Saving pin to database");
        let mut tx = self.pool.begin().await?;
        // TODO: if transaction fails, it does not throw error?
        Self::save_raw(&mut tx, item).await?;
        tx.commit().await?;

        Ok(())
    }

    pub async fn save_bulk(&self, items: &[Entry]) -> Result<()> {
        debug!("Saving entries in bulk to database");
        let mut tx = self.pool.begin().await?;
        for el in items {
            Self::save_raw(&mut tx, &el).await?;
        }
        tx.commit().await?;

        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("update entries set deleted_at = ?2, updated_at = ?2 where id = ?1")
            .bind(id.to_string())
            .bind(OffsetDateTime::now_utc().unix_timestamp())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn after(&self, timestamp: OffsetDateTime) -> Result<Vec<Entry>> {
        debug!("Query entries before from datbase");
        let res = sqlx::query_as("select * from entries where updated_at > ?1")
            .bind(timestamp.unix_timestamp_nanos() as i64)
            .fetch(&self.pool)
            .map_ok(|DbEntry(entry)| entry)
            .try_collect()
            .await?;

        Ok(res)
    }

    pub async fn list(
        &self,
        filters: &[FilterMode],
        context: &Context,
        search: &str,
    ) -> Result<Vec<Entry>> {
        let mut query = SqlBuilder::select_from("entries");
        query.field("*").order_desc("updated_at");
        query.and_where_is_null("deleted_at");
        for filter in filters {
            match filter {
                FilterMode::All => &mut query,
                FilterMode::Directory => query.and_where_eq("cwd", quote(&context.cwd)),
                FilterMode::Workspace => query.and_where_eq(
                    "cgd",
                    quote(
                        context
                            .cgd
                            .as_ref()
                            .unwrap_or(&"XXXXXXXXXXXXXX".to_string()),
                    ),
                ),
            };
        }

        if !search.is_empty() {
            query.and_where_like_any("value", search);
        }

        let query = query.sql().expect("Failed to parse query");
        let res = sqlx::query_as(&query)
            .fetch(&self.pool)
            .map_ok(|DbEntry(entry)| entry)
            .try_collect()
            .await?;

        Ok(res)
    }

    pub async fn count(
        &self,
        filters: &[FilterMode],
        context: &Context,
        search: &str,
    ) -> Result<i64> {
        let mut query = SqlBuilder::select_from("entries");
        query.field("count(1)");
        query.and_where_is_null("deleted_at");
        for filter in filters {
            match filter {
                FilterMode::All => &mut query,
                FilterMode::Directory => query.and_where_eq("cwd", quote(&context.cwd)),
                FilterMode::Workspace => query.and_where_eq(
                    "cgd",
                    quote(
                        context
                            .cgd
                            .as_ref()
                            .unwrap_or(&"XXXXXXXXXXXXXX".to_string()),
                    ),
                ),
            };
        }

        if !search.is_empty() {
            query.and_where_like_any("value", search);
        }

        let query = query.sql().expect("Failed to parse query");
        let res: (i64,) = sqlx::query_as(&query).fetch_one(&self.pool).await?;

        Ok(res.0)
    }
}
