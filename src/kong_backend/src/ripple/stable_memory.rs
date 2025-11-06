use ic_stable_structures::StableCell;

use crate::stable_memory::{Memory, CACHED_RIPPLE_ADDRESS};

/// Helper function to access the cached Ripple address
pub fn with_cached_ripple_address<R>(f: impl FnOnce(&StableCell<String, Memory>) -> R) -> R {
    CACHED_RIPPLE_ADDRESS.with(|cell| f(&cell.borrow()))
}

/// Helper function to mutate the cached Ripple address
pub fn with_cached_ripple_address_mut<R>(f: impl FnOnce(&mut StableCell<String, Memory>) -> R) -> R {
    CACHED_RIPPLE_ADDRESS.with(|cell| f(&mut cell.borrow_mut()))
}

/// Get the cached Ripple address from stable memory
pub fn get_cached_ripple_address() -> String {
    with_cached_ripple_address(|cell| cell.get().clone())
}

/// Set the cached Ripple address in stable memory
pub fn set_cached_ripple_address(address: String) {
    with_cached_ripple_address_mut(|cell| {
        cell.set(address).expect("Failed to set cached Ripple address");
    });
}
