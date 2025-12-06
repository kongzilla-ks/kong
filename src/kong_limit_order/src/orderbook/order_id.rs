use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, CandidType, Serialize, Deserialize)]
pub struct OrderId(pub u64);


impl From<u64> for OrderId {
    fn from(value: u64) -> Self {
        OrderId(value)
    }
}

impl Into<u64> for OrderId {
    fn into(self) -> u64 {
        self.0
    }
}

impl OrderId {
    pub fn next(&mut self) {
        self.0 += 1;
    }
}

impl std::fmt::Display for OrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}