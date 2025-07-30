use candid::decode_one;
use ic_cdk;

use crate::solana::stable_memory::get_solana_transaction;
use crate::stable_transfer::tx_id::TxId;

#[allow(clippy::module_inception)]
pub mod add_pool;
pub mod add_pool_args;
pub mod add_pool_reply;

use add_pool_args::AddPoolArgs;

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

    if let Ok(add_pool_args) = decode_one::<AddPoolArgs>(&args_bytes) {
        check_transaction_ready(&add_pool_args.signature_0, &add_pool_args.tx_id_0)?;
        check_transaction_ready(&add_pool_args.signature_1, &add_pool_args.tx_id_1)?;
        
        check_solana_signature_provided(&add_pool_args.token_0, &add_pool_args.signature_0)?;
        check_solana_signature_provided(&add_pool_args.token_1, &add_pool_args.signature_1)?;
    }

    Ok(())
}
