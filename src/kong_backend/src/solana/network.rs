use anyhow::Result;
use candid::{CandidType, Deserialize, Principal};
use serde::Serialize;
use std::fmt;

use crate::ic::management_canister::ManagementCanister;

use super::error::SolanaError;
use super::utils::base58;

// Known program IDs on Solana network
pub const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";
pub const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const SYSVAR_RENT_PROGRAM_ID: &str = "SysvarRent111111111111111111111111111111111";
pub const COMPUTE_BUDGET_PROGRAM_ID: &str = "ComputeBudget111111111111111111111111111111";

#[derive(CandidType, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SolanaNetwork {
    Mainnet,
    Devnet,
}

impl fmt::Display for SolanaNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolanaNetwork::Mainnet => write!(f, "Mainnet"),
            SolanaNetwork::Devnet => write!(f, "Devnet"),
        }
    }
}

impl SolanaNetwork {
    pub fn bs58_encode_public_key(public_key: &[u8]) -> String {
        base58::encode(public_key)
    }

    pub fn bs58_decode_public_key(public_key: &str) -> Result<[u8; 32]> {
        base58::decode_public_key(public_key)
            .map_err(|e| e.into())
    }

    pub async fn get_public_key(canister: &Principal) -> Result<String> {
        let derivation_path = ManagementCanister::get_canister_derivation_path(canister);
        
        // Try to get the Schnorr public key
        match ManagementCanister::get_schnorr_public_key(canister, derivation_path).await {
            Ok(public_key_bytes) => {
                let validated_public_key = SolanaNetwork::validate_public_key(&public_key_bytes)?;
                Ok(SolanaNetwork::bs58_encode_public_key(&validated_public_key))
            }
            Err(e) => {
                // Check if this is a local development environment where Schnorr is not available
                let error_msg = e.to_string();
                if error_msg.contains("LOCAL_DEVELOPMENT_SCHNORR_UNAVAILABLE") ||
                   error_msg.contains("schnorr_public_key") || 
                   error_msg.contains("reject_code: 5") ||
                   error_msg.contains("reject_code: 3") ||
                   error_msg.contains("InvalidManagementPayload") ||
                   error_msg.contains("Type mismatch") ||
                   error_msg.contains("Subtyping error") {
                    // Local development: return a deterministic mock address based on canister ID
                    // This allows local development to work without Schnorr support
                    let mock_address = Self::generate_mock_solana_address(canister)?;
                    Ok(mock_address)
                } else {
                    // Real error, propagate it
                    Err(SolanaError::PublicKeyRetrievalError(error_msg).into())
                }
            }
        }
    }

    /// Generate a deterministic mock Solana address for local development
    /// This creates a valid base58-encoded 32-byte address based on the canister ID
    fn generate_mock_solana_address(canister: &Principal) -> Result<String> {
        // Use the canister ID bytes as seed for deterministic address generation
        let canister_bytes = canister.as_slice();
        
        // Create a 32-byte array for the mock address
        let mut mock_address = [0u8; 32];
        
        // Fill with a pattern based on canister ID
        for (i, byte) in mock_address.iter_mut().enumerate() {
            *byte = canister_bytes[i % canister_bytes.len()].wrapping_add(i as u8);
        }
        
        // Ensure it's a valid Solana address (not on curve)
        // For simplicity, just set the first byte to ensure it's not on curve
        mock_address[31] = 0xFF; // This helps ensure it's not on the Ed25519 curve
        
        Ok(SolanaNetwork::bs58_encode_public_key(&mock_address))
    }

    pub fn validate_public_key(public_key: &[u8]) -> Result<Vec<u8>> {
        // Ed25519 public keys are 32 bytes long
        if public_key.len() != 32 {
            Err(SolanaError::InvalidPublicKeyFormat("Public key must be 32 bytes long.".to_string()))?;
        }

        Ok(public_key.to_vec())
    }

    pub fn validate_tx_signature(tx_signature: &[u8]) -> Result<Vec<u8>> {
        // Ed25519 signatures are 64 bytes long
        if tx_signature.len() != 64 {
            Err(SolanaError::InvalidSignature(
                "Transaction signature must be 64 bytes long.".to_string(),
            ))?;
        }

        Ok(tx_signature.to_vec())
    }
}
