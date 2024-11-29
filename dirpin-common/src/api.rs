use crate::domain::SyncVersion;
use time::OffsetDateTime;

#[derive(Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct HealthCheckResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SyncRequest {
    #[serde(with = "time::serde::rfc3339")]
    pub last_sync_ts: OffsetDateTime,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AddSyncRequest {
    pub items: Vec<AddEntryRequest>,
    #[serde(with = "time::serde::rfc3339")]
    pub last_sync_ts: OffsetDateTime,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
/// The entry reference coming from the remote
pub struct RefDelete {
    /// Host: id of the entry
    pub client_id: String,
    /// Host: version of the entry
    pub version: SyncVersion,
    /// Host: updated_at of the entry
    pub updated_at: OffsetDateTime,
    /// Host: deleted_at of the entry
    pub deleted_at: OffsetDateTime,
    /// Differnet entity kind. Now one of entry/workspace
    pub kind: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RefItem {
    pub data: String,
    pub kind: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SyncResponse {
    /// These are all with deleted_at field None
    pub updated: Vec<RefItem>,
    /// These are all with delted_at field Some(_)
    pub deleted: Vec<RefDelete>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StatusResponse {
    /// The username of the currently signed in user
    pub username: String,
    /// The server version of the library
    pub version: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AddEntryRequest {
    pub id: String,
    pub version: u32,
    pub data: String,
    /// Differnet entity kind. Now one of entry/workspace
    pub kind: String,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub host_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RegisterResponse {
    pub session: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LogoutResponse {
    pub ok: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub host_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LoginResponse {
    pub session: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ErrorMessage {
    pub value: String,
}
