use ic_cdk::query;

use crate::ic::guards::not_in_maintenance_mode;
use crate::stable_token::token_map;

/// Checks if a token exists by its address
/// This is more efficient than fetching all tokens
#[query(guard = "not_in_maintenance_mode")]
fn token_exists(address: String) -> bool {
    token_map::get_by_address(&address).is_ok()
}