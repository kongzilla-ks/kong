//! Transfer verification utilities
//! 
//! This module provides common functionality for verifying transfers and handling amount mismatches
//! across different operations (swap, add_liquidity, add_pool). It ensures consistent behavior
//! when the actual transfer amount on the blockchain differs from the expected amount.
//! 
//! # Amount Mismatch Handling
//! 
//! When a user initiates a transfer, they specify an amount. However, the actual amount recorded
//! on the blockchain may differ due to:
//! - Transfer fees not accounted for by the user
//! - Token contract behavior
//! - Rounding differences
//! 
//! This module handles these mismatches by:
//! 1. Recording the transfer with the actual amount to prevent reuse
//! 2. Returning a clear error message with both expected and actual amounts
//! 3. Enabling the calling code to return tokens to the user

use candid::Nat;
use crate::stable_token::stable_token::StableToken;
use crate::stable_token::token::Token;
use crate::stable_transfer::{stable_transfer::StableTransfer, transfer_map, tx_id::TxId};
use crate::stable_request::{request_map, status::StatusCode};
use super::verify_transfer::verify_transfer;

#[derive(Debug, Clone, Copy)]
pub enum TokenType {
    PayToken,
    Token0,
    Token1,
}

impl TokenType {
    fn verify_status(&self) -> StatusCode {
        match self {
            TokenType::PayToken => StatusCode::VerifyPayToken,
            TokenType::Token0 => StatusCode::VerifyToken0,
            TokenType::Token1 => StatusCode::VerifyToken1,
        }
    }
    
    fn verify_failed_status(&self) -> StatusCode {
        match self {
            TokenType::PayToken => StatusCode::VerifyPayTokenFailed,
            TokenType::Token0 => StatusCode::VerifyToken0Failed,
            TokenType::Token1 => StatusCode::VerifyToken1Failed,
        }
    }
    
    fn verify_success_status(&self) -> StatusCode {
        match self {
            TokenType::PayToken => StatusCode::VerifyPayTokenSuccess,
            TokenType::Token0 => StatusCode::VerifyToken0Success,
            TokenType::Token1 => StatusCode::VerifyToken1Success,
        }
    }
}

/// Verifies a transfer and records it in the transfer map
/// 
/// This function:
/// 1. Verifies the transfer exists on the blockchain
/// 2. Checks for duplicate transfers
/// 3. Compares the actual amount with the expected amount
/// 4. Records the transfer to prevent reuse
/// 
/// # Arguments
/// 
/// * `request_id` - The unique identifier for this request
/// * `token_type` - The type of token being verified (PayToken, Token0, or Token1)
/// * `token` - The token being transferred
/// * `tx_id` - The transaction/block ID on the blockchain
/// * `expected_amount` - The amount the user specified
/// * `ts` - The timestamp of the operation
/// 
/// # Returns
/// 
/// * `Ok(transfer_id)` - The ID of the recorded transfer
/// * `Err(message)` - Error message if verification fails or amount mismatches
/// 
/// # Amount Mismatch Behavior
/// 
/// When the actual amount differs from the expected amount:
/// 1. The transfer is still recorded with the actual amount to prevent reuse
/// 2. An error is returned with both amounts for clarity
/// 3. The calling code can then initiate a token return
pub async fn verify_and_record_transfer(
    request_id: u64,
    token_type: TokenType,
    token: &StableToken,
    tx_id: &Nat,
    expected_amount: &Nat,
    ts: u64,
) -> Result<u64, String> {
    let token_id = token.token_id();
    
    request_map::update_status(request_id, token_type.verify_status(), None);
    
    match verify_transfer(token, tx_id, expected_amount).await {
        Ok(actual_amount) => {
            // Debug: Print what we got
            eprintln!("DEBUG verify_and_record_transfer: expected_amount={}, actual_amount={}", expected_amount, actual_amount);
            
            if transfer_map::contain(token_id, tx_id) {
                let e = format!("Duplicate block id #{}", tx_id);
                request_map::update_status(request_id, token_type.verify_failed_status(), Some(&e));
                return Err(e);
            }
            
            if actual_amount != *expected_amount {
                // IMPORTANT: Record the transfer with the actual amount to prevent reuse
                let _transfer_id = transfer_map::insert(&StableTransfer {
                    transfer_id: 0,
                    request_id,
                    is_send: true,
                    amount: actual_amount.clone(),
                    token_id,
                    tx_id: TxId::BlockIndex(tx_id.clone()),
                    ts,
                });
                
                let e = format!("Transfer amount mismatch: expected {} but got {}", expected_amount, actual_amount);
                request_map::update_status(request_id, token_type.verify_failed_status(), Some(&e));
                return Err(e);
            }
            
            let transfer_id = transfer_map::insert(&StableTransfer {
                transfer_id: 0,
                request_id,
                is_send: true,
                amount: expected_amount.clone(),
                token_id,
                tx_id: TxId::BlockIndex(tx_id.clone()),
                ts,
            });
            
            request_map::update_status(request_id, token_type.verify_success_status(), None);
            Ok(transfer_id)
        }
        Err(e) => {
            request_map::update_status(request_id, token_type.verify_failed_status(), Some(&e));
            Err(e)
        }
    }
}

/// Checks if an error message indicates an amount mismatch
/// 
/// # Arguments
/// 
/// * `error` - The error message to check
/// 
/// # Returns
/// 
/// `true` if the error indicates an amount mismatch, `false` otherwise
pub fn is_amount_mismatch_error(error: &str) -> bool {
    error.contains("Transfer amount mismatch:")
}