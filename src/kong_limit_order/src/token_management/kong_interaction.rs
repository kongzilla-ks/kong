use candid::{CandidType, Nat, Principal};
use kong_lib::{
    ic::address::Address,
    stable_token::{stable_token::StableToken, token::Token},
    stable_transfer::tx_id::TxId,
    swap::{swap_args::SwapArgs, swap_reply::SwapReply},
    token_management::send,
};
use serde::{Deserialize, Serialize};

use crate::stable_memory_helpers::get_kong_backend;

const KONG_BACKEND_ERROR_PREFIX: &str = "Kong backend error:";

#[derive(CandidType, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendAndSwapErr {
    pub error: String,
    pub used_txid: Option<TxId>,
    pub is_kong_error: bool,
}

pub async fn send_assets_and_swap(
    pay_amount: Nat,
    pay_token: &StableToken,
    receive_symbol: String,
    receive_address: String,
    pay_tx_id: Option<TxId>,
) -> Result<SwapReply, SendAndSwapErr> {
    let kong_backend = Principal::from_text(get_kong_backend()).unwrap();
    let kong_backend_address: Address = Address::PrincipalId(kong_backend.into());

    let pay_amount = pay_amount.clone() - pay_token.fee();

    let block_id = if let Some(block_id) = &pay_tx_id {
        block_id.clone()
    } else {
        ic_cdk::println!("Sending assets: {}", pay_amount);
        let block_id = send::send(&pay_amount, &kong_backend_address, pay_token, None)
            .await
            .map_err(|error| SendAndSwapErr{error, used_txid: None, is_kong_error: false})?;
        TxId::BlockIndex(block_id)
    };

    // Need closure so ? exits from the closure, not function
    let kong_backend_call = async || {
        ic_cdk::call::Call::unbounded_wait(kong_backend, "swap")
            .with_arg(SwapArgs {
                pay_token: pay_token.symbol().clone(),
                pay_amount: pay_amount,
                pay_tx_id: Some(block_id.clone()),
                receive_token: receive_symbol,
                receive_amount: None,
                receive_address: Some(receive_address),
                max_slippage: Some(100.0), // Default kong backend slippage is 2, which may fail
                referred_by: None,
                pay_signature: None,
            })
            .await
            .map_err(|e| e.to_string())?
            .candid::<Result<SwapReply, String>>()
            .map_err(|e| e.to_string())?
            .map_err(|e| format!("{} {}", KONG_BACKEND_ERROR_PREFIX, e))
    };

    kong_backend_call().await.map_err(|error| {
        let (is_kong_error, error) = match error.strip_prefix(KONG_BACKEND_ERROR_PREFIX)
        {
            Some(e) => (true, e.to_string()),
            None => (false, error),
        };
        SendAndSwapErr{error, used_txid: Some(block_id), is_kong_error}
    })
}
