use std::cmp::PartialOrd;
use std::convert::From;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SyncVersion(u32);

impl SyncVersion {
    pub fn new() -> Self {
        Self(1)
    }

    pub fn inner(&self) -> u32 {
        self.0
    }

    pub fn bump(&mut self) {
        self.0 += 1;
    }
}

impl PartialOrd for SyncVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl From<u32> for SyncVersion {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
