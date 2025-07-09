use crate::add_liquidity::add_liquidity_reply_helpers::to_add_liquidity_reply;
use crate::add_pool::add_pool_reply_helpers::to_add_pool_reply;
use crate::remove_liquidity::remove_liquidity_reply_helpers::to_remove_liquidity_reply;
use crate::send::send_reply::SendReply;
use crate::stable_tx::stable_tx::StableTx::{self, AddLiquidity, AddPool, RemoveLiquidity, Send, Swap};
use crate::swap::swap_reply_helpers::to_swap_reply;

use super::txs_reply::TxsReply;

pub fn to_txs_reply(tx: &StableTx) -> TxsReply {
    match tx {
        AddPool(tx) => TxsReply::AddPool(to_add_pool_reply(tx)),
        AddLiquidity(tx) => TxsReply::AddLiquidity(to_add_liquidity_reply(tx)),
        RemoveLiquidity(tx) => TxsReply::RemoveLiquidity(to_remove_liquidity_reply(tx)),
        Swap(tx) => TxsReply::Swap(to_swap_reply(tx)),
        Send(tx) => TxsReply::Send(SendReply::try_from(tx).unwrap_or_else(|_| SendReply {
            tx_id: tx.tx_id,
            request_id: tx.request_id,
            status: tx.status.to_string(),
            chain: "Chain not found".to_string(),
            symbol: "Symbol not found".to_string(),
            amount: tx.amount.clone(),
            to_address: "Address not found".to_string(),
            ts: tx.ts,
        })),
    }
}
