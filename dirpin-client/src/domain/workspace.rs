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

#[derive(Debug)]
pub struct WorkspacePath {
    host_id: HostId,
    path: String,
}

#[derive(Debug)]
pub struct Workspace {
    id: WorkspaceId,
    name: String,
    git: String,
    paths: Vec<WorkspacePath>,
    updated_at: OffsetDateTime,
    deleted_at: Option<OffsetDateTime>,
    version: SyncVersion,
}
