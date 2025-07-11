use icrc_ledger_types::icrc1::account::Account;

use crate::chains::chains::SOL_CHAIN;
use crate::ic::guards::not_in_maintenance_mode;
use crate::ic::network::ICNetwork;
use crate::stable_kong_settings::kong_settings_map;
use crate::stable_memory::{get_solana_transaction, TRANSFER_ARCHIVE_MAP, TRANSFER_MAP};
use crate::stable_request::{request_map, status::StatusCode};
use crate::stable_token::{token::Token, token_map};
use crate::stable_transfer::tx_id::TxId;
use crate::swap::return_pay_token::return_pay_token;

use super::stable_transfer::{StableTransfer, StableTransferId};

pub async fn archive_transfer_map() {
    if not_in_maintenance_mode().is_err() {
        return;
    }

    // archive transfers
    TRANSFER_MAP.with(|transfer_map| {
        TRANSFER_ARCHIVE_MAP.with(|transfer_archive_map| {
            let transfer = transfer_map.borrow();
            let mut transfer_archive = transfer_archive_map.borrow_mut();
            let start_transfer_id = transfer_archive.last_key_value().map_or(0_u64, |(k, _)| k.0);
            let end_transfer_id = transfer.last_key_value().map_or(0_u64, |(k, _)| k.0);
            for transfer_id in start_transfer_id..=end_transfer_id {
                if let Some(transfer) = transfer.get(&StableTransferId(transfer_id)) {
                    transfer_archive.insert(StableTransferId(transfer_id), transfer);
                }
            }
        });
    });

    // Process old transfers before removing them
    let one_hour_ago = ICNetwork::get_time() - 3_600_000_000_000;
    let mut remove_list = Vec::new();
    let mut unclaimed_solana_transfers = Vec::new();
    
    // First pass: collect transfers to process
    TRANSFER_MAP.with(|transfer_map| {
        transfer_map.borrow().iter().for_each(|(transfer_id, transfer)| {
            if transfer.ts < one_hour_ago {
                remove_list.push(transfer_id);
                
                // Check if this is an unclaimed incoming Solana transfer
                if transfer.is_send && matches!(transfer.tx_id, TxId::TransactionId(_)) {
                    // This is an incoming transfer (is_send = true from user perspective)
                    // Check if it's a Solana transfer that wasn't used in a swap
                    if let TxId::TransactionId(ref signature) = transfer.tx_id {
                        // Check if this signature was used in any successful operation
                        // If we're archiving it after 1 hour and it's still just sitting here,
                        // it means the user never successfully used it
                        unclaimed_solana_transfers.push((transfer.clone(), signature.clone()));
                    }
                }
            }
        });
    });

    // Process unclaimed Solana transfers
    for (transfer, signature) in unclaimed_solana_transfers {
        // Check if we should return this transfer
        if should_return_transfer(&transfer, &signature).await {
            ic_cdk::println!("Returning unclaimed Solana transfer: {} amount: {}", signature, transfer.amount);
            
            // Get token information
            if let Some(token) = token_map::get_by_token_id(transfer.token_id) {
                if token.chain() == SOL_CHAIN {
                    // Extract sender address from Solana transaction metadata
                    if let Some(_sender_address) = get_solana_sender_address(&signature) {
                        // Create a synthetic request for tracking
                        let request_id = kong_settings_map::inc_request_map_idx();
                        
                        // Create initial request entry
                        request_map::update_status(
                            request_id,
                            StatusCode::Start,
                            Some(&format!("Auto-return unclaimed transfer after 1hr timeout: {}", signature)),
                        );

                        // Create a dummy account for the return (Solana addresses don't map to IC principals)
                        let dummy_principal = Account {
                            owner: ic_cdk::api::canister_self(),
                            subaccount: None,
                        };

                        let mut transfer_ids = vec![transfer.transfer_id];

                        // Call the existing return_pay_token function
                        return_pay_token(
                            request_id,
                            0, // No specific user_id for timeout returns
                            &dummy_principal,
                            &token,
                            &transfer.amount,
                            None, // No receive token for timeout returns
                            &mut transfer_ids,
                            transfer.ts,
                        ).await;

                        // Log the return attempt
                        if let Some(request) = request_map::get_by_request_id(request_id) {
                            if let Some(latest_status) = request.statuses.last() {
                                match latest_status.status_code {
                                    StatusCode::ReturnPayTokenSuccess => {
                                        ic_cdk::println!("Successfully returned unclaimed transfer: {}", signature);
                                    }
                                    StatusCode::ReturnPayTokenFailed => {
                                        ic_cdk::println!("Failed to return unclaimed transfer: {}", signature);
                                    }
                                    _ => {
                                        ic_cdk::println!("Unexpected status for return: {:?}", latest_status.status_code);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Remove old transfers from the active map
    TRANSFER_MAP.with(|transfer_map| {
        remove_list.iter().for_each(|transfer_id| {
            transfer_map.borrow_mut().remove(transfer_id);
        });
    });
}

// Check if we should return this transfer
async fn should_return_transfer(transfer: &StableTransfer, signature: &str) -> bool {
    // Only return if:
    // 1. It's an incoming transfer (is_send = true)
    // 2. It's a Solana transfer
    // 3. We have the transaction metadata
    // 4. It wasn't used in any successful operation
    
    if !transfer.is_send {
        return false; // This is an outgoing transfer, not incoming
    }

    // Check if we have the Solana transaction metadata
    if get_solana_transaction(signature.to_string()).is_none() {
        return false; // No metadata available
    }

    // If this transfer is being archived after 1 hour and it's still in the map,
    // it means it was never successfully used in a swap/liquidity operation
    true
}

// Extract sender address from Solana transaction metadata
fn get_solana_sender_address(signature: &str) -> Option<String> {
    if let Some(notification) = get_solana_transaction(signature.to_string()) {
        if let Some(metadata_json) = notification.metadata {
            if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_json) {
                // Try different fields where sender might be stored
                if let Some(sender) = metadata.get("sender")
                    .or_else(|| metadata.get("authority"))
                    .or_else(|| metadata.get("sender_wallet"))
                    .and_then(|v| v.as_str()) {
                    return Some(sender.to_string());
                }
            }
        }
    }
    None
}