pub mod add_liquidity;
pub mod add_liquidity_amounts;
pub mod add_pool;
pub mod add_token;
pub mod canister;
pub mod chains;
pub mod claims;
pub mod controllers;
pub mod helpers;
pub mod ic;
pub mod kong_backend;
pub mod kong_data;
pub mod pools;
pub mod remove_liquidity;
pub mod remove_liquidity_amounts;
pub mod requests;
pub mod send;
pub mod solana;
pub mod stable_claim;
pub mod stable_kong_settings;
pub mod stable_lp_token;
pub mod stable_memory;
pub mod stable_pool;
pub mod stable_request;
pub mod stable_token;
pub mod stable_transfer;
pub mod stable_tx;
pub mod stable_user;
pub mod swap;
pub mod swap_amounts;
pub mod tokens;
pub mod transfers;
pub mod user;
pub mod user_balances;

pub const APP_NAME: &str = "KongSwap";
pub const APP_VERSION: &str = "v0.0.26";

// Custom getrandom implementation for IC canisters
use getrandom::{register_custom_getrandom, Error};
use std::cell::RefCell;

thread_local! {
    static RANDOM_SEED: RefCell<[u8; 32]> = RefCell::new([0u8; 32]);
}

fn custom_getrandom(buf: &mut [u8]) -> Result<(), Error> {
    // Use IC's time-based entropy as a simple fallback
    // This is not cryptographically secure but sufficient for basic randomness needs
    let time_nanos = ic_cdk::api::time();
    let mut seed_bytes = time_nanos.to_le_bytes();

    // Simple XOR-based PRNG for filling the buffer
    for (i, byte) in buf.iter_mut().enumerate() {
        let idx = i % seed_bytes.len();
        *byte = seed_bytes[idx] ^ ((time_nanos >> (i % 64)) as u8);
        // Mix the seed for next iteration
        seed_bytes[idx] = seed_bytes[idx].wrapping_add(1);
    }

    Ok(())
}

register_custom_getrandom!(custom_getrandom);
