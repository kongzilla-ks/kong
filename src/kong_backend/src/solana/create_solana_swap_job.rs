//! Module for creating Solana swap jobs for outgoing transfers
//!
//! This handles the creation of swap jobs that will be processed by kong_rpc
//! to execute Solana transactions.

use candid::Nat;
use num_traits::ToPrimitive;

use crate::ic::address::Address;
use crate::solana::stable_memory::{get_cached_solana_address, get_next_solana_swap_job_id, with_swap_job_queue_mut};
use crate::solana::swap_job::{SwapJob, SwapJobId, SwapJobStatus};
use crate::solana::transaction::builder::{SplTransferWithAtaParams, TransactionBuilder};
use crate::solana::transaction::sign::sign_transaction;
use crate::stable_token::stable_token::StableToken;

/// Creates a Solana swap job for processing an outgoing transfer
pub async fn create_solana_swap_job(
    request_id: u64,
    user_id: u32,
    receive_token: &StableToken,
    receive_amount: &Nat,
    to_address: &Address,
    ts: u64,
) -> Result<u64, String> {
    // Validate that this is a Solana token
    if let StableToken::Solana(sol_token) = receive_token {
        // Extract destination Solana address
        let destination_address = match to_address {
            Address::AccountId(account_id) => {
                // For backward compatibility, some users might still use AccountId
                // Convert the 32-byte AccountId to a base58 Solana address
                bs58::encode(account_id).into_string()
            }
            Address::PrincipalId(_) => {
                return Err("Cannot send Solana tokens to IC Principal. Please provide a Solana address.".to_string());
            }
            Address::SolanaAddress(address) => {
                // Direct Solana address - this is what we want
                address.clone()
            }
        };

        // Get Kong's Solana address
        let kong_address = get_cached_solana_address();

        // Get the job ID
        let job_id = get_next_solana_swap_job_id();

        // API boundary: Solana blockchain requires u64 amounts (max ~18.4e18 lamports)
        // This is acceptable since Solana tokens have at most 9 decimals
        let amount_u64 = receive_amount
            .0
            .to_u64()
            .ok_or("Amount too large for Solana transfer (max ~18.4e18)")?;

        // Build transaction instructions based on token type
        let instructions = if sol_token.mint_address == "11111111111111111111111111111111" {
            // Native SOL transfer
            TransactionBuilder::build_transfer_sol_transaction(
                &kong_address,
                &destination_address,
                amount_u64,
                Some(format!("Kong swap job #{}", job_id)),
            )
            .await
            .map_err(|e| format!("Failed to build SOL transfer: {}", e))?
        } else {
            // SPL token transfer with ATA creation
            let from_token_account = TransactionBuilder::derive_associated_token_account(&kong_address, &sol_token.mint_address)
                .map_err(|e| format!("Failed to derive source ATA: {}", e))?;
            let to_token_account = TransactionBuilder::derive_associated_token_account(&destination_address, &sol_token.mint_address)
                .map_err(|e| format!("Failed to derive destination ATA: {}", e))?;

            let params = SplTransferWithAtaParams {
                from_address: &kong_address,
                from_token_account: &from_token_account,
                to_wallet_address: &destination_address,
                to_token_account: &to_token_account,
                mint_address: &sol_token.mint_address,
                fee_payer: &kong_address,
                amount: amount_u64,
                memo: Some(format!("Kong swap job #{}", job_id)),
            };

            TransactionBuilder::build_transfer_spl_with_ata_transaction(params)
                .await
                .map_err(|e| format!("Failed to build SPL transfer with ATA: {}", e))?
        };

        // Sign the transaction
        let signed_tx = sign_transaction(instructions, &kong_address)
            .await
            .map_err(|e| format!("Failed to sign transaction: {}", e))?;

        // Extract signature for tracking first (before encoding)
        let tx_sig = if !signed_tx.signatures.is_empty() {
            bs58::encode(&signed_tx.signatures[0]).into_string()
        } else {
            return Err("No signature in signed transaction".to_string());
        };

        // Encode the signed transaction using proper Solana transaction format
        let encoded_tx = signed_tx.encode().map_err(|e| format!("Failed to encode transaction: {}", e))?;

        // Create the swap job using passed timestamp

        let swap_job = SwapJob::new(
            job_id,
            user_id,
            request_id,
            SwapJobStatus::Pending,
            ts,
            ts,
            encoded_tx,
            None,
            None,
            tx_sig,
        );

        // Store the job in the queue
        with_swap_job_queue_mut(|queue| {
            queue.insert(SwapJobId(job_id), swap_job);
        });

        Ok(job_id)
    } else {
        Err("Not a Solana token".to_string())
    }
}
