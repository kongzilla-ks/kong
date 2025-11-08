use std::borrow::Cow;

use candid::{CandidType, Nat};
use ic_stable_structures::{storable::Bound, Storable};
use kong_lib::stable_transfer::tx_id::TxId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct KongRefund {
    pub symbol: String,
    pub amount: Nat,
    pub sent_tx_id: TxId,
}

impl Storable for KongRefund {
    fn to_bytes(&self) -> Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode KongRefund").into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode KongRefund")
    }

    const BOUND: Bound = Bound::Unbounded;
}
