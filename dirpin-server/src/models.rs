use time::OffsetDateTime;

#[derive(Debug)]
pub struct NewPin {
    pub client_id: String,
    pub user_id: i64,
    pub timestamp: OffsetDateTime,
    pub version: u32,
    pub data: String,
}
