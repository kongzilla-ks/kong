use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct ClaimsReply {
    pub claim_id: u64,
    pub status: String,
    pub chain: String,
    pub symbol: String,
    pub canister_id: Option<String>,
    pub amount: Nat,
    pub fee: Nat,
    pub to_address: String,
    pub desc: String,
    pub ts: u64,
}
