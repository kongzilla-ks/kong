use anyhow::Result;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use std::str::FromStr;

use super::error::SolanaError;
use super::network::SolanaNetwork;
use super::sdk::offchain_message::OffchainMessage;
use super::sdk::pubkey::Pubkey;
use super::sdk::signature::Signature as SolanaSignature;

/// Verify an Ed25519 signature for a message
pub fn verify_ed25519_signature(message: &[u8], signature: &[u8], public_key: &str) -> Result<bool> {
    let public_key_bytes = SolanaNetwork::bs58_decode_public_key(public_key)?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|_| SolanaError::InvalidPublicKeyFormat("Invalid Ed25519 public key".to_string()))?;
    if signature.len() != 64 {
        return Err(SolanaError::InvalidSignature("Signature must be 64 bytes".to_string()).into());
    }

    // Create signature from bytes - handle conversion error properly
    let signature_array: [u8; 64] = signature
        .try_into()
        .map_err(|_| SolanaError::InvalidSignature("Failed to convert signature to 64-byte array".to_string()))?;
    let signature = Signature::from_bytes(&signature_array);

    Ok(verifying_key.verify(message, &signature).is_ok())
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
