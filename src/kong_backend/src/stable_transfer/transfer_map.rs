use candid::Nat;

use crate::ic::network::ICNetwork;
use crate::stable_kong_settings::kong_settings_map;
use crate::stable_memory::TRANSFER_MAP;
use crate::stable_transfer::stable_transfer::{StableTransfer, StableTransferId};

use super::tx_id::TxId;

pub fn get_by_transfer_id(transfer_id: u64) -> Option<StableTransfer> {
    TRANSFER_MAP.with(|m| m.borrow().get(&StableTransferId(transfer_id)))
}

pub fn contain(token_id: u32, block_id: &Nat) -> bool {
    TRANSFER_MAP.with(|m| {
        m.borrow()
            .iter()
            .any(|(_, v)| v.token_id == token_id && v.tx_id == TxId::BlockIndex(block_id.clone()))
    })
}

/// Check if a transaction (either IC block or Solana signature) has already been used
pub fn contains_tx_id(token_id: u32, tx_id: &TxId) -> bool {
    TRANSFER_MAP.with(|m| {
        m.borrow()
            .iter()
            .any(|(_, v)| v.token_id == token_id && &v.tx_id == tx_id)
    })
}

/// Check if a Solana transaction signature has already been used for a token
pub fn contains_tx_signature(token_id: u32, tx_signature: &str) -> bool {
    TRANSFER_MAP.with(|m| {
        m.borrow()
            .iter()
            .any(|(_, v)| v.token_id == token_id && v.tx_id == TxId::TransactionId(tx_signature.to_string()))
    })
}

/// Check if a Solana transaction signature has been used for any token
pub fn contains_tx_signature_any_token(tx_signature: &str) -> bool {
    TRANSFER_MAP.with(|m| {
        m.borrow()
            .iter()
            .any(|(_, v)| v.tx_id == TxId::TransactionId(tx_signature.to_string()))
    })
}

pub fn insert(transfer: &StableTransfer) -> u64 {
    TRANSFER_MAP.with(|m| {
        let mut map = m.borrow_mut();
        let transfer_id = kong_settings_map::inc_transfer_map_idx();
        let insert_transfer = StableTransfer {
            transfer_id,
            ..transfer.clone()
        };
        map.insert(StableTransferId(transfer_id), insert_transfer);
        transfer_id
    })
}

pub fn archive_to_kong_data(transfer_id: u64) -> Result<(), String> {
    if !kong_settings_map::get().archive_to_kong_data {
        return Ok(());
    }

    let transfer = match get_by_transfer_id(transfer_id) {
        Some(transfer) => transfer,
        None => return Err(format!("Failed to archive. transfer_id #{} not found", transfer_id)),
    };
    let transfer_json = match serde_json::to_string(&transfer) {
        Ok(transfer_json) => transfer_json,
        Err(e) => return Err(format!("Failed to archive transfer_id #{}. {}", transfer_id, e)),
    };

            ic_cdk::futures::spawn(async move {
        let kong_data = kong_settings_map::get().kong_data;
        match ic_cdk::call::Call::unbounded_wait(kong_data, "update_transfer")
            .with_arg((transfer_json,))
            .await
            .map_err(|e| format!("{:?}", e))
            .and_then(|response| response.candid::<(Result<String, String>,)>()
                .map_err(|e| format!("{:?}", e)))
            .unwrap_or_else(|e| (Err(e),))
            .0
        {
            Ok(_) => (),
            Err(e) => ICNetwork::error_log(&format!("Failed to archive transfer_id #{}. {}", transfer.transfer_id, e)),
        }
    });

    Ok(())
}
