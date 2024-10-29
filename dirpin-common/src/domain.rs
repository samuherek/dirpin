use std::str::{FromStr};
use std::string::ToString;
use uuid::Uuid;

pub struct HostId(Uuid);

impl HostId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl FromStr for HostId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id = Uuid::from_str(s)?;
        Ok(Self(id))
    }
}

impl ToString for HostId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}
