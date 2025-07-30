use candid::decode_one;
use ic_cdk;

use crate::solana::stable_memory::get_solana_transaction;
use crate::stable_transfer::tx_id::TxId;

pub mod archive_to_kong_data;
pub mod calculate_amounts;

pub mod return_pay_token;
pub mod send_receive_token;
#[allow(clippy::module_inception)]
pub mod swap;
pub mod swap_amounts;
pub mod swap_args;
pub mod swap_calc;
pub mod swap_calc_impl;
pub mod swap_reply;
pub mod swap_transfer;
pub mod swap_transfer_from;
pub mod update_liquidity_pool;

use swap_args::SwapArgs;

fn check_transaction_ready(signature: &Option<String>, tx_id: &Option<TxId>) -> Result<(), String> {
    if let (Some(_signature), Some(TxId::TransactionId(tx_sig))) = (signature, tx_id) {
        if get_solana_transaction(tx_sig.to_string()).is_none() {
            return Err("TRANSACTION_NOT_READY".to_string());
        }
    }
    Ok(())
}

fn check_solana_signature_provided(token: &str, signature: &Option<String>) -> Result<(), String> {
    if token.starts_with("SOL.") && signature.is_none() {
        return Err("Solana token transfer requires signature".to_string());
    }
    Ok(())
}

pub fn validate_arguments() -> Result<(), String> {
    let args_bytes = ic_cdk::api::msg_arg_data();

    if args_bytes.len() > 10_000 {
        return Err("Request payload too large".to_string());
    }

    if let Ok(swap_args) = decode_one::<SwapArgs>(&args_bytes) {
        check_transaction_ready(&swap_args.pay_signature, &swap_args.pay_tx_id)?;
        
        check_solana_signature_provided(&swap_args.pay_token, &swap_args.pay_signature)?;

        if swap_args.receive_token.starts_with("SOL.") && swap_args.receive_address.is_none() {
            return Err("Solana token requires receive_address".to_string());
        }
    }

    Ok(())
}

pub fn validate_arguments_async() -> Result<(), String> {
    let args_bytes = ic_cdk::api::msg_arg_data();

    if args_bytes.len() > 10_000 {
        return Err("Request payload too large".to_string());
    }

    // Skip transaction readiness check for async swaps
    // Let the async swap logic handle transaction timing

    if let Ok(swap_args) = decode_one::<SwapArgs>(&args_bytes) {
        if swap_args.receive_token.starts_with("SOL.") && swap_args.receive_address.is_none() {
            return Err("Solana token requires receive_address".to_string());
        }
    }

    Ok(())
}
