use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

use super::status_tx::StatusTx;

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct AddLiquidityTx {
    pub tx_id: u64,
    pub pool_id: u32,
    pub user_id: u32,    // store here as well, otherwise need to get via request_id
    pub request_id: u64, // get_by_request_id are expensive calls
    pub status: StatusTx,
    pub amount_0: Nat,
    pub amount_1: Nat,
    pub add_lp_token_amount: Nat,
    pub transfer_ids: Vec<u64>,
    pub claim_ids: Vec<u64>,
    pub ts: u64,
}
