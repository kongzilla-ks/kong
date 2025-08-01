//! Transaction signing for Solana
//!
//! This module handles signing of Solana transactions using IC's Schnorr signatures.

use anyhow::Result;

use crate::kong_backend::KongBackend;
use crate::solana::error::SolanaError;
use crate::solana::sdk::instruction::Instruction;

use super::serialize::serialize_message;

/// Signed transaction ready for submission
#[derive(Debug, Clone)]
pub struct SignedTransaction {
    pub signatures: Vec<Vec<u8>>,
    pub message: Vec<u8>, // Serialized message
}

impl SignedTransaction {
    /// Encode the signed transaction for Solana network submission
    ///
    /// Returns a base58-encoded transaction in the format expected by Solana RPC:
    /// - signature count (1 byte)
    /// - signature (64 bytes, padded/truncated as needed)  
    /// - message data
    pub fn encode(&self) -> Result<String> {
        // Create a buffer for the serialized transaction
        let mut transaction_bytes = Vec::new();

        // 1. Add signature count as a single byte (we only have one signature)
        transaction_bytes.push(1);

        // 2. Add the signature (64 bytes for Ed25519)
        if self.signatures.is_empty() {
            return Err(SolanaError::SigningError("No signatures found".to_string()).into());
        }

        let signature = &self.signatures[0];
        let signature_bytes = match signature.len().cmp(&64) {
            std::cmp::Ordering::Less => {
                // Pad with zeros if too short
                let mut padded = signature.clone();
                padded.resize(64, 0);
                padded
            }
            std::cmp::Ordering::Greater => {
                // Truncate if too long
                signature[0..64].to_vec()
            }
            std::cmp::Ordering::Equal => {
                // Use as is if exactly 64 bytes
                signature.clone()
            }
        };
        transaction_bytes.extend_from_slice(&signature_bytes);

        // 3. Add the message data
        transaction_bytes.extend_from_slice(&self.message);

        // Base58 encode the transaction
        let encoded = bs58::encode(transaction_bytes).into_string();

        Ok(encoded)
    }
}

/// Sign transaction instructions
pub async fn sign_transaction(instructions: Vec<Instruction>, payer: &str) -> Result<SignedTransaction> {
    // Serialize the message
    let message_bytes = serialize_message(instructions, payer).await?;

    // Sign with Schnorr
    let signature = KongBackend::sign_with_schnorr(&message_bytes)
        .await
        .map_err(|e| SolanaError::SigningError(e.to_string()))?;

    Ok(SignedTransaction {
        signatures: vec![signature],
        message: message_bytes,
    })
}

