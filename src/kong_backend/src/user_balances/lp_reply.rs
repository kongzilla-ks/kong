use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct LPReply {
    pub symbol: String,
    pub name: String,
    pub pool_id: u32,
    pub lp_token_id: u64,
    pub balance: f64,
    pub balance_nat: Nat,
    pub usd_balance: f64,
    pub usd_balance_nat: Nat,
    pub chain_0: String,
    pub symbol_0: String,
    pub address_0: String,
    pub amount_0: f64,
    pub amount_0_nat: Nat,
    pub usd_amount_0: f64,
    pub usd_amount_0_nat: Nat,
    pub chain_1: String,
    pub symbol_1: String,
    pub address_1: String,
    pub amount_1: f64,
    pub amount_1_nat: Nat,
    pub usd_amount_1: f64,
    pub usd_amount_1_nat: Nat,
    pub ts: u64,
}
