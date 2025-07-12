use ic_cdk::query;

use crate::ic::guards::not_in_maintenance_mode;
use crate::stable_pool::pool_map;

use super::pools_reply::PoolReply;

#[query(guard = "not_in_maintenance_mode")]
fn pools(symbol: Option<String>) -> Result<Vec<PoolReply>, String> {
    Ok(match symbol {
        Some(symbol) => pool_map::get_by_token_wildcard(&symbol),
        None => pool_map::get(),
    }
    .iter()
    .map(PoolReply::from)
    .collect())
}
