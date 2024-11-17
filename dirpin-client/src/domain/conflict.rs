use crate::domain::entry::Entry;
use std::str::FromStr;
use uuid::Uuid;

pub enum ConflictRef {
    Entry,
    Workspace,
}

impl ConflictRef {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::Workspace => "workspace",
        }
    }
}

impl FromStr for ConflictRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "entry" => Ok(Self::Entry),
            "Workspace" => Ok(Self::Workspace),
            _ => Err("Failed to parse ConflictRef from string".into()),
        }
    }
}

impl std::fmt::Display for ConflictRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub struct Conflict {
    pub ref_id: Uuid,
    pub ref_kind: ConflictRef,
    /// TODO: probably not a good idea to have it as a string
    pub data: String,
}

impl TryFrom<&Entry> for Conflict {
    type Error = serde_json::Error;

    fn try_from(value: &Entry) -> Result<Self, Self::Error> {
        let data = serde_json::to_string(value)?;
        Ok(Self {
            ref_id: value.id,
            ref_kind: ConflictRef::Entry,
            data,
        })
    }
}
