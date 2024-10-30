use time::OffsetDateTime;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SyncRequest {
    #[serde(with = "time::serde::rfc3339")]
    pub from: OffsetDateTime,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SyncResponse {
    pub ok: String,
}
