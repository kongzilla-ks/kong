use anyhow::Result;
use candid::Principal;

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

pub struct SolanaNetwork;

impl SolanaNetwork {
    pub fn bs58_encode_public_key(public_key: &[u8]) -> String {
        base58::encode(public_key)
    }

    pub fn bs58_decode_public_key(public_key: &str) -> Result<[u8; 32]> {
        base58::decode_public_key(public_key).map_err(Into::into)
    }

    pub async fn get_public_key(canister: &Principal) -> Result<String> {
        let derivation_path = ManagementCanister::get_canister_derivation_path(canister);

        // Get the Schnorr public key - fail properly if not available
        let public_key_bytes = ManagementCanister::get_schnorr_public_key(canister, derivation_path)
            .await
            .map_err(|e| SolanaError::PublicKeyRetrievalError(e.to_string()))?;

        let validated_public_key = SolanaNetwork::validate_public_key(&public_key_bytes)?;
        Ok(SolanaNetwork::bs58_encode_public_key(&validated_public_key))
    }

    pub fn validate_public_key(public_key: &[u8]) -> Result<Vec<u8>> {
        if public_key.len() == 32 {
            Ok(public_key.to_vec())
        } else {
            Err(SolanaError::InvalidPublicKeyFormat("Public key must be 32 bytes long.".to_string()).into())
        }
    }

}
