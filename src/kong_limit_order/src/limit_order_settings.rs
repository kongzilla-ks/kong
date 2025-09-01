use candid::CandidType;
use ic_stable_structures::{storable::Bound, Storable};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct LimitOrderSettings {
    pub kong_backend: String,
    pub max_orders_per_instrument: usize,
    pub synthetic_orderbook_max_hops: usize,
}

// TODO: how to properly pass kong backend?
pub const CANISTER_ID_KONG_BACKEND: &str = "2ipq2-uqaaa-aaaar-qailq-cai";

impl Default for LimitOrderSettings {
    fn default() -> Self {
        LimitOrderSettings {
            kong_backend: CANISTER_ID_KONG_BACKEND.to_string(),
            max_orders_per_instrument: 10,
            synthetic_orderbook_max_hops: 3,
        }
    }
}

impl Storable for LimitOrderSettings {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode LimitOrderSettings").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).unwrap_or_default()
    }

    const BOUND: Bound = Bound::Unbounded;
}
