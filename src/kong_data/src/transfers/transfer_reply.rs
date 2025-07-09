use candid::{CandidType, Deserialize, Nat};
use serde::Serialize;

use crate::chains::chains::IC_CHAIN;
use crate::stable_token::stable_token::StableToken;
use crate::stable_transfer::stable_transfer::StableTransfer;
use crate::stable_transfer::tx_id::TxId;

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct TransferIdReply {
    pub transfer_id: u64,
    pub transfer: TransferReply,
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub enum TransferReply {
    IC(ICTransferReply),
    Solana(SolanaTransferReply),
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct ICTransferReply {
    pub chain: String,
    pub symbol: String,
    pub is_send: bool, // from user's perspective. so if is_send is true, it means the user is sending the token
    pub amount: Nat,
    pub canister_id: String,
    pub block_index: Nat,
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct SolanaTransferReply {
    pub chain: String,
    pub symbol: String,
    pub is_send: bool,
    pub amount: Nat,
    pub mint_address: String,
    pub signature: String,
    pub slot: Option<u64>,
}

impl TryFrom<(u64, &StableTransfer, &StableToken)> for TransferIdReply {
    type Error = String;
    
    fn try_from((transfer_id, transfer, token): (u64, &StableTransfer, &StableToken)) -> Result<Self, Self::Error> {
        match token {
            // Case 1: The token is an IC token
            StableToken::IC(token) => match &transfer.tx_id {
                TxId::BlockIndex(block_index) => Ok(TransferIdReply {
                    transfer_id,
                    transfer: TransferReply::IC(ICTransferReply {
                        chain: IC_CHAIN.to_string(),
                        symbol: token.symbol.clone(),
                        is_send: transfer.is_send,
                        amount: transfer.amount.clone(),
                        canister_id: token.canister_id.to_string(),
                        block_index: block_index.clone(),
                    }),
                }),
                TxId::TransactionHash(hash) => {
                    // For IC tokens with TransactionHash, this likely indicates a non-standard transfer
                    // Since kong_data doesn't have Solana token type, we'll use the token's actual chain
                    // which will be "IC" for IC tokens. This is a temporary fix until proper Solana support is added.
                    Ok(TransferIdReply {
                        transfer_id,
                        transfer: TransferReply::Solana(SolanaTransferReply {
                            chain: token.chain(), // Use the actual token chain instead of hardcoding "Bitcoin"
                            symbol: token.symbol.clone(),
                            is_send: transfer.is_send,
                            amount: transfer.amount.clone(),
                            mint_address: hash.clone(),
                            signature: hash.clone(),
                            slot: None,
                        }),
                    })
                }
            },
            // Case 2: LP tokens  
            StableToken::LP(_token) => Err("LP tokens are not transferable".to_string()),
        }
    }
}
