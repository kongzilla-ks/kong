use candid::Nat;
use num::{BigRational, Zero};

use crate::helpers::math_helpers::price_rounded;
use crate::helpers::nat_helpers::nat_zero;
use crate::stable_pool::pool_map;
use crate::stable_token::stable_token::StableToken;
use crate::stable_token::token::Token;
use crate::stable_token::token_map;
use crate::stable_tx::status_tx::StatusTx;
use crate::stable_tx::swap_tx::SwapTx;
use crate::transfers::transfer_reply::TransferIdReply;
use crate::stable_transfer::transfer_map;

use super::swap_calc::SwapCalc;
use super::swap_reply::{SwapReply, SwapTxReply};

fn to_swap_tx_reply(swap: &SwapCalc, ts: u64) -> Option<SwapTxReply> {
    let pool = pool_map::get_by_pool_id(swap.pool_id)?;
    let pay_token = token_map::get_by_token_id(swap.pay_token_id)?;
    let pay_chain = pay_token.chain().to_string();
    let pay_address: String = pay_token.address().to_string();
    let pay_symbol = pay_token.symbol().to_string();
    let receive_token = token_map::get_by_token_id(swap.receive_token_id)?;
    let receive_chain = receive_token.chain().to_string();
    let receive_address: String = receive_token.address().to_string();
    let receive_symbol = receive_token.symbol().to_string();
    let price = swap.get_price().unwrap_or(BigRational::zero());
    let price_f64 = price_rounded(&price).unwrap_or(0_f64);
    Some(SwapTxReply {
        pool_symbol: pool.symbol(),
        pay_chain,
        pay_address,
        pay_symbol,
        pay_amount: swap.pay_amount.clone(),
        receive_chain,
        receive_address,
        receive_symbol,
        receive_amount: swap.receive_amount.clone(),
        price: price_f64,
        lp_fee: swap.lp_fee.clone(),
        gas_fee: swap.gas_fee.clone(),
        ts,
    })
}

fn to_txs(txs: &[SwapCalc], ts: u64) -> Vec<SwapTxReply> {
    txs.iter().filter_map(|tx| to_swap_tx_reply(tx, ts)).collect()
}

fn get_tokens_info(pay_token_id: u32, receive_token_id: u32) -> (String, String, String, String, String, String) {
    let pay_token = token_map::get_by_token_id(pay_token_id);
    let (pay_chain, pay_address, pay_symbol) = pay_token.map_or_else(
        || {
            (
                "Pay chain not found".to_string(),
                "Pay address not found".to_string(),
                "Pay symbol not found".to_string(),
            )
        },
        |token| (token.chain().to_string(), token.address(), token.symbol().to_string()),
    );
    let receive_token = token_map::get_by_token_id(receive_token_id);
    let (receive_chain, receive_address, receive_symbol) = receive_token.map_or_else(
        || {
            (
                "Receive chain not found".to_string(),
                "Receive address not found".to_string(),
                "Receive symbol not found".to_string(),
            )
        },
        |token| (token.chain().to_string(), token.address(), token.symbol().to_string()),
    );
    (pay_chain, pay_address, pay_symbol, receive_chain, receive_address, receive_symbol)
}

pub fn to_swap_reply(swap_tx: &SwapTx) -> SwapReply {
    let (pay_chain, pay_address, pay_symbol, receive_chain, receive_address, receive_symbol) =
        get_tokens_info(swap_tx.pay_token_id, swap_tx.receive_token_id);
    SwapReply {
        tx_id: swap_tx.tx_id,
        request_id: swap_tx.request_id,
        status: swap_tx.status.to_string(),
        pay_chain,
        pay_address,
        pay_symbol,
        pay_amount: swap_tx.pay_amount.clone(),
        receive_chain,
        receive_address,
        receive_symbol,
        receive_amount: swap_tx.receive_amount.clone(),
        mid_price: swap_tx.mid_price,
        price: swap_tx.price,
        slippage: swap_tx.slippage,
        txs: to_txs(&swap_tx.txs, swap_tx.ts),
        transfer_ids: swap_tx.transfer_ids.iter().filter_map(|&transfer_id| {
            let transfer = transfer_map::get_by_transfer_id(transfer_id)?;
            let token = token_map::get_by_token_id(transfer.token_id)?;
            TransferIdReply::try_from((transfer_id, &transfer, &token)).ok()
        }).collect(),
        claim_ids: swap_tx.claim_ids.clone(),
        ts: swap_tx.ts,
    }
}

pub fn to_swap_reply_failed(
    request_id: u64,
    pay_token: &StableToken,
    pay_amount: &Nat,
    receive_token: Option<&StableToken>,
    transfer_ids: &[u64],
    claim_ids: &[u64],
    ts: u64,
) -> SwapReply {
    // Pay Token
    let pay_chain = pay_token.chain().to_string();
    let pay_address = pay_token.address().to_string();
    let pay_symbol = pay_token.symbol().to_string();
    // Receive token
    let receive_chain = receive_token.map_or_else(|| "Receive chain not found".to_string(), |token| token.chain().to_string());
    let receive_address = receive_token.map_or_else(|| "Receive address not found".to_string(), |token| token.address().to_string());
    let receive_symbol = receive_token.map_or_else(|| "Receive symbol not found".to_string(), |token| token.symbol().to_string());
    SwapReply {
        tx_id: 0,
        request_id,
        status: StatusTx::Failed.to_string(),
        pay_chain,
        pay_address,
        pay_symbol,
        pay_amount: pay_amount.clone(),
        receive_chain,
        receive_address,
        receive_symbol,
        receive_amount: nat_zero(),
        mid_price: 0_f64,
        price: 0_f64,
        slippage: 0_f64,
        txs: Vec::new(),
        transfer_ids: transfer_ids.iter().filter_map(|&transfer_id| {
            let transfer = transfer_map::get_by_transfer_id(transfer_id)?;
            let token = token_map::get_by_token_id(transfer.token_id)?;
            TransferIdReply::try_from((transfer_id, &transfer, &token)).ok()
        }).collect(),
        claim_ids: claim_ids.to_vec(),
        ts,
    }
}
