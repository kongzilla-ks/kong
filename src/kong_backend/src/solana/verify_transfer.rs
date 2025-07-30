//! Solana transfer verification module
//! Handles verification of Solana/SPL token transfers by checking signatures and on-chain transactions

use anyhow::Result;
use candid::Nat;
use num_traits::ToPrimitive;
use std::str::FromStr;

use crate::ic::network::ICNetwork;
use crate::stable_memory::get_solana_transaction;
use crate::solana::kong_rpc::transaction_notification::{TransactionNotification, TransactionNotificationStatus};
use super::error::SolanaError;
use super::sdk::offchain_message::OffchainMessage;
use super::sdk::pubkey::Pubkey;
use super::sdk::signature::Signature as SolanaSignature;

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
    // Get transaction and parse metadata once
    let transaction = get_solana_transaction(tx_id).ok_or_else(|| {
        format!(
            "Solana transaction {} not found. Make sure kong_rpc has processed this transaction.",
            tx_id
        )
    })?;

    let metadata_json = transaction.metadata.as_ref()
        .ok_or("Transaction metadata is missing")?;
    
    let metadata: serde_json::Value = serde_json::from_str(metadata_json)
        .map_err(|e| format!("Failed to parse transaction metadata: {}", e))?;

    // Extract sender from the parsed metadata
    let sender_pubkey = extract_solana_sender_from_metadata(&metadata, is_spl_token)?;
    
    // Verify the signature on the canonical message
    // The message should already contain the pay_address that matches the sender
    verify_canonical_message(canonical_message, &sender_pubkey, signature)
        .map_err(|e| format!("Signature verification failed: {}", e))?;
    
    // Verify the actual Solana transaction on-chain using the parsed metadata
    verify_solana_transaction_with_metadata(&transaction, &metadata, &sender_pubkey, amount, is_spl_token)?;
    
    Ok(SolanaVerificationResult {
        tx_signature: tx_id.to_string(),
        from_address: sender_pubkey,
        amount: amount.clone(),
    })
}

/// Extract sender from parsed metadata based on token type
fn extract_solana_sender_from_metadata(metadata: &serde_json::Value, is_spl_token: bool) -> Result<String, String> {
    // Extract sender based on token type
    let sender = if is_spl_token {
        // For SPL tokens: use "authority" or "sender_wallet" (the actual wallet that signed)
        metadata
            .get("authority")
            .or_else(|| metadata.get("sender_wallet"))
            .and_then(|v| v.as_str())
            .ok_or("SPL transaction metadata missing authority/sender_wallet information")?
    } else {
        // For native SOL: use "sender" (the wallet address)
        metadata
            .get("sender")
            .and_then(|v| v.as_str())
            .ok_or("SOL transaction metadata missing sender information")?
    };

    Ok(sender.to_string())
}

