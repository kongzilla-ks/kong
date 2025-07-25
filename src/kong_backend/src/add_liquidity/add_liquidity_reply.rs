use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

use crate::helpers::nat_helpers::nat_zero;
use crate::stable_pool::pool_map;
use crate::stable_tx::add_liquidity_tx::AddLiquidityTx;
use crate::stable_tx::status_tx::StatusTx;
use crate::stable_transfer::transfer_map;
use crate::stable_token::token_map;
use crate::transfers::transfer_reply::TransferIdReply;

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct AddLiquidityReply {
    pub tx_id: u64,
    pub request_id: u64,
    pub status: String,
    pub symbol: String,
    pub chain_0: String,
        pub address_0: String,
    pub symbol_0: String,
    pub amount_0: Nat,
    pub chain_1: String,
        pub address_1: String,
    pub symbol_1: String,
    pub amount_1: Nat,
    pub add_lp_token_amount: Nat,
    pub transfer_ids: Vec<TransferIdReply>,
    pub claim_ids: Vec<u64>,
    pub ts: u64,
}

impl TryFrom<&AddLiquidityTx> for AddLiquidityReply {
    type Error = String;
    
    fn try_from(add_liquidity_tx: &AddLiquidityTx) -> Result<Self, Self::Error> {
        let pool = pool_map::get_by_pool_id(add_liquidity_tx.pool_id)
            .ok_or_else(|| "Pool not found".to_string())?;
        
        Ok(AddLiquidityReply {
            tx_id: add_liquidity_tx.tx_id,
            request_id: add_liquidity_tx.request_id,
            status: add_liquidity_tx.status.to_string(),
            symbol: pool.symbol(),
            chain_0: pool.chain_0(),
            address_0: pool.address_0(),
            symbol_0: pool.symbol_0(),
            amount_0: add_liquidity_tx.amount_0.clone(),
            chain_1: pool.chain_1(),
            address_1: pool.address_1(),
            symbol_1: pool.symbol_1(),
            amount_1: add_liquidity_tx.amount_1.clone(),
            add_lp_token_amount: add_liquidity_tx.add_lp_token_amount.clone(),
            transfer_ids: add_liquidity_tx.transfer_ids.iter().filter_map(|&transfer_id| {
                let transfer = transfer_map::get_by_transfer_id(transfer_id)?;
                let token = token_map::get_by_token_id(transfer.token_id)?;
                TransferIdReply::try_from((transfer_id, &transfer, &token)).ok()
            }).collect(),
            claim_ids: add_liquidity_tx.claim_ids.clone(),
            ts: add_liquidity_tx.ts,
        })
    }
}

impl AddLiquidityReply {
    pub fn failed(pool_id: u32, request_id: u64, transfer_ids: &[u64], claim_ids: &[u64], ts: u64) -> Self {
        let pool = pool_map::get_by_pool_id(pool_id);
        let (symbol, chain_0, address_0, symbol_0, chain_1, address_1, symbol_1) = pool.map_or_else(
            || (
                "Pool symbol not found".to_string(),
                "Pool chain_0 not found".to_string(),
                "Pool address_0 not found".to_string(),
                "Pool symbol_0 not found".to_string(),
                "Pool chain_1 not found".to_string(),
                "Pool address_1 not found".to_string(),
                "Pool symbol_1 not found".to_string(),
            ),
            |pool| (
                pool.symbol(),
                pool.chain_0(),
                pool.address_0(),
                pool.symbol_0(),
                pool.chain_1(),
                pool.address_1(),
                pool.symbol_1(),
            ),
        );
        
        AddLiquidityReply {
            tx_id: 0,
            request_id,
            status: StatusTx::Failed.to_string(),
            symbol,
            chain_0,
            address_0,
            symbol_0,
            amount_0: nat_zero(),
            chain_1,
            address_1,
            symbol_1,
            amount_1: nat_zero(),
            add_lp_token_amount: nat_zero(),
            transfer_ids: transfer_ids.iter().filter_map(|&transfer_id| {
                let transfer = transfer_map::get_by_transfer_id(transfer_id)?;
                let token = token_map::get_by_token_id(transfer.token_id)?;
                TransferIdReply::try_from((transfer_id, &transfer, &token)).ok()
            }).collect(),
            claim_ids: claim_ids.to_vec(),
            ts,
        }
    }
}
