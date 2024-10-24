#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub version: String,
}
