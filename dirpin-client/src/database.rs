use crate::domain::conflict::Conflict;
use crate::domain::context::Context;
use crate::domain::entry::{Entry, EntryKind};
use crate::domain::host::HostId;
use crate::domain::workspace::{Workspace, WorkspaceId, WorkspacePath};
use dirpin_common::api::RefDelete;
use dirpin_common::domain::SyncVersion;
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
pub struct DbWorkspace(pub Workspace);

impl<'r> FromRow<'r, SqliteRow> for DbEntry {
    fn from_row(row: &'r SqliteRow) -> sqlx::Result<Self> {
        Ok(Self(Entry {
            id: row
                .try_get("id")
                .map(|x: &str| Uuid::parse_str(x).unwrap())?,
            value: row.try_get("value")?,
            desc: row.try_get("desc")?,
            data: row.try_get("data")?,
            kind: row
                .try_get("kind")
                .map(|x: &str| EntryKind::from_str(x).unwrap())?,
            path: row.try_get("path")?,
            updated_at: row
                .try_get("updated_at")
                .map(|x: i64| OffsetDateTime::from_unix_timestamp_nanos(x as i128).unwrap())?,
            deleted_at: row
                .try_get("deleted_at")
                .map(|x: Option<i64>| x.map(|y| OffsetDateTime::from_unix_timestamp(y).unwrap()))?,
            version: row.try_get("version").map(|x: u32| SyncVersion::from(x))?,
            workspace_id: row.try_get("workspace_id").map(|x: &str| x.parse().ok())?,
            host_id: row
                .try_get("host_id")
                .map(|x: &str| HostId::from_str(x).unwrap())?,
        }))
    }
}

