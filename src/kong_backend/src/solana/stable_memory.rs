use ic_stable_structures::{StableBTreeMap, StableCell};

use crate::ic::network::ICNetwork;
use crate::solana::swap_job::SwapJob;
use crate::stable_memory::{
    Memory, CACHED_SOLANA_ADDRESS, NEXT_SOLANA_SWAP_JOB_ID, SOLANA_BLOCKHASH, SOLANA_SWAP_JOB_QUEUE, SOLANA_TX_NOTIFICATIONS,
};

use super::kong_rpc::transaction_notification::{TransactionNotification, TransactionNotificationId};
use super::swap_job::SwapJobId;

/// Helper function to access the cached Solana address
pub fn with_cached_solana_address<R>(f: impl FnOnce(&StableCell<String, Memory>) -> R) -> R {
    CACHED_SOLANA_ADDRESS.with(|cell| f(&cell.borrow()))
}

/// Helper function to mutate the cached Solana address
pub fn with_cached_solana_address_mut<R>(f: impl FnOnce(&mut StableCell<String, Memory>) -> R) -> R {
    CACHED_SOLANA_ADDRESS.with(|cell| f(&mut cell.borrow_mut()))
}

/// Get the cached Solana address from stable memory
pub fn get_cached_solana_address() -> String {
    with_cached_solana_address(|cell| cell.get().clone())
}

/// Set the cached Solana address in stable memory
pub fn set_cached_solana_address(address: String) {
    with_cached_solana_address_mut(|cell| {
        cell.set(address).expect("Failed to set cached Solana address");
    });
}

/// Helper function to access the latest blockhash cell
pub fn with_solana_blockhash<R>(f: impl FnOnce(&StableCell<String, Memory>) -> R) -> R {
    SOLANA_BLOCKHASH.with(|cell| f(&cell.borrow()))
}

/// Helper function to mutate the latest blockhash cell
pub fn with_solana_blockhash_mut<R>(f: impl FnOnce(&mut StableCell<String, Memory>) -> R) -> R {
    SOLANA_BLOCKHASH.with(|cell| f(&mut cell.borrow_mut()))
}

/// Get the next unique ID for a Solana swap job and increment the counter.
pub fn get_next_solana_swap_job_id() -> u64 {
    NEXT_SOLANA_SWAP_JOB_ID.with(|cell| {
        let current_id = *cell.borrow().get();
        let next_id = current_id + 1;
        // Before updating, ensure the cell isn't borrowed mutably elsewhere if using set directly.
        // For StableCell, it's simpler:
        cell.borrow_mut().set(next_id).expect("Failed to set next_solana_swap_job_id");
        current_id
    })
}

/// Helper function to access the swap job queue
pub fn with_swap_job_queue<R>(f: impl FnOnce(&StableBTreeMap<SwapJobId, SwapJob, Memory>) -> R) -> R {
    SOLANA_SWAP_JOB_QUEUE.with(|cell| f(&cell.borrow()))
}

/// Helper function to mutate the swap job queue
pub fn with_swap_job_queue_mut<R>(f: impl FnOnce(&mut StableBTreeMap<SwapJobId, SwapJob, Memory>) -> R) -> R {
    SOLANA_SWAP_JOB_QUEUE.with(|cell| f(&mut cell.borrow_mut()))
}

/// Helper function to access solana transaction notifications
pub fn with_solana_tx_notifications<R>(
    f: impl FnOnce(&StableBTreeMap<TransactionNotificationId, TransactionNotification, Memory>) -> R,
) -> R {
    SOLANA_TX_NOTIFICATIONS.with(|cell| f(&cell.borrow()))
}

/// Helper function to mutate solana transaction notifications
pub fn with_solana_tx_notifications_mut<R>(
    f: impl FnOnce(&mut StableBTreeMap<TransactionNotificationId, TransactionNotification, Memory>) -> R,
) -> R {
    SOLANA_TX_NOTIFICATIONS.with(|cell| f(&mut cell.borrow_mut()))
}

/// Get a transaction by tx_signature
pub fn get_solana_transaction(tx_signature: String) -> Option<TransactionNotification> {
    with_solana_tx_notifications(|notifications| notifications.get(&TransactionNotificationId(tx_signature)))
}

/// Clean up old notifications (older than 24 hours)
///
/// Used by the canister's background timer (canister.rs:125) which runs every hour
/// to remove transaction notifications that are older than 24 hours.
pub fn cleanup_old_notifications() {
    const TWENTY_FOUR_HOURS_NANOS: u64 = 24 * 60 * 60 * 1_000_000_000;
    let current_time = ICNetwork::get_time();
    let cutoff_time = current_time.saturating_sub(TWENTY_FOUR_HOURS_NANOS);

    with_solana_tx_notifications_mut(|notifications| {
        let mut to_remove = Vec::new();

        // Find old entries
        for (key, notification) in notifications.iter() {
            if notification.timestamp < cutoff_time {
                to_remove.push(key.clone());
            }
        }

        // Remove them
        for key in to_remove.iter() {
            notifications.remove(key);
        }
    })
}
