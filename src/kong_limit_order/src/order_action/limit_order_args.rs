use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct LimitOrderArgs {
    pub price_str: String,
    pub expired_at_epoch_seconds: Option<u64>,
}
