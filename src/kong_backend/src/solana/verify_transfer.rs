//! Solana transfer verification module
//! Handles verification of Solana/SPL token transfers by checking signatures and on-chain transactions

use candid::Nat;
use ic_cdk::futures::spawn;
use ic_cdk_timers::set_timer;
use std::time::Duration;

use super::payment_verification::{extract_solana_sender_from_transaction, verify_solana_transaction};
use super::signature_verification::verify_canonical_message;
use crate::ic::network::ICNetwork;
use crate::stable_request::{request::Request, request_map, stable_request::StableRequest, status::StatusCode};
use crate::stable_user::user_map;

/// Result of Solana transfer verification
#[derive(Debug, Clone)]
pub struct SolanaVerificationResult {
    pub tx_signature: String,
    pub from_address: String,
    pub amount: Nat,
}

/// Verify a Solana transfer by checking signature and on-chain transaction
///
/// # Arguments
/// * `tx_id` - The transaction signature/ID
/// * `signature` - The Ed25519 signature on the canonical message
/// * `amount` - The expected transfer amount
/// * `canonical_message` - The message that was signed (without sender, as it will be extracted)
/// * `is_spl_token` - Whether this is an SPL token (true) or native SOL (false)
///
/// # Returns
/// * `Ok(SolanaVerificationResult)` - If verification succeeds
/// * `Err(String)` - If verification fails with reason
pub async fn verify_transfer(
    tx_id: &str,
    signature: &str,
    amount: &Nat,
    canonical_message: &str,
    is_spl_token: bool,
) -> Result<SolanaVerificationResult, String> {
    // Extract sender from the Solana transaction (for security, we trust only the blockchain)
    let sender_pubkey = extract_solana_sender_from_transaction(tx_id, is_spl_token).await?;
    
    // Verify the signature on the canonical message
    // The message should already contain the pay_address that matches the sender
    verify_canonical_message(canonical_message, &sender_pubkey, signature)
        .map_err(|e| format!("Signature verification failed: {}", e))?;
    
    // Verify the actual Solana transaction on-chain
    verify_solana_transaction(tx_id, &sender_pubkey, amount, is_spl_token).await?;
    
    Ok(SolanaVerificationResult {
        tx_signature: tx_id.to_string(),
        from_address: sender_pubkey,
        amount: amount.clone(),
    })
}

/// Asynchronously verify a Solana transfer with retry mechanism (delay until transaction is found)
///
/// # Arguments
/// * `tx_id` - The transaction signature/ID
/// * `signature` - The Ed25519 signature on the canonical message
/// * `amount` - The expected transfer amount
/// * `canonical_message` - The message that was signed (without sender, as it will be extracted)
/// * `is_spl_token` - Whether this is an SPL token (true) or native SOL (false)
/// * `max_retries` - Maximum number of retry attempts
/// * `delay_ms` - Delay between retries in milliseconds
///
/// # Returns
/// * `Ok(u64)` - Request ID for tracking verification progress
/// * `Err(String)` - If verification setup fails
pub async fn verify_transfer_async(
    tx_id: &str,
    signature: &str,
    amount: &Nat,
    canonical_message: &str,
    is_spl_token: bool,
    max_retries: u32,
    delay_ms: u64,
) -> Result<u64, String> {
    // Create user if not exists and setup request tracking
    let user_id = user_map::insert(None)?;
    let ts = ICNetwork::get_time();
    
    // Create a request for async Solana verification tracking
    let verification_request = format!(
        "tx_id: {}, amount: {}, is_spl: {}",
        tx_id, amount, is_spl_token
    );
    
    // Create request for tracking
    let request = Request::SolanaVerifyAsync(verification_request);
    let request_id = request_map::insert(&StableRequest::new(user_id, &request, ts));
    
    // Clone data for async closure
    let tx_id_clone = tx_id.to_string();
    let signature_clone = signature.to_string();
    let amount_clone = amount.clone();
    let canonical_message_clone = canonical_message.to_string();
    
    // Start async verification process
    spawn(async move {
        verify_with_retry(
            request_id,
            &tx_id_clone,
            &signature_clone,
            &amount_clone,
            &canonical_message_clone,
            is_spl_token,
            max_retries,
            delay_ms,
            0, // current attempt
        ).await;
    });
    
    // Update initial status and return request_id for tracking
    request_map::update_status(request_id, StatusCode::Start, None);
    Ok(request_id)
}

async fn verify_with_retry(
    request_id: u64,
    tx_id: &str,
    signature: &str,
    amount: &Nat,
    canonical_message: &str,
    is_spl_token: bool,
    max_retries: u32,
    delay_ms: u64,
    current_attempt: u32,
) {
    if current_attempt >= max_retries {
        request_map::update_status(
            request_id,
            StatusCode::Failed,
            Some(&format!("Maximum retries ({}) exceeded for transaction {}", max_retries, tx_id)),
        );
        return;
    }

    // Update status to show verification attempt
    request_map::update_status(
        request_id,
        StatusCode::VerifyPayToken,
        Some(&format!("Attempt {} of {}", current_attempt + 1, max_retries)),
    );

    // Try to verify the transaction
    match verify_transfer(tx_id, signature, amount, canonical_message, is_spl_token).await {
        Ok(_result) => {
            // Success - update status and store result
            request_map::update_status(request_id, StatusCode::Success, Some("Transaction verified successfully"));
            // Note: Consider storing the verification result if needed
        }
        Err(error) => {
            // Check if this is a "transaction not found" error that should be retried
            if error.contains("not found") || error.contains("Make sure kong_rpc has processed") {
                // Transaction not yet available, schedule retry
                let tx_id_clone = tx_id.to_string();
                let signature_clone = signature.to_string();
                let amount_clone = amount.clone();
                let canonical_message_clone = canonical_message.to_string();
                let next_attempt = current_attempt + 1;
                
                set_timer(Duration::from_millis(delay_ms), move || {
                    spawn(async move {
                        verify_with_retry(
                            request_id,
                            &tx_id_clone,
                            &signature_clone,
                            &amount_clone,
                            &canonical_message_clone,
                            is_spl_token,
                            max_retries,
                            delay_ms,
                            next_attempt,
                        ).await;
                    });
                });
            } else {
                // Permanent error - don't retry
                request_map::update_status(
                    request_id,
                    StatusCode::Failed,
                    Some(&format!("Verification failed: {}", error)),
                );
            }
        }
    }
}