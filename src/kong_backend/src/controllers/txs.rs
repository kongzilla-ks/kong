use ic_cdk::{query, update};
use std::collections::BTreeMap;

use crate::ic::guards::caller_is_kingkong;
use crate::stable_memory::{TX_ARCHIVE_MAP, TX_MAP};
use crate::stable_tx::stable_tx::StableTxId;
use crate::stable_tx::tx_archive::archive_tx_map;
use crate::stable_tx::tx_map;
use crate::txs::txs_reply::TxsReply;
use crate::txs::txs_reply_impl::to_txs_reply;

const MAX_TXS: usize = 1_000;

#[query(hidden = true, guard = "caller_is_kingkong")]
fn backup_txs(tx_id: Option<u64>, num_txs: Option<u16>) -> Result<String, String> {
    TX_MAP.with(|m| {
        let map = m.borrow();
        let txs: BTreeMap<_, _> = match tx_id {
            Some(tx_id) => {
                let num_txs = num_txs.map_or(1, |n| n as usize);
                let start_key = StableTxId(tx_id);
                map.range(start_key..).take(num_txs).collect()
            }
            None => {
                let num_txs = num_txs.map_or(MAX_TXS, |n| n as usize);
                map.iter().take(num_txs).collect()
            }
        };

        serde_json::to_string(&txs).map_err(|e| format!("Failed to serialize txs: {}", e))
    })
}

#[query(hidden = true, guard = "caller_is_kingkong")]
fn backup_archive_txs(tx_id: Option<u64>, num_txs: Option<u16>) -> Result<String, String> {
    TX_ARCHIVE_MAP.with(|m| {
        let map = m.borrow();
        let txs: BTreeMap<_, _> = match tx_id {
            Some(tx_id) => {
                let num_txs = num_txs.map_or(1, |n| n as usize);
                let start_key = StableTxId(tx_id);
                map.range(start_key..).take(num_txs).collect()
            }
            None => {
                let num_txs = num_txs.map_or(MAX_TXS, |n| n as usize);
                map.iter().take(num_txs).collect()
            }
        };

        serde_json::to_string(&txs).map_err(|e| format!("Failed to serialize txs: {}", e))
    })
}

#[update(hidden = true, guard = "caller_is_kingkong")]
fn archive_txs() -> Result<String, String> {
    archive_tx_map();
    Ok("txs archived".to_string())
}

#[query(hidden = true, guard = "caller_is_kingkong")]
pub fn get_txs(tx_id: Option<u64>, user_id: Option<u32>, token_id: Option<u32>) -> Result<Vec<TxsReply>, String> {
    let txs = match tx_id {
        Some(tx_id) => tx_map::get_by_tx_and_user_id(tx_id, user_id).into_iter().collect(),
        None => tx_map::get_by_user_and_token_id(user_id, token_id, MAX_TXS),
    };

    Ok(txs.iter().map(to_txs_reply).collect())
}

#[update(hidden = true, guard = "caller_is_kingkong")]
fn remove_txs(start_tx_id: u64, end_tx_id: u64) -> Result<String, String> {
    TX_MAP.with(|m| {
        let mut map = m.borrow_mut();
        let keys_to_remove: Vec<_> = map
            .iter()
            .filter(|(tx_id, _)| tx_id.0 >= start_tx_id && tx_id.0 <= end_tx_id)
            .map(|(tx_id, _)| tx_id)
            .collect();
        keys_to_remove.iter().for_each(|tx_id| {
            map.remove(tx_id);
        });
    });
    Ok("txs removed".to_string())
}
