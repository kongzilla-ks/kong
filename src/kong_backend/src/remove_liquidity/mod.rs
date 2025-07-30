use candid::decode_one;
use ic_cdk;

#[allow(clippy::module_inception)]
pub mod remove_liquidity;
pub mod remove_liquidity_args;
pub mod remove_liquidity_reply;

use remove_liquidity_args::RemoveLiquidityArgs;

pub fn validate_arguments() -> Result<(), String> {
    let args_bytes = ic_cdk::api::msg_arg_data();

    if args_bytes.len() > 10_000 {
        return Err("Request payload too large".to_string());
    }

    if let Ok(remove_args) = decode_one::<RemoveLiquidityArgs>(&args_bytes) {
        if remove_args.token_0.starts_with("SOL.") && remove_args.payout_address_0.is_none() {
            return Err("Solana token requires payout_address_0".to_string());
        }
        if remove_args.token_1.starts_with("SOL.") && remove_args.payout_address_1.is_none() {
            return Err("Solana token requires payout_address_1".to_string());
        }
    }

    Ok(())
}
