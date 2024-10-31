use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Pin {
    pub id: Uuid,
    pub data: String,
    pub hostname: String,
    pub cwd: String,
    pub cgd: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
    pub version: u32,
}

impl Pin {
    pub fn new(data: String, hostname: String, cwd: String, cgd: Option<String>) -> Self {
        let id = Uuid::now_v7();
        let created_at = OffsetDateTime::now_utc();
        let updated_at = created_at.clone();
        Self {
            id,
            data,
            hostname,
            cwd,
            cgd,
            created_at,
            updated_at,
            deleted_at: None,
            version: 1,
        }
    }
}