impl<'r> FromRow<'r, SqliteRow> for DbWorkspace {
    fn from_row(row: &'r SqliteRow) -> sqlx::Result<Self> {
        Ok(Self(Workspace {
            id: row.try_get("id").map(|x: &str| x.parse().unwrap())?,
            name: row.try_get("name")?,
            git: row.try_get("git")?,
            paths: row.try_get("paths").map(|x: &str| {
                // TODO: theoretically, there can be a "," in a path that would fail to correctly
                // deserialize the path from the list.
                x.split(",")
                    .map(|y| WorkspacePath::try_from(y).unwrap())
                    .collect()
            })?,
            updated_at: row
                .try_get("updated_at")
                .map(|x: i64| OffsetDateTime::from_unix_timestamp_nanos(x as i128).unwrap())?,
            deleted_at: row
                .try_get("deleted_at")
                .map(|x: Option<i64>| x.map(|y| OffsetDateTime::from_unix_timestamp(y).unwrap()))?,
            version: row.try_get("version").map(|x: u32| SyncVersion::from(x))?,
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

    async fn save_tx(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, v: &Entry) -> Result<()> {
        sqlx::query(
            r#"
            insert into entries(
                id, value, desc, data, kind, path, updated_at, deleted_at, version, workspace_id, host_id
            ) values(
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
            )
            on conflict(id) do update set
                value = ?2,
                desc = ?3,
                data = ?4,
                kind = ?5,
                path = ?6,
                updated_at = ?7,
                deleted_at = ?8,
                version = ?9,
                workspace_id = ?10,
                host_id = ?11
            "#,
        )
            .bind(v.id.to_string())
            .bind(v.value.as_str())
            .bind(v.desc.to_owned())
            .bind(v.data.to_owned())
            .bind(v.kind.as_str())
            .bind(v.path.as_str())
            .bind(v.updated_at.unix_timestamp_nanos() as i64)
            .bind(v.deleted_at.map(|x| x.unix_timestamp()))
            .bind(v.version.inner())
            .bind(v.workspace_id.as_ref().map(|x| x.to_string()))
            .bind(v.host_id.to_string())
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn save(&self, item: &Entry) -> Result<()> {
        debug!("Saving entry to database");
        let mut tx = self.pool.begin().await?;
        // TODO: if transaction fails, it does not throw error?
        Self::save_tx(&mut tx, item).await?;
        tx.commit().await?;

        Ok(())
    }

    pub async fn save_bulk(&self, items: &[Entry]) -> Result<()> {
        debug!("Saving entries in bulk to database");
        let mut tx = self.pool.begin().await?;
        for el in items {
            Self::save_tx(&mut tx, &el).await?;
        }
        tx.commit().await?;

        Ok(())
    }

    pub async fn workspace(
        &self,
        workspace_id: Option<WorkspaceId>,
        workspace_name: Option<String>,
        context: &Context,
    ) -> Result<Option<Workspace>> {
        debug!("Get workspace from database");
        let mut query = SqlBuilder::select_from("workspaces");
        query.field("*");
        query.and_where_is_null("deleted_at");

        let host_path =
            WorkspacePath::new(context.host_id.clone(), context.path.clone()).to_string();
        query.and_where_like_any("paths", host_path);

        match workspace_id {
            Some(id) => query.and_where_eq("id", id.to_string()),
            None => &mut query,
        };

        match &context.git {
            Some(git) => query.and_where_eq("git", quote(git)),
            None => &mut query,
        };

        match workspace_name {
            Some(name) => query.and_where_eq("name", quote(name.to_string())),
            None => &mut query,
        };

        let query = query.sql().expect("Failed to parse query");
        let res = sqlx::query_as(&query)
            .fetch_optional(&self.pool)
            .await?
            .map(|DbWorkspace(ws)| ws);

        Ok(res)
    }

    pub async fn list_workspaces(&self, search: &str) -> Result<Vec<Workspace>> {
        debug!("Query workspaces from datbase");
        let mut query = SqlBuilder::select_from("workspaces");
        query.field("*");

        if !search.is_empty() {
            query.and_where_like_any("value", search);
        }

        let query = query.sql().expect("Failed to parse query");
        let res = sqlx::query_as(&query)
            .fetch(&self.pool)
            .map_ok(|DbWorkspace(ws)| ws)
            .try_collect()
            .await?;

        Ok(res)
    }

    pub async fn after_workspaces(&self, updated_at: OffsetDateTime) -> Result<Vec<Workspace>> {
        debug!("Query workspaces before from datbase");
        let res = sqlx::query_as(
            "select * from workspaces where updated_at >= ?1 and deleted_at is null",
        )
        .bind(updated_at.unix_timestamp_nanos() as i64)
        .fetch(&self.pool)
        .map_ok(|DbWorkspace(ws)| ws)
        .try_collect()
        .await?;

        Ok(res)
    }

    pub async fn deleted_after_workspaces(
        &self,
        deleted_at: OffsetDateTime,
    ) -> Result<Vec<Workspace>> {
        debug!("Query deleted workspaces before from datbase");
        let res = sqlx::query_as("select * from workspaces where deleted_at >= ?1")
            .bind(deleted_at.unix_timestamp())
            .fetch(&self.pool)
            .map_ok(|DbWorkspace(ws)| ws)
            .try_collect()
            .await?;

        Ok(res)
    }

    pub async fn save_workspace(&self, v: &Workspace) -> Result<()> {
        debug!("Saving workspace to database");
        let mut tx = self.pool.begin().await?;
        // TODO: if transaction fails, it does not throw error?
        Self::save_workspace_tx(&mut tx, v).await?;
        tx.commit().await?;

        Ok(())
    }

    async fn save_workspace_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        v: &Workspace,
    ) -> Result<()> {
        debug!("Saving workspace in database");
        sqlx::query(
            r#"
            insert into workspaces(
                id, name, git, paths, updated_at, deleted_at, version
            )
            values(
                ?1, ?2, ?3, ?4, ?5, ?6, ?7
            ) 
            on conflict(id) do update set 
                id = ?1,
                name = ?2,
                git = ?3,
                paths = ?4,
                updated_at = ?5,
                deleted_at = ?6,
                version = ?7
            "#,
        )
        .bind(v.id.to_string())
        .bind(v.name.as_str())
        .bind(v.git.as_ref().map(|x| x.as_str()))
        .bind(
            v.paths
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(","),
        )
        .bind(v.updated_at.unix_timestamp_nanos() as i64)
        .bind(v.deleted_at.map(|x| x.unix_timestamp()))
        .bind(v.version.inner())
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn save_workspace_bulk(&self, items: &[Workspace]) -> Result<()> {
        debug!("Saving workspace in bulk to database");
        let mut tx = self.pool.begin().await?;
        for el in items {
            Self::save_workspace_tx(&mut tx, &el).await?;
        }
        tx.commit().await?;

        Ok(())
    }

    async fn delete_workspace_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        v: &RefDelete,
    ) -> Result<()> {
        sqlx::query(
            r#"
            insert into workspaces(
                id, name, git, paths, updated_at, deleted_at, version
            )
            values(
                ?1, ?2, ?3, ?4, ?5, ?6, ?7
            ) 
            on conflict(id) do update set 
                name = ?2,
                git = ?3,
                paths = ?4,
                updated_at = ?5,
                deleted_at = ?6,
                version = ?7
            "#,
        )
        .bind(v.client_id.as_str())
        .bind("")
        .bind(None::<String>)
        .bind("")
        .bind(v.updated_at.unix_timestamp_nanos() as i64)
        .bind(v.deleted_at.unix_timestamp())
        .bind(v.version.inner())
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn delete_workspace_ref_bulk(&self, items: &[RefDelete]) -> Result<()> {
        debug!("Deleting workspaces in bulk in database");
        let mut tx = self.pool.begin().await?;
        for el in items {
            Self::delete_workspace_tx(&mut tx, &el).await?;
        }
        tx.commit().await?;

        Ok(())
    }

    pub async fn list_workspace_deleted(&self) -> Result<Vec<Workspace>> {
        debug!("Query workspaces deleted datbase");
        let res = sqlx::query_as("select * from workspaces where deleted_at not null")
            .fetch(&self.pool)
            .map_ok(|DbWorkspace(ws)| ws)
            .try_collect()
            .await?;

        Ok(res)
    }

    async fn save_conflict_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        v: &Conflict,
    ) -> Result<()> {
        sqlx::query(
            r#"
            insert into conflicts(
                ref_id, ref_kind, data
            ) values(
                ?1, ?2, ?3
            )
            on conflict(ref_id) do update set
                ref_id = ?1,
                ref_kind = ?2,
                data = ?3,
            "#,
        )
        .bind(v.ref_id.to_string())
        .bind(v.ref_kind.to_string())
        .bind(v.data.to_owned())
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn save_conflicts_bulk(&self, items: &[Conflict]) -> Result<()> {
        debug!("Saving conflicts in bulk to database");
        let mut tx = self.pool.begin().await?;
        for el in items {
            Self::save_conflict_tx(&mut tx, &el).await?;
        }
        tx.commit().await?;

        Ok(())
    }

    pub async fn delete_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        v: &RefDelete,
    ) -> Result<()> {
        sqlx::query(
            r#"
            insert into entries(
                id, value, desc, data, kind, path, updated_at, deleted_at, version, workspace_id, host_id
            ) values(
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
            )
            on conflict(id) do update set
                value = ?2,
                desc = ?3,
                data = ?4,
                kind = ?5,
                path = ?6,
                updated_at = ?7,
                deleted_at = ?8,
                version = ?9,
                workspace_id = ?10,
                host_id = ?11
            "#,
        )
        .bind(v.client_id.as_str())
        .bind("")
        .bind("")
        .bind("")
        .bind(EntryKind::Note.to_string())
        .bind("")
        .bind(v.updated_at.unix_timestamp_nanos() as i64)
        .bind(v.deleted_at.unix_timestamp())
        .bind(v.version.inner())
        .bind(None::<String>)
        .bind("x@x")
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn delete_ref_bulk(&self, items: &[RefDelete]) -> Result<()> {
        debug!("Deleting entries in bulk in database");
        let mut tx = self.pool.begin().await?;
        for el in items {
            Self::delete_tx(&mut tx, &el).await?;
        }
        tx.commit().await?;

        Ok(())
    }

    pub async fn delete_bulk(&self, items: &[RefDelete]) -> Result<()> {
        debug!("Deleting entries in bulk in database");
        let mut tx = self.pool.begin().await?;
        for el in items {
            Self::delete_tx(&mut tx, &el).await?;
        }
        tx.commit().await?;

        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("update entries set deleted_at = ?2 where id = ?1")
            .bind(id.to_string())
            .bind(OffsetDateTime::now_utc().unix_timestamp())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn deleted_after(&self, deleted_at: OffsetDateTime) -> Result<Vec<Entry>> {
        debug!("Query deleted before from datbase");
        let res = sqlx::query_as("select * from entries where deleted_at >= ?1")
            .bind(deleted_at.unix_timestamp())
            .fetch(&self.pool)
            .map_ok(|DbEntry(entry)| entry)
            .try_collect()
            .await?;

        Ok(res)
    }

    pub async fn list_deleted(&self) -> Result<Vec<Entry>> {
        debug!("Query entries deleted datbase");
        let res = sqlx::query_as("select * from entries where deleted_at not null")
            .fetch(&self.pool)
            .map_ok(|DbEntry(entry)| entry)
            .try_collect()
            .await?;

        Ok(res)
    }

    pub async fn after(&self, updated_at: OffsetDateTime) -> Result<Vec<Entry>> {
        debug!("Query entries before from datbase");
        let res =
            sqlx::query_as("select * from entries where updated_at >= ?1 and deleted_at is null")
                .bind(updated_at.unix_timestamp_nanos() as i64)
                .fetch(&self.pool)
                .map_ok(|DbEntry(entry)| entry)
                .try_collect()
                .await?;

        Ok(res)
    }

    pub async fn list(
        &self,
        filter: FilterMode,
        context: &Context,
        workspace: Option<&Workspace>,
        search: &str,
    ) -> Result<Vec<Entry>> {
        let mut query = SqlBuilder::select_from("entries");
        query.field("*").order_desc("updated_at");
        query.and_where_is_null("deleted_at");

        match filter {
            FilterMode::All => &mut query,
            FilterMode::Directory => query.and_where_eq("path", quote(&context.path)),
            FilterMode::Workspace => {
                if let Some(workspace) = workspace {
                    query.and_where_eq("workspace_id", quote(workspace.id.to_string()))
                } else {
                    query.and_where_like_left("path", quote(&context.path))
                }
            }
        };

        // for filter in filters {
        //     match filter {
        //         FilterMode::All => &mut query,
        //         FilterMode::Directory => query.and_where_eq("cwd", quote(&context.cwd)),
        //         FilterMode::Workspace => query.and_where_eq(
        //             "cgd",
        //             quote(
        //                 context
        //                     .cgd
        //                     .as_ref()
        //                     .unwrap_or(&"XXXXXXXXXXXXXX".to_string()),
        //             ),
        //         ),
        //     };
        // }

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
        // todo!();
        // for filter in filters {
        //     match filter {
        //         FilterMode::All => &mut query,
        //         FilterMode::Directory => query.and_where_eq("cwd", quote(&context.cwd)),
        //         FilterMode::Workspace => query.and_where_eq(
        //             "cgd",
        //             quote(
        //                 context
        //                     .cgd
        //                     .as_ref()
        //                     .unwrap_or(&"XXXXXXXXXXXXXX".to_string()),
        //             ),
        //         ),
        //     };
        // }

        if !search.is_empty() {
            query.and_where_like_any("value", search);
        }

        let query = query.sql().expect("Failed to parse query");
        let res: (i64,) = sqlx::query_as(&query).fetch_one(&self.pool).await?;

        Ok(res.0)
    }
}
