use ic_cdk::query;
use ic_stable_structures::Memory;
use serde_json::json;

use crate::helpers::math_helpers::{bytes_to_megabytes, to_trillions};
use crate::ic::guards::caller_is_kingkong;

use crate::stable_memory::{
    CLAIM_MAP, CLAIM_MEMORY_ID, DB_UPDATE_MAP, DB_UPDATE_MEMORY_ID, KONG_SETTINGS_MEMORY_ID, LP_TOKEN_MAP, LP_TOKEN_MEMORY_ID,
    MEMORY_MANAGER, POOL_MAP, POOL_MEMORY_ID, REQUEST_MAP, REQUEST_MEMORY_ID, TOKEN_MAP, TOKEN_MEMORY_ID, TRANSFER_MAP, TRANSFER_MEMORY_ID,
    TX_MAP, TX_MEMORY_ID, USER_MAP, USER_MEMORY_ID,
};

#[cfg(target_arch = "wasm32")]
const WASM_PAGE_SIZE: u64 = 65536;

fn get_cycles() -> u128 {
    #[cfg(target_arch = "wasm32")]
    {
        ic_cdk::api::canister_balance128()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

fn get_stable_memory_size() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        (ic_cdk::api::stable::stable_size() as u64) * WASM_PAGE_SIZE
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

fn get_heap_memory_size() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        (core::arch::wasm32::memory_size(0) as u64) * WASM_PAGE_SIZE
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

#[query(hidden = true, guard = "caller_is_kingkong")]
async fn status() -> Result<String, String> {
    serde_json::to_string(&json! {
        {
            "Kong Data Cycles Balance": format!("{} T", to_trillions(get_cycles())),
            "Heap Memory": format!("{} MiB", bytes_to_megabytes(get_heap_memory_size())),
            "Stable Memory": format!("{} MiB", bytes_to_megabytes(get_stable_memory_size())),
            "Stable - Kong Settings": format!("{} x 64k WASM page", MEMORY_MANAGER.with(|m| m.borrow().get(KONG_SETTINGS_MEMORY_ID).size())),
            "Stable - User Map": format!("{} x 64k WASM page", MEMORY_MANAGER.with(|m| m.borrow().get(USER_MEMORY_ID).size())),
            "Stable - Token Map": format!("{} x 64k WASM page", MEMORY_MANAGER.with(|m| m.borrow().get(TOKEN_MEMORY_ID).size())),
            "Stable - Pool Map": format!("{} x 64k WASM page", MEMORY_MANAGER.with(|m| m.borrow().get(POOL_MEMORY_ID).size())),
            "Stable - Tx Map": format!("{} x 64k WASM page", MEMORY_MANAGER.with(|m| m.borrow().get(TX_MEMORY_ID).size())),
            "Stable - Request Map": format!("{} x 64k WASM page", MEMORY_MANAGER.with(|m| m.borrow().get(REQUEST_MEMORY_ID).size())),
            "Stable - Transfer Map": format!("{} x 64k WASM page", MEMORY_MANAGER.with(|m| m.borrow().get(TRANSFER_MEMORY_ID).size())),
            "Stable - Claim Map": format!("{} x 64k WASM page", MEMORY_MANAGER.with(|m| m.borrow().get(CLAIM_MEMORY_ID).size())),
            "Stable - LP Tokens Map": format!("{} x 64k WASM page", MEMORY_MANAGER.with(|m| m.borrow().get(LP_TOKEN_MEMORY_ID).size())),
            "Stable - DB Update Map": format!("{} x 64k WASM page", MEMORY_MANAGER.with(|m| m.borrow().get(DB_UPDATE_MEMORY_ID).size())),
            "# of users": get_number_of_users(),
            "# of tokens": get_number_of_tokens(),
            "# of pools": get_number_of_pools(),
            "# of requests": get_number_of_requests(),            
            "# of txs": get_number_of_txs(),
            "# of transfers": get_number_of_transfers(),
            "# of claims": get_number_of_claims(),
            "# of LP positions": get_number_of_lp_positions(),
            "# of db updates": get_number_of_db_updates(),
        }
    })
    .map_err(|e| format!("Failed to serialize: {}", e))
}

pub fn get_number_of_users() -> u64 {
    USER_MAP.with(|m| m.borrow().len())
}

pub fn get_number_of_tokens() -> u64 {
    TOKEN_MAP.with(|m| m.borrow().len())
}

pub fn get_number_of_pools() -> u64 {
    POOL_MAP.with(|m| m.borrow().len())
}

pub fn get_number_of_txs() -> u64 {
    TX_MAP.with(|m| m.borrow().len())
}

pub fn get_number_of_requests() -> u64 {
    REQUEST_MAP.with(|m| m.borrow().len())
}

pub fn get_number_of_transfers() -> u64 {
    TRANSFER_MAP.with(|m| m.borrow().len())
}

pub fn get_number_of_claims() -> u64 {
    CLAIM_MAP.with(|m| m.borrow().len())
}

pub fn get_number_of_lp_positions() -> u64 {
    LP_TOKEN_MAP.with(|m| m.borrow().len())
}

pub fn get_number_of_db_updates() -> u64 {
    DB_UPDATE_MAP.with(|m| m.borrow().len())
}
