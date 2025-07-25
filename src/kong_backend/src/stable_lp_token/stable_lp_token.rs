use candid::{CandidType, Nat};
use ic_stable_structures::{storable::Bound, Storable};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StableLPTokenId(pub u64);

impl Storable for StableLPTokenId {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode StableLPTokenId").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode StableLPTokenId")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct StableLPToken {
    pub lp_token_id: u64, // unique id (same as StableLPTokenLedgerId) for LP_TOKEN_LEDGER
    pub user_id: u32,     // user id of the token holder
    pub token_id: u32,    // token id of the token
    pub amount: Nat,      // amount the user holds of the token
    pub ts: u64,          // timestamp of the last token update
}

impl StableLPToken {
    pub fn new(user_id: u32, token_id: u32, amount: Nat, ts: u64) -> Self {
        Self {
            lp_token_id: 0,
            user_id,
            token_id,
            amount,
            ts,
        }
    }
}

impl Storable for StableLPToken {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode StableLPToken").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode StableLPToken")
    }

    const BOUND: Bound = Bound::Unbounded;
}
