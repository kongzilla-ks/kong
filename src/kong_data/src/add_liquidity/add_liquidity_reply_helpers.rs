use super::add_liquidity_reply::AddLiquidityReply;

use crate::stable_pool::pool_map;
use crate::stable_tx::add_liquidity_tx::AddLiquidityTx;

fn get_pool_info(pool_id: u32) -> (String, String, String, String, String, String, String) {
    pool_map::get_by_pool_id(pool_id).map_or_else(
        || {
            (
                "Pool symbol not found".to_string(),
                "Pool chain_0 not found".to_string(),
                "Pool address_0 not found".to_string(),
                "Pool symbol_0 not found".to_string(),
                "Pool chain_1 not found".to_string(),
                "Pool address_1 not found".to_string(),
                "Pool symbol_1 not found".to_string(),
            )
        },
        |pool| {
            (
                pool.symbol(),
                pool.chain_0(),
                pool.address_0(),
                pool.symbol_0(),
                pool.chain_1(),
                pool.address_1(),
                pool.symbol_1(),
            )
        },
    )
}

pub fn to_add_liquidity_reply(add_liquidity_tx: &AddLiquidityTx) -> AddLiquidityReply {
    AddLiquidityReply::try_from(add_liquidity_tx).unwrap_or_else(|_| {
        AddLiquidityReply::failed(add_liquidity_tx.pool_id, add_liquidity_tx.request_id, &add_liquidity_tx.transfer_ids, &add_liquidity_tx.claim_ids, add_liquidity_tx.ts)
    })
}
