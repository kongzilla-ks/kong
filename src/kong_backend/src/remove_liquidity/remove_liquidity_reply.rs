use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

use crate::stable_pool::pool_map;
use crate::stable_tx::remove_liquidity_tx::RemoveLiquidityTx;
use crate::transfers::transfer_reply::TransferIdReply;
use crate::stable_transfer::transfer_map;
use crate::stable_token::token_map;

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct RemoveLiquidityReply {
    pub tx_id: u64,
    pub request_id: u64,
    pub status: String,
    pub symbol: String,
    pub chain_0: String,
    #[serde(default = "empty_string")]
    pub address_0: String,
    pub symbol_0: String,
    pub amount_0: Nat,
    pub lp_fee_0: Nat,
    pub chain_1: String,
    #[serde(default = "empty_string")]
    pub address_1: String,
    pub symbol_1: String,
    pub amount_1: Nat,
    pub lp_fee_1: Nat,
    pub remove_lp_token_amount: Nat,
    pub transfer_ids: Vec<TransferIdReply>,
    pub claim_ids: Vec<u64>,
    pub ts: u64,
}

fn empty_string() -> String {
    String::new()
}

impl TryFrom<&RemoveLiquidityTx> for RemoveLiquidityReply {
    type Error = String;
    
    fn try_from(remove_liquidity_tx: &RemoveLiquidityTx) -> Result<Self, Self::Error> {
        let pool = pool_map::get_by_pool_id(remove_liquidity_tx.pool_id)
            .ok_or_else(|| "Pool not found".to_string())?;
        
        Ok(RemoveLiquidityReply {
            tx_id: remove_liquidity_tx.tx_id,
            request_id: remove_liquidity_tx.request_id,
            status: remove_liquidity_tx.status.to_string(),
            symbol: pool.symbol(),
            chain_0: pool.chain_0(),
            address_0: pool.address_0(),
            symbol_0: pool.symbol_0(),
            amount_0: remove_liquidity_tx.amount_0.clone(),
            lp_fee_0: remove_liquidity_tx.lp_fee_0.clone(),
            chain_1: pool.chain_1(),
            address_1: pool.address_1(),
            symbol_1: pool.symbol_1(),
            amount_1: remove_liquidity_tx.amount_1.clone(),
            lp_fee_1: remove_liquidity_tx.lp_fee_1.clone(),
            remove_lp_token_amount: remove_liquidity_tx.remove_lp_token_amount.clone(),
            transfer_ids: remove_liquidity_tx.transfer_ids.iter().filter_map(|&transfer_id| {
                let transfer = transfer_map::get_by_transfer_id(transfer_id)?;
                let token = token_map::get_by_token_id(transfer.token_id)?;
                TransferIdReply::try_from((transfer_id, &transfer, &token)).ok()
            }).collect(),
            claim_ids: remove_liquidity_tx.claim_ids.clone(),
            ts: remove_liquidity_tx.ts,
        })
    }
}
