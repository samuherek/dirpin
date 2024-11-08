use time::OffsetDateTime;

#[derive(Debug)]
pub struct NewPin {
    pub client_id: String,
    pub user_id: i64,
    pub timestamp: OffsetDateTime,
    pub version: u32,
    pub data: String,
}

#[derive(Debug)]
pub struct Pin {
    pub id: u32,
    pub client_id: String,
    pub user_id: i64,
    pub timestamp: OffsetDateTime,
    pub version: u32,
    pub data: String,
}

#[derive(Debug)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug)]
pub struct User {
    pub id: u32,
    pub username: String,
    pub email: String,
    pub password: String,
    pub verified_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug)]
pub struct NewSession {
    pub user_id: u32,
    pub token: String,
}
