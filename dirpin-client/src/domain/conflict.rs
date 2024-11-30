use crate::domain::entry::Entry;
use crate::domain::workspace::Workspace;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug)]
pub enum ConflictKind {
    Entry,
    Workspace,
}

impl ConflictKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::Workspace => "workspace",
        }
    }
}

impl FromStr for ConflictKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "entry" => Ok(Self::Entry),
            "workspace" => Ok(Self::Workspace),
            _ => Err("Failed to parse ConflictRef from string".into()),
        }
    }
}

impl std::fmt::Display for ConflictKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub trait HasId {
    fn id(&self) -> &Uuid;
}

#[derive(Debug)]
pub enum Conflict {
    Entry(Entry),
    Workspace(Workspace),
}

impl Conflict {
    pub fn id(&self) -> String {
        match self {
            Conflict::Entry(v) => v.id.to_string(),
            Conflict::Workspace(v) => v.id.to_string(),
        }
    }

    pub fn kind(&self) -> &str {
        match self {
            Conflict::Entry(_) => "entry",
            Conflict::Workspace(_) => "workspace",
        }
    }

    pub fn data(&self) -> eyre::Result<String> {
        match self {
            Conflict::Entry(v) => Ok(serde_json::to_string(&v)?),
            Conflict::Workspace(v) => Ok(serde_json::to_string(&v)?),
        }
    }
}
