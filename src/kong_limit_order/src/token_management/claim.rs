use candid::{CandidType, Nat, Principal};
use ic_stable_structures::{storable::Bound, Storable};
use kong_lib::ic::address::Address;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimStatus {
    Unclaimed,
    Claiming,
    Claimed,
    Failed(String),
}

impl ClaimStatus {
    pub fn is_active(&self) -> bool {
        match self {
            ClaimStatus::Unclaimed | ClaimStatus::Claiming => true,
            ClaimStatus::Claimed | ClaimStatus::Failed(_) => false,
        }
    }
}

impl std::fmt::Display for ClaimStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClaimStatus::Unclaimed => write!(f, "Unclaimed"),
            ClaimStatus::Claiming => write!(f, "Claiming"),
            ClaimStatus::Claimed => write!(f, "Success"),
            ClaimStatus::Failed(s) => write!(f, "Failed, reason={s}"),
        }
    }
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub claim_id: u64,
    pub user: Principal,
    pub status: ClaimStatus,
    pub token_symbol: String,
    pub amount: Nat,
    pub to_address: Option<Address>, // optional, will default to caller's principal id
    pub transfer_ids: Vec<u64>,
    pub create_ts: u64,
    pub update_ts: u64,
    pub failed_attempts: u32,
}

impl Claim {
    pub fn new(user: Principal, token_symbol: String, amount: &Nat, to_address: Option<Address>) -> Self {
        let now = ic_cdk::api::time();
        Self {
            claim_id: 0, // will be set with insert_claim into CLAIM_MAP
            user,
            status: ClaimStatus::Unclaimed,
            token_symbol,
            amount: amount.clone(),
            to_address,
            transfer_ids: Vec::new(),
            create_ts: now,
            update_ts: now,
            failed_attempts: 0,
        }
    }
}

impl Storable for Claim {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(self).unwrap().into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).unwrap()
    }

    const BOUND: Bound = Bound::Unbounded;
}
