use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::chains::chains::LP_CHAIN;
use crate::stable_pool::pool_map;
use crate::stable_pool::stable_pool::StablePool;

pub const LP_DECIMALS: u8 = 8; // LP token decimal

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct LPToken {
    pub token_id: u32,
    pub address: String, // unique identifier for the token
    pub symbol: String,
    pub decimals: u8,
    #[serde(default = "false_bool")]
    pub is_removed: bool,
}

fn false_bool() -> bool {
    false
}

impl LPToken {
    pub fn name(&self) -> String {
        format!("{} LP Token", self.symbol)
    }

    pub fn chain(&self) -> String {
        LP_CHAIN.to_string()
    }

    /// Pool that the LP token belongs to
    pub fn pool_of(&self) -> Option<StablePool> {
        pool_map::get_by_lp_token_id(self.token_id)
    }
}
