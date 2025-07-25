use anyhow::Result;
use std::str::FromStr;

use super::error::SolanaError;
use super::sdk::offchain_message::OffchainMessage;
use super::sdk::pubkey::Pubkey;
use super::sdk::signature::Signature as SolanaSignature;

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
