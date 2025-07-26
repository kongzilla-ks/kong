//! Solana transfer verification module
//! Handles verification of Solana/SPL token transfers by checking signatures and on-chain transactions

use candid::Nat;

use super::payment_verification::{extract_solana_sender_from_transaction, verify_solana_transaction};
use super::signature_verification::verify_canonical_message;

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
/// * `canonical_message` - The message that was signed
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
    // Extract sender from the Solana transaction
    let sender_pubkey = extract_solana_sender_from_transaction(tx_id, is_spl_token).await?;
    
    // Verify the signature on the canonical message
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