use candid::CandidType;
use ic_stable_structures::{storable::Bound, Storable};
use kong_lib::ic::canister_address::KONG_BACKEND;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct LimitOrderSettings {
    pub kong_backend: String,
    // pub limit_backend: String, // use canister_self instead
    pub max_orders_per_instrument: usize,
    pub synthetic_orderbook_max_hops: usize,
    pub next_claim_id: u64,
    pub twap_default_seconds_delay_after_failure: u64,
    pub next_kong_refund_id: u64,
    // TODO: add maintenance mode
}

impl Default for LimitOrderSettings {
    fn default() -> Self {
        LimitOrderSettings {
            kong_backend: KONG_BACKEND.to_string(),
            // limit_backend: KONG_LIMIT.to_string(),
            max_orders_per_instrument: 10,
            synthetic_orderbook_max_hops: 3,
            next_claim_id: 1,
            twap_default_seconds_delay_after_failure: 10,
            next_kong_refund_id: 1,
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
