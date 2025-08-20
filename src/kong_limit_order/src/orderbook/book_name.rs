use candid::CandidType;
use ic_stable_structures::{storable::Bound, Storable};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BookName(String, String);

impl BookName {
    pub fn new(symbol_0: &str, symbol_1: &str) -> Self {
        BookName(symbol_0.to_string(), symbol_1.to_string())
    }

    pub fn new_from_raw_str(pair: &String) -> Result<Self, String> {
        let mut it = pair.split('/');
        let val1 = it.next().ok_or("Invalid book name".to_string())?;
        let val2 = it.next().ok_or("Invalid book name. Missing '/'".to_string())?;
        match it.next() {
            Some(_) => Err("Invalid book name. Multiple '/'".to_string()),
            None => Ok(Self::new(val1, val2)),
        }
    }

    pub fn reversed(&self) -> Self {
        Self::new(self.symbol_1(), self.symbol_0())
    }

    pub fn symbol_0(&self) -> &str {
        &self.0
    }

    pub fn symbol_1(&self) -> &str {
        &self.1
    }

    pub fn symbols(&self) -> (&str, &str) {
        (self.0.as_str(), self.1.as_str())
    }
}


impl std::fmt::Display for BookName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}/{}", self.0, self.1)
    }
}

impl Storable for BookName {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode BookName").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode BookName")
    }

    const BOUND: Bound = Bound::Unbounded;
}
