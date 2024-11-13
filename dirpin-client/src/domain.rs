use crate::utils;
use dirpin_common::api::Deleted;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use std::string::ToString;
use time::OffsetDateTime;
use uuid::{self, Uuid};

/// Readable host identifier that is reasonably unique for the current user host group.
/// This is mainly used for remote session identifiers or anything else that needs to be
/// stored on the remote server for syncing when identifying different user devices.
pub struct HostId(String);

impl HostId {
    const HOST_WORDS: [&'static str; 16] = [
        "apple", "sun", "star", "cloud", "tree", "river", "moon", "stone", "fire", "ice", "bird",
        "mountain", "ocean", "wind", "rain", "sand",
    ];

    /// Generate a current host id that deterministically depend on the hostname and username
    /// of this computer. Used to readably identify the host of the current remote user.
    ///
    /// TODO: Check if this is unique enough or it can have colisions when we have more than
    /// xxx hosts per remote user.
    pub fn gen_host_id() -> Self {
        let user_host = utils::get_host_user();
        let mut hasher = Sha256::new();
        hasher.update(user_host);
        let hash = hasher.finalize();

        let seed: u64 = u64::from_be_bytes(hash[..8].try_into().unwrap());
        let mut rng = StdRng::seed_from_u64(seed);

        let first = Self::HOST_WORDS[rng.gen_range(0..Self::HOST_WORDS.len())];
        let second = Self::HOST_WORDS[rng.gen_range(0..Self::HOST_WORDS.len())];
        let num = rng.gen_range(0..100);

        let inner = format!("{first}-{second}-{num}").into();
        Self(inner)
    }
}

impl FromStr for HostId {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split('-').collect::<Vec<_>>();

        if parts.len() != 3 {
            return Err("Input should have exactly three parts separated by '-'");
        }
        if !parts[0].chars().all(char::is_alphabetic) {
            return Err("First split needs to contain only letters");
        }
        if !parts[1].chars().all(char::is_alphabetic) {
            return Err("Second split needs to contain only letters");
        }
        if !parts[2].chars().all(char::is_numeric) {
            return Err("Third split needs to contain only numbers");
        }

        Ok(Self(s.into()))
    }
}

impl AsRef<str> for HostId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for HostId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
/// TODO: Add the ability to have custom types
pub enum EntryKind {
    Note,
    Cmd,
    Todo,
}

impl FromStr for EntryKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "note" => Ok(Self::Note),
            "cmd" => Ok(Self::Cmd),
            "todo" => Ok(Self::Todo),
            _ => Ok(Self::Note),
        }
    }
}

impl EntryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntryKind::Note => "note",
            EntryKind::Cmd => "cmd",
            EntryKind::Todo => "todo",
        }
    }
}

impl std::fmt::Display for EntryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: Uuid,
    pub value: String,
    pub data: Option<String>,
    pub kind: EntryKind,
    pub hostname: String,
    pub cwd: String,
    pub cgd: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
    pub version: u32,
}

impl Entry {
    pub fn new(value: String, hostname: String, cwd: String, cgd: Option<String>) -> Self {
        let id = Uuid::now_v7();
        let created_at = OffsetDateTime::now_utc();
        let updated_at = created_at.clone();
        Self {
            id,
            value,
            data: None,
            kind: EntryKind::Note,
            hostname,
            cwd,
            cgd,
            created_at,
            updated_at,
            deleted_at: None,
            version: 1,
        }
    }

    pub fn kind(mut self, kind: EntryKind) -> Self {
        self.kind = kind;
        self
    }
}

#[derive(Debug, Clone)]
/// Used as a version
pub struct EntryDelete {
    pub id: Uuid,
    pub updated_at: OffsetDateTime,
    pub deleted_at: OffsetDateTime,
    pub version: u32,
}
impl TryFrom<Deleted> for EntryDelete {
    type Error = uuid::Error;

    fn try_from(value: Deleted) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&value.client_id)?,
            updated_at: value.updated_at,
            deleted_at: value.deleted_at,
            version: value.version,
        })
    }
}
