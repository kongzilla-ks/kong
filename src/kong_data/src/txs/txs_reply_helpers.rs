use crate::add_liquidity::add_liquidity_reply::AddLiquidityReplyHelpers;
use crate::add_pool::add_pool_reply::AddPoolReplyHelpers;
use crate::remove_liquidity::remove_liquidity_reply::RemoveLiquidityReplyHelpers;
use crate::send::send_reply::SendReplyHelpers;
use crate::stable_tx::stable_tx::StableTx::{self, AddLiquidity, AddPool, RemoveLiquidity, Send, Swap};
use crate::swap::swap_reply_helpers::to_swap_reply;

use super::txs_reply::TxsReply;

pub fn to_txs_reply(tx: &StableTx) -> TxsReply {
    match tx {
        AddPool(tx) => TxsReply::AddPool(tx.to_add_pool_reply()),
        AddLiquidity(tx) => TxsReply::AddLiquidity(tx.to_add_liquidity_reply()),
        RemoveLiquidity(tx) => TxsReply::RemoveLiquidity(tx.to_remove_liquidity_reply()),
        Swap(tx) => TxsReply::Swap(to_swap_reply(tx)),
        Send(tx) => TxsReply::Send(tx.to_send_reply()),
    }
}
