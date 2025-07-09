use crate::chains::chains::{IC_CHAIN, SOL_CHAIN};
use crate::stable_token::stable_token::StableToken;
use crate::stable_token::token_map;
use crate::stable_transfer::transfer_map;
use crate::stable_transfer::tx_id::TxId;

use super::transfer_reply::{ICTransferReply, SolanaTransferReply, TransferIdReply, TransferReply};

pub fn to_transfer_ids(transfer_ids: &[u64]) -> Vec<TransferIdReply> {
    transfer_ids.iter().filter_map(|&transfer_id| to_transfer_id(transfer_id)).collect()
}

pub fn to_transfer_id(transfer_id: u64) -> Option<TransferIdReply> {
    // 1. Get the transfer by its ID
    match transfer_map::get_by_transfer_id(transfer_id) {
        Some(transfer) => {
            // 2. Get the associated token information
            match token_map::get_by_token_id(transfer.token_id) {
                // Case 1: The token is an IC token
                Some(StableToken::IC(token)) => match transfer.tx_id {
                    TxId::BlockIndex(block_index) => Some(TransferIdReply {
                        transfer_id,
                        transfer: TransferReply::IC(ICTransferReply {
                            chain: IC_CHAIN.to_string(),
                            symbol: token.symbol,
                            is_send: transfer.is_send,
                            amount: transfer.amount,
                            canister_id: token.canister_id.to_string(),
                            block_index,
                        }),
                    }),
                    _ => None, // A BlockIndex is expected for IC tokens
                },
                // Case 2: The token is a Solana token
                Some(StableToken::Solana(token)) => match &transfer.tx_id {
                    TxId::TransactionId(signature) => Some(TransferIdReply {
                        transfer_id,
                        transfer: TransferReply::Solana(SolanaTransferReply {
                            chain: SOL_CHAIN.to_string(),
                            symbol: token.symbol,
                            is_send: transfer.is_send,
                            amount: transfer.amount,
                            mint_address: token.mint_address,
                            signature: signature.clone(),
                            slot: None, // This can be populated later if the slot info is stored
                        }),
                    }),
                    _ => None, // A TransactionId is expected for Solana tokens
                },
                _ => None, // Token not found
            }
        }
        _ => None, // Transfer not found
    }
}
