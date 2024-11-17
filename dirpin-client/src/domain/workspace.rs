use crate::domain::context::Context;
use crate::domain::host::HostId;
use dirpin_common::domain::SyncVersion;
use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceId(Uuid);

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

impl FromStr for WorkspaceId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

#[derive(Debug, Clone)]
pub struct WorkspacePath {
    host_id: HostId,
    path: String,
}

impl WorkspacePath {
    pub fn new(host_id: HostId, path: String) -> Self {
        Self { host_id, path }
    }
}

impl TryFrom<&str> for WorkspacePath {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (host, path) = value
            .split_once(':')
            .ok_or("Incorrect workspace path format")?;

        let host_id = HostId::from_str(host).map_err(|_| "Incorrect host format".to_string())?;

        Ok(Self {
            host_id,
            path: path.into(),
        })
    }
}

impl std::fmt::Display for WorkspacePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host_id, self.path)
    }
}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub git: Option<String>,
    pub paths: Vec<WorkspacePath>,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
    pub version: SyncVersion,
}

impl Workspace {
    pub fn new(name: String, context: &Context) -> Self {
        // TODO: get the name based on global context and only take out the name of the directory
        Self {
            id: WorkspaceId(Uuid::now_v7()),
            name,
            git: context.git.clone(),
            paths: vec![WorkspacePath::new(
                context.host_id.clone(),
                context.path.clone(),
            )],
            updated_at: OffsetDateTime::now_utc(),
            deleted_at: None,
            version: SyncVersion::new(),
        }
    }
}
