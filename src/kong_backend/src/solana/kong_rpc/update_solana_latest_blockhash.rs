use ic_cdk::update;

use crate::ic::guards::caller_is_kong_rpc;
use crate::stable_memory::with_solana_latest_blockhash_mut;

/// Update the latest Solana blockhash (called by kong_rpc)
#[update(hidden = true, guard = "caller_is_kong_rpc")]
pub fn update_solana_latest_blockhash(blockhash: String) -> Result<(), String> {
    with_solana_latest_blockhash_mut(|cell| {
        cell.set(blockhash).map_err(|_| "Failed to update latest blockhash".to_string())?;
        Ok(())
    })
}
