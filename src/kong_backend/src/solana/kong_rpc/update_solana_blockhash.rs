use ic_cdk::update;

use crate::ic::guards::caller_is_kong_rpc;

/// Update the Solana blockhash (called by kong_rpc)
/// Currently disabled - canister fetches blockhash via HTTP outcalls
#[update(hidden = true, guard = "caller_is_kong_rpc")]
pub fn update_solana_blockhash(_blockhash: String) -> Result<(), String> {
    Err("Blockhash updates disabled - using HTTP outcalls".to_string())
}
