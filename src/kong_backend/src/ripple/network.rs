use anyhow::Result;
use candid::Principal;
use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

use crate::ic::management_canister::ManagementCanister;

use super::error::RippleError;
use super::utils::base58;

pub struct RippleNetwork;

impl RippleNetwork {
    pub async fn get_public_key(canister: &Principal) -> Result<String> {
        let derivation_path = ManagementCanister::get_canister_derivation_path(canister);

        // Get the Schnorr public key - fail properly if not available
        let public_key_bytes = ManagementCanister::get_schnorr_public_key(canister, derivation_path)
            .await
            .map_err(|e| RippleError::PublicKeyRetrievalError(e.to_string()))?;

        // Step 1: SHA256(pubkey)
        let sha = Sha256::digest(public_key_bytes);

        // Step 2: RIPEMD160(SHA256(pubkey))
        let mut ripemd = Ripemd160::new();
        ripemd.update(sha);
        let account_id = ripemd.finalize(); // 20 bytes

        // Step 3: payload = version byte (0x00) + account_id
        let mut payload = Vec::with_capacity(1 + account_id.len());
        payload.push(0x00u8);
        payload.extend_from_slice(&account_id);

        // Step 4: checksum = first 4 bytes of SHA256(SHA256(payload))
        let first = Sha256::digest(&payload);
        let second = Sha256::digest(first);
        let checksum = &second[..4];

        // Step 5: final = payload + checksum
        let mut final_bytes = payload;
        final_bytes.extend_from_slice(checksum);

        Ok(base58::encode_wallet_address(&final_bytes))
    }
}
