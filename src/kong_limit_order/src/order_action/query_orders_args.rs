use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct QueryOrdersArgs {
    pub token_0: String,
    pub token_1: String,
    pub start_ts: Option<u64>,
    pub limit: Option<usize>,
}
