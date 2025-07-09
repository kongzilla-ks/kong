use crate::stable_token::token_map;
use crate::stable_transfer::transfer_map;

use super::transfer_reply::TransferIdReply;

pub fn to_transfer_ids(transfer_ids: &[u64]) -> Vec<TransferIdReply> {
    transfer_ids.iter().filter_map(|&transfer_id| to_transfer_id(transfer_id)).collect()
}

pub fn to_transfer_id(transfer_id: u64) -> Option<TransferIdReply> {
    let transfer = transfer_map::get_by_transfer_id(transfer_id)?;
    let token = token_map::get_by_token_id(transfer.token_id)?;
    TransferIdReply::try_from((transfer_id, &transfer, &token)).ok()
}
