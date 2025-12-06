use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct QueryOrdersArgs {
    pub receive_token: String,
    pub send_token: String,
    pub start_ts: Option<u64>,
    pub limit: Option<usize>,
}
