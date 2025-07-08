use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

use crate::helpers::nat_helpers::nat_zero;
use crate::stable_pool::pool_map;
use crate::stable_token::token::Token;
use crate::stable_tx::add_pool_tx::AddPoolTx;
use crate::stable_tx::status_tx::StatusTx;
use crate::transfers::transfer_reply::TransferIdReply;
use crate::transfers::transfer_reply_helpers::to_transfer_ids;

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct AddPoolReply {
    pub tx_id: u64,
    #[serde(default = "zero_u32")]
    pub pool_id: u32,
    pub request_id: u64,
    pub status: String,
    #[serde(default = "empty_string")]
    pub name: String,
    pub symbol: String,
    pub chain_0: String,
    #[serde(default = "empty_string")]
    pub address_0: String,
    pub symbol_0: String,
    pub amount_0: Nat,
    pub balance_0: Nat,
    pub chain_1: String,
    #[serde(default = "empty_string")]
    pub address_1: String,
    pub symbol_1: String,
    pub amount_1: Nat,
    pub balance_1: Nat,
    pub lp_fee_bps: u8,
    pub lp_token_symbol: String,
    pub add_lp_token_amount: Nat,
    pub transfer_ids: Vec<TransferIdReply>,
    pub claim_ids: Vec<u64>,
    #[serde(default = "false_bool")]
    pub is_removed: bool,
    pub ts: u64,
}

fn zero_u32() -> u32 {
    0
}

fn empty_string() -> String {
    String::new()
}

fn false_bool() -> bool {
    false
}

impl From<&AddPoolTx> for AddPoolReply {
    fn from(add_pool_tx: &AddPoolTx) -> Self {
        let (name, symbol, chain_0, address_0, symbol_0, balance_0, chain_1, address_1, symbol_1, balance_1, lp_fee_bps, lp_token_symbol) =
            pool_map::get_by_pool_id(add_pool_tx.pool_id).map_or_else(
                || {
                    (
                        "Pool name not added".to_string(),
                        "Pool symbol not added".to_string(),
                        "Pool chain_0 not added".to_string(),
                        "Pool address_0 not added".to_string(),
                        "Pool symbol_0 not added".to_string(),
                        nat_zero(),
                        "Pool chain_1 not added".to_string(),
                        "Pool address_1 not added".to_string(),
                        "Pool symbol_1 not added".to_string(),
                        nat_zero(),
                        0,
                        "LP token not added".to_string(),
                    )
                },
                |pool| {
                    (
                        pool.name(),
                        pool.symbol(),
                        pool.chain_0(),
                        pool.address_0(),
                        pool.symbol_0(),
                        pool.balance_0.clone(),
                        pool.chain_1(),
                        pool.address_1(),
                        pool.symbol_1(),
                        pool.balance_1.clone(),
                        pool.lp_fee_bps,
                        pool.lp_token().symbol().to_string(),
                    )
                },
            );

        AddPoolReply {
            tx_id: add_pool_tx.tx_id,
            pool_id: add_pool_tx.pool_id,
            request_id: add_pool_tx.request_id,
            status: add_pool_tx.status.to_string(),
            name,
            symbol,
            chain_0,
            address_0,
            symbol_0,
            amount_0: add_pool_tx.amount_0.clone(),
            balance_0,
            chain_1,
            address_1,
            symbol_1,
            amount_1: add_pool_tx.amount_1.clone(),
            balance_1,
            lp_fee_bps,
            lp_token_symbol,
            add_lp_token_amount: add_pool_tx.add_lp_token_amount.clone(),
            transfer_ids: to_transfer_ids(&add_pool_tx.transfer_ids),
            claim_ids: add_pool_tx.claim_ids.clone(),
            is_removed: add_pool_tx.is_removed,
            ts: add_pool_tx.ts,
        }
    }
}

impl AddPoolReply {
    #[allow(clippy::too_many_arguments)]
    pub fn failed(
        request_id: u64,
        chain_0: &str,
        address_0: &str,
        symbol_0: &str,
        chain_1: &str,
        address_1: &str,
        symbol_1: &str,
        transfer_ids: &[u64],
        claim_ids: &[u64],
        ts: u64,
    ) -> Self {
        AddPoolReply {
            tx_id: 0,
            pool_id: 0,
            request_id,
            status: StatusTx::Failed.to_string(),
            name: "Pool not added".to_string(),
            symbol: "Pool not added".to_string(),
            chain_0: chain_0.to_string(),
            address_0: address_0.to_string(),
            symbol_0: symbol_0.to_string(),
            amount_0: nat_zero(),
            balance_0: nat_zero(),
            chain_1: chain_1.to_string(),
            address_1: address_1.to_string(),
            symbol_1: symbol_1.to_string(),
            amount_1: nat_zero(),
            balance_1: nat_zero(),
            lp_fee_bps: 0,
            lp_token_symbol: "LP token not added".to_string(),
            add_lp_token_amount: nat_zero(),
            transfer_ids: to_transfer_ids(transfer_ids),
            claim_ids: claim_ids.to_vec(),
            is_removed: false,
            ts,
        }
    }
}
