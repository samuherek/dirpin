use std::str::FromStr;
use time::OffsetDateTime;

#[derive(Debug)]
/// The entry update coming from the Host.
pub struct NewEntry {
    pub client_id: String,
    pub user_id: i64,
    pub version: u32,
    pub data: String,
    pub kind: String,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

#[derive(Debug)]
pub enum RemoteKind {
    Entry,
    Workspace,
}

impl std::fmt::Display for RemoteKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            RemoteKind::Entry => "entry",
            RemoteKind::Workspace => "workspace",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for RemoteKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "entry" => Ok(Self::Entry),
            "workspace" => Ok(Self::Workspace),
            _ => Err("Failed to parse RemoteKind".to_string()),
        }
    }
}

#[derive(Debug)]
/// Remote entry in the remote server
pub struct Entry {
    /// Remote: db id
    pub id: u32,
    /// Host: The id for the entry on the client side
    pub client_id: String,
    /// Remote: user id
    pub user_id: i64,
    /// Host: Version of the entry to conflict detect uploads
    pub version: u32,
    /// Host: The encrypted data of the entry
    pub data: String,
    /// Host: unencrypted type of entry -> entry/workspace
    /// TODO: Think wehether we need to actually encrypt this or it's ok.
    pub kind: String,
    /// Remote: synced_at timestamp
    pub synced_at: OffsetDateTime,
    /// Host: updated_at of the entry to conflict detect uploads
    pub updated_at: OffsetDateTime,
    /// Host: deleted_at of the entry to conflict detect uploads
    pub deleted_at: Option<OffsetDateTime>,
}

#[derive(Debug)]
/// New user submittion coming from the host
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug)]
/// Remote user in the db
pub struct User {
    pub id: u32,
    pub username: String,
    pub email: String,
    pub password: String,
    pub verified_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug)]
/// Remote session
pub struct NewSession {
    pub user_id: u32,
    pub host_id: String,
    pub token: String,
    pub expires_at: OffsetDateTime,
}

#[derive(Debug)]
pub struct HostSession {
    pub user_id: u32,
    pub host_id: String,
}

#[derive(Debug)]
pub struct RenewSession {
    pub id: u32,
    pub token: String,
    pub expires_at: OffsetDateTime,
}

#[derive(Debug)]
pub struct Session {
    pub id: u32,
    pub user_id: u32,
    pub token: String,
    pub host_id: Option<String>,
    pub expires_at: OffsetDateTime,
}
