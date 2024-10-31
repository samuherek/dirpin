use time::OffsetDateTime;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
pub struct SyncResponse {
    pub updated: Vec<String>,
    pub deleted: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AddPinRequest {
    pub id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
    pub version: u32,
    pub data: String,
}
