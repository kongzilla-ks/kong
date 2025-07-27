use ic_cdk::query;

use super::super::stable_memory::get_cached_solana_address;

/// Get the cached Solana address for this canister
/// This is a fast query method that returns the cached address
#[query]
pub fn get_solana_address() -> Result<String, String> {
    let address = get_cached_solana_address();
    if address.is_empty() {
        Err("Solana address not cached yet. Call cache_solana_address() first.".to_string())
    } else {
        Ok(address)
    }
}
