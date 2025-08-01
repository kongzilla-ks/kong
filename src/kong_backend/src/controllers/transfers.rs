use ic_cdk::{query, update};
use std::cmp::max;
use std::collections::BTreeMap;

use crate::ic::guards::caller_is_kingkong;
use crate::ic::network::ICNetwork;
use crate::solana::stable_memory::get_solana_transaction;
use crate::stable_memory::{TRANSFER_ARCHIVE_MAP, TRANSFER_MAP};
use crate::stable_transfer::stable_transfer::{StableTransfer, StableTransferId};
use crate::stable_transfer::transfer_archive::archive_transfer_map;

const MAX_TRANSFERS: usize = 1000;

#[query(hidden = true, guard = "caller_is_kingkong")]
fn max_transfer_idx() -> u64 {
    TRANSFER_MAP.with(|m| m.borrow().last_key_value().map_or(0, |(k, _)| k.0))
}

/// serialize TRANSFER_ARCHIVE_MAP for backup
/// used for storing backup
#[query(hidden = true, guard = "caller_is_kingkong")]
fn backup_archive_transfers(transfer_id: Option<u64>, num_requests: Option<u16>) -> Result<String, String> {
    TRANSFER_ARCHIVE_MAP.with(|m| {
        let map = m.borrow();
        let transfers: BTreeMap<_, _> = match transfer_id {
            Some(transfer_id) => {
                let start_id = StableTransferId(transfer_id);
                let num_requests = num_requests.map_or(1, |n| n as usize);
                map.range(start_id..).take(num_requests).collect()
            }
            None => {
                let num_requests = num_requests.map_or(MAX_TRANSFERS, |n| n as usize);
                map.iter().take(num_requests).collect()
            }
        };
        serde_json::to_string(&transfers).map_err(|e| format!("Failed to serialize transfers: {}", e))
    })
}

/// deserialize StableTransfer and update TRANSFER_MAP
#[update(hidden = true, guard = "caller_is_kingkong")]
fn update_transfers(stable_transfers_json: String) -> Result<String, String> {
    let transfers: BTreeMap<StableTransferId, StableTransfer> = match serde_json::from_str(&stable_transfers_json) {
        Ok(transfers) => transfers,
        Err(e) => return Err(format!("Invalid transfers: {}", e)),
    };

    TRANSFER_MAP.with(|transfer_map| {
        let mut map = transfer_map.borrow_mut();
        for (k, v) in transfers {
            map.insert(k, v);
        }
    });

    Ok("Transfers updated".to_string())
}

#[update(hidden = true, guard = "caller_is_kingkong")]
async fn archive_transfers() -> Result<String, String> {
    archive_transfer_map().await;

    Ok("Transfers archived".to_string())
}

#[update(hidden = true, guard = "caller_is_kingkong")]
fn archive_transfers_num() -> Result<String, String> {
    TRANSFER_MAP.with(|transfer_map| {
        TRANSFER_ARCHIVE_MAP.with(|transfer_archive_map| {
            let transfer = transfer_map.borrow();
            let mut transfer_archive = transfer_archive_map.borrow_mut();
            let start_transfer_id = max(
                transfer.first_key_value().map_or(0_u64, |(k, _)| k.0),
                transfer_archive.last_key_value().map_or(0_u64, |(k, _)| k.0),
            );
            let end_transfer_id = start_transfer_id + MAX_TRANSFERS as u64;
            for transfer_id in start_transfer_id..=end_transfer_id {
                if let Some(transfer) = transfer.get(&StableTransferId(transfer_id)) {
                    transfer_archive.insert(StableTransferId(transfer_id), transfer);
                }
            }
        });
    });

    Ok("Transfers archived num".to_string())
}

#[update(hidden = true, guard = "caller_is_kingkong")]
fn remove_transfers() -> Result<String, String> {
    // only keep transfers from the last hour
    let one_hour_ago = ICNetwork::get_time() - 3_600_000_000_000;
    let mut remove_list = Vec::new();
    TRANSFER_MAP.with(|transfer_map| {
        transfer_map.borrow().iter().for_each(|(transfer_id, transfer)| {
            if transfer.ts < one_hour_ago {
                remove_list.push(transfer_id);
            }
        });
    });
    TRANSFER_MAP.with(|transfer_map| {
        remove_list.iter().for_each(|transfer_id| {
            transfer_map.borrow_mut().remove(transfer_id);
        });
    });

    Ok("Transfers removed".to_string())
}

/// remove archive transfers older than ts
#[update(hidden = true, guard = "caller_is_kingkong")]
fn remove_archive_transfers(ts: u64) -> Result<String, String> {
    TRANSFER_ARCHIVE_MAP.with(|m| {
        let mut map = m.borrow_mut();
        let keys_to_remove: Vec<_> = map.iter().filter(|(_, v)| v.ts < ts).map(|(k, _)| k).collect();
        keys_to_remove.iter().for_each(|k| {
            map.remove(k);
        });
    });

    Ok("Archive transfers removed".to_string())
}

/// Get Solana transaction data
#[query(hidden = true, guard = "caller_is_kingkong")]
fn debug_get_solana_transaction(signature: String) -> Result<String, String> {
    match get_solana_transaction(signature) {
        Some(tx) => Ok(format!(
            "Transaction found: status={:?}, metadata={:?}, timestamp={}", 
            tx.status, tx.metadata, tx.timestamp
        )),
        None => Err("Transaction not found".to_string()),
    }
}

/// remove archive transfers where transfer_id <= transfer_ids
#[update(hidden = true, guard = "caller_is_kingkong")]
fn remove_archive_transfers_ids(transfer_ids: u64) -> Result<String, String> {
    TRANSFER_ARCHIVE_MAP.with(|m| {
        let mut map = m.borrow_mut();
        let keys_to_remove: Vec<_> = map.iter().filter(|(k, _)| k.0 <= transfer_ids).map(|(k, _)| k).collect();
        keys_to_remove.iter().for_each(|k| {
            map.remove(k);
        });
    });

    Ok("Archive transfers removed".to_string())
}
