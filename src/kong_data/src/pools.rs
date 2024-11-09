use ic_cdk::{query, update};
use kong_lib::stable_pool::stable_pool::{StablePool, StablePoolId};
use std::collections::BTreeMap;

use super::guards::caller_is_kong_backend;
use super::stable_memory::POOL_MAP;

const MAX_POOLS: usize = 1_000;

//#[query(hidden = true, guard = "caller_is_kingkong")]
#[query(hidden = true)]
fn backup_pools(pool_id: Option<u32>, num_pools: Option<u16>) -> Result<String, String> {
    POOL_MAP.with(|m| {
        let map = m.borrow();
        let pools: BTreeMap<_, _> = match pool_id {
            Some(pool_id) => {
                let num_pools = num_pools.map_or(1, |n| n as usize);
                let start_key = StablePoolId(pool_id);
                map.range(start_key..).take(num_pools).collect()
            }
            None => {
                let num_pools = num_pools.map_or(MAX_POOLS, |n| n as usize);
                map.iter().take(num_pools).collect()
            }
        };

        serde_json::to_string(&pools).map_err(|e| format!("Failed to serialize pools: {}", e))
    })
}

//#[update(guard = "caller_is_kong_backend")]
#[update(hidden = true)]
fn archive_pools(tokens: String) -> Result<String, String> {
    let pools: BTreeMap<StablePoolId, StablePool> = match serde_json::from_str(&tokens) {
        Ok(pools) => pools,
        Err(e) => return Err(format!("Invalid pools: {}", e)),
    };

    POOL_MAP.with(|user_map| {
        let mut map = user_map.borrow_mut();
        for (k, v) in pools.into_iter() {
            map.insert(k, v);
        }
    });

    Ok("Pools archived".to_string())
}
