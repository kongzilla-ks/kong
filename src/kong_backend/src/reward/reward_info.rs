use crate::stable_token::stable_token::StableTokenId;
use crate::reward::reward_rules::RewardRules;
use candid::{CandidType, Nat};
use ic_stable_structures::{storable::Bound, Storable};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RewardInfoId(pub u32);

impl Storable for RewardInfoId {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode RewardInfoId").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode RewardInfoId")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct RewardInfo {
    pub id: RewardInfoId,

    pub reward_amount: Nat,
    pub reward_token_id: StableTokenId,

    pub created_ts: u64,
    pub updated_ts: u64,
    pub updated_user: String,

    pub is_active: bool,

    pub reward_rules: RewardRules,
}

impl Storable for RewardInfo {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode StableLPToken").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode StableLPToken")
    }

    const BOUND: Bound = Bound::Unbounded;
}