/// Verify a Solana transaction exists and matches expected parameters using pre-parsed metadata
fn verify_solana_transaction_with_metadata(
    transaction: &TransactionNotification,
    metadata: &serde_json::Value,
    expected_sender: &str,
    expected_amount: &Nat,
    is_spl_token: bool,
) -> Result<(), String> {
    // Check transaction status
    match transaction.status {
        TransactionNotificationStatus::Confirmed | TransactionNotificationStatus::Finalized => {} // Good statuses
        TransactionNotificationStatus::Failed => return Err("Solana transaction failed".to_string()),
        TransactionNotificationStatus::Processed => return Err("Solana transaction still being processed".to_string()),
    }

    // Verify blockchain timestamp freshness (5 minute window)
    // Solana transactions must be recent to prevent replay attacks with old transactions
    const MAX_TRANSACTION_AGE_MS: u64 = 300_000; // 5 minutes in milliseconds

    // Extract blockTime from metadata (unix timestamp in seconds from Solana RPC)
    let block_time = metadata
        .get("blockTime")
        .and_then(|v| v.as_u64())
        .ok_or("Solana transaction missing transaction blockTime")?;

    // Convert to milliseconds and check age
    let tx_timestamp_ms = block_time * 1000;
    let current_time_ms = ICNetwork::get_time() / 1_000_000; // Convert from nanoseconds
    let age_ms = current_time_ms.saturating_sub(tx_timestamp_ms);

    if age_ms > MAX_TRANSACTION_AGE_MS {
        return Err(format!(
            "Solana transaction is too old: {} minutes ago. Transactions must be less than {} minutes old.",
            age_ms / 60_000,
            MAX_TRANSACTION_AGE_MS / 60_000
        ));
    }

    // Verify transaction details using the already parsed metadata
    
    // Check sender matches based on token type
    let actual_sender = if is_spl_token {
        // For SPL tokens: use "authority" or "sender_wallet"
        metadata
            .get("authority")
            .or_else(|| metadata.get("sender_wallet"))
            .and_then(|v| v.as_str())
            .ok_or("SPL transaction metadata missing authority/sender_wallet information")?
    } else {
        // For native SOL: use "sender"
        metadata
            .get("sender")
            .and_then(|v| v.as_str())
            .ok_or("SOL transaction metadata missing sender information")?
    };

    if actual_sender != expected_sender {
        return Err(format!(
            "Transaction sender mismatch. Expected: {}, Got: {}",
            expected_sender, actual_sender
        ));
    }

    // Check amount matches
    let actual_amount = metadata
        .get("amount")
        .and_then(|v| v.as_u64())
        .ok_or("Transaction metadata missing amount")?;

    // API boundary: Solana returns u64 amounts, so we must convert for comparison
    let expected_amount_u64 = expected_amount
        .0
        .to_u64()
        .ok_or("Expected amount too large for Solana (max ~18.4e18)")?;
    if actual_amount != expected_amount_u64 {
        return Err(format!(
            "Transaction amount mismatch. Expected: {}, Got: {}",
            expected_amount_u64, actual_amount
        ));
    }

    Ok(())
}

fn verify_raw_message(message: &str, pubkey: &Pubkey, signature: &SolanaSignature) -> Result<()> {
    let verify_key = ed25519_dalek::VerifyingKey::from_bytes(&pubkey.to_bytes())?;
    let ed25519_signature = signature.as_ref().try_into()?;
    verify_key.verify_strict(message.as_bytes(), &ed25519_signature)?;
    Ok(())
}

/// Verify a signature against a canonical message
///
/// This unified flow accepts both raw signatures and Solana CLI prefixed signatures.
/// It tries raw signature verification first (most common in production),
/// then falls back to prefixed signature verification if needed.
///
/// Note: The Solana CLI adds a "\xffsolana offchain" prefix when signing messages.
pub fn verify_canonical_message(message: &str, public_key: &str, signature: &str) -> Result<()> {
    let pubkey = Pubkey::from_str(public_key)?;
    let sig = SolanaSignature::from_str(signature)?;

    // Try raw signature first (most common case)
    // Wallets like Phantom and Solflare sign the raw message directly
    if verify_raw_message(message, &pubkey, &sig).is_ok() {
        return Ok(());
    }

    // If raw verification fails, try with Solana's offchain message prefix
    // The Solana CLI and some developer tools add this prefix when signing
    let offchain_message = OffchainMessage::new(0, message.as_bytes())
        .map_err(|e| SolanaError::InvalidMessageSigning(format!("Failed to create offchain message: {}", e)))?;

    offchain_message
        .verify(&pubkey, &sig)
        .map_err(|e| SolanaError::InvalidMessageSigning(format!("Invalid signature. Error: {}", e)))?;

    Ok(())
}

/// Extract sender from a Solana transaction based on token type
/// This is a compatibility wrapper that maintains the single metadata parsing goal
pub async fn extract_solana_sender_from_transaction(tx_signature: &str, is_spl_token: bool) -> Result<String, String> {
    let transaction = get_solana_transaction(tx_signature).ok_or_else(|| {
        format!(
            "Solana transaction {} not found. Make sure kong_rpc has processed this transaction.",
            tx_signature
        )
    })?;

    let metadata_json = transaction.metadata.as_ref()
        .ok_or("Transaction metadata is missing")?;
    
    let metadata: serde_json::Value = serde_json::from_str(metadata_json)
        .map_err(|e| format!("Failed to parse transaction metadata: {}", e))?;

    // Use the existing metadata extraction function
    extract_solana_sender_from_metadata(&metadata, is_spl_token)
}

