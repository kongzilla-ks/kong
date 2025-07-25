use candid::CandidType;
use ic_stable_structures::{storable::Bound, Storable};
use serde::{Deserialize, Serialize};

use crate::ic::network::ICNetwork;

// reserved user ids
// 0: all users - users for stable_messages to broadcast to all users
// 1: system - system user
// 2: claims timer - user id to identify claim was made by system timer
// 3-99: reserved for future use
// 100-: user ids
pub const ANONYMOUS_USER_ID: u32 = 0;
#[allow(dead_code)]
pub const ALL_USERS_USER_ID: u32 = 1;
#[allow(dead_code)]
pub const SYSTEM_USER_ID: u32 = 2;
pub const CLAIMS_TIMER_USER_ID: u32 = 3;

#[derive(CandidType, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StableUserId(pub u32);

impl Storable for StableUserId {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode StableUserId").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode StableUserId")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct StableUser {
    pub user_id: u32,
    pub principal_id: String,
    pub my_referral_code: String,
    pub referred_by: Option<u32>, // user_id of the user who referred this user
    pub referred_by_expires_at: Option<u64>,
    // fee level for the user. user's pays lp_fee = 100 - fee_level / 100
    // so 0 = no discount, 100 = pays no lp_fee on swaps
    pub fee_level: u8,
    pub fee_level_expires_at: Option<u64>,
}

impl Default for StableUser {
    fn default() -> Self {
        StableUser {
            user_id: ANONYMOUS_USER_ID,
            principal_id: ICNetwork::caller().to_text(),
            my_referral_code: "".to_string(),
            referred_by: None,
            referred_by_expires_at: None,
            fee_level: 0,
            fee_level_expires_at: None,
        }
    }
}

impl Storable for StableUser {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode StableUser").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode StableUser")
    }

    const BOUND: Bound = Bound::Unbounded;
}
