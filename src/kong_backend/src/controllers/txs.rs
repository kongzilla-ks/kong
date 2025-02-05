use ic_cdk::{query, update};
use std::collections::BTreeMap;

use crate::ic::guards::caller_is_kingkong;
use crate::stable_memory::{TX_ARCHIVE_MAP, TX_MAP};
use crate::stable_tx::stable_tx::{StableTx, StableTxId};
use crate::stable_tx::tx::Tx;

const MAX_TXS: usize = 100;

/// serialize TX_ARCHIVE_MAP for backup
/// used for storing backup
#[query(hidden = true, guard = "caller_is_kingkong")]
fn backup_archive_txs(tx_id: Option<u64>, num_txs: Option<u16>) -> Result<String, String> {
    TX_ARCHIVE_MAP.with(|m| {
        let map = m.borrow();
        let txs: BTreeMap<_, _> = match tx_id {
            Some(tx_id) => {
                let start_id = StableTxId(tx_id);
                let num_txs = num_txs.map_or(1, |n| n as usize);
                map.range(start_id..).take(num_txs).collect()
            }
            None => {
                let num_txs = num_txs.map_or(MAX_TXS, |n| n as usize);
                map.iter().take(num_txs).collect()
            }
        };
        serde_json::to_string(&txs).map_err(|e| format!("Failed to serialize txs: {}", e))
    })
}

/// deserialize StableTx and update TX_MAP
#[update(hidden = true, guard = "caller_is_kingkong")]
fn update_txs(stable_txs_json: String) -> Result<String, String> {
    let txs: BTreeMap<StableTxId, StableTx> = match serde_json::from_str(&stable_txs_json) {
        Ok(txs) => txs,
        Err(e) => return Err(format!("Invalid txs: {}", e)),
    };

    TX_MAP.with(|tx_map| {
        let mut map = tx_map.borrow_mut();
        for (k, v) in txs {
            map.insert(k, v);
        }
    });

    Ok("Txs updated".to_string())
}

/// remove archive txs older than ts
#[update(hidden = true, guard = "caller_is_kingkong")]
fn remove_archive_txs(ts: u64) -> Result<String, String> {
    TX_ARCHIVE_MAP.with(|m| {
        let mut map = m.borrow_mut();
        let keys_to_remove: Vec<_> = map.iter().filter(|(_, v)| v.ts() < ts).map(|(k, _)| k).collect();
        keys_to_remove.iter().for_each(|k| {
            map.remove(k);
        });
    });

    Ok("Archive txs removed".to_string())
}

/// remove archive txs where tx_id <= tx_ids
#[update(hidden = true, guard = "caller_is_kingkong")]
fn remove_archive_txs_ids(tx_ids: u64) -> Result<String, String> {
    TX_ARCHIVE_MAP.with(|m| {
        let mut map = m.borrow_mut();
        let keys_to_remove: Vec<_> = map.iter().filter(|(k, _)| k.0 <= tx_ids).map(|(k, _)| k).collect();
        keys_to_remove.iter().for_each(|k| {
            map.remove(k);
        });
    });

    Ok("Archive txs removed".to_string())
}
