use candid::{CandidType, Deserialize};
use ic_stable_structures::{storable::Bound, Storable};
use serde::Serialize;
use std::borrow::Cow;

/// Key for transaction notification storage
#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TransactionNotificationId(pub String); // signature

impl Storable for TransactionNotificationId {
    fn to_bytes(&self) -> Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode TransactionNotificationId").into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode TransactionNotificationId")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct TransactionNotification {
    pub signature: String,
    pub status: String,           // e.g., "processed", "confirmed", "finalized", "failed"
    pub metadata: Option<String>, // Store full RPC response or parsed details
    pub timestamp: u64,
}

impl Storable for TransactionNotification {
    fn to_bytes(&self) -> Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode TransactionNotification").into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode TransactionNotification")
    }

    const BOUND: Bound = Bound::Unbounded;
}
