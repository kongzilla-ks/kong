use ic_cdk::update;

use crate::ic::network::ICNetwork;
use crate::{ic::guards::caller_is_kong_rpc, stable_memory::with_solana_tx_notifications_mut};

use super::transaction_notification::{TransactionNotification, TransactionNotificationId};

/// Notify about a Solana transfer (called by kong_rpc)
#[update(hidden = true, guard = "caller_is_kong_rpc")]
pub fn notify_solana_transfer(tx_signature: String, metadata: Option<String>) -> Result<(), String> {
    let key = TransactionNotificationId(tx_signature);
    let value = TransactionNotification {
        status: "confirmed".to_string(), // Incoming payments are always confirmed
        metadata,
        timestamp: ICNetwork::get_time(),
    };
    with_solana_tx_notifications_mut(|notification| {
        notification.insert(key, value);
        Ok(())
    })
}
