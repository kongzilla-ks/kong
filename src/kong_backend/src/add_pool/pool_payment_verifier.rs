//! Pool-specific payment verification module
//! Handles verification of incoming payments for pool creation

use anyhow::Result;
use candid::{Nat, Principal};

use crate::stable_token::stable_token::StableToken;
use crate::stable_transfer::tx_id::TxId;
use crate::solana::signature_verification::verify_canonical_message;
use crate::solana::payment_verification::{
    extract_solana_sender_from_transaction,
    verify_solana_transaction,
};

use super::message_builder::CanonicalAddPoolMessage;
use super::add_pool_args::AddPoolArgs;

/// Result of pool payment verification
pub enum PoolPaymentVerification {
    /// Solana/SPL payment verified via signature and transaction
    SolanaPayment {
        tx_signature: String,
        from_address: String,
        amount: Nat,
    },
}

/// Pool payment verifier that handles cross-chain pool creation payments
pub struct PoolPaymentVerifier {
    _caller: Principal,
}

impl PoolPaymentVerifier {
    /// Create a new PoolPaymentVerifier instance
    pub fn new(caller: Principal) -> Self {
        Self { _caller: caller }
    }

    /// Verify payment for pool creation with signature
    pub async fn verify_pool_payment(
        &self,
        args: &AddPoolArgs,
        token: &StableToken,
        amount: &Nat,
        tx_id: &TxId,
        signature: &str,
    ) -> Result<PoolPaymentVerification, String> {
        // Only Solana tokens are supported for cross-chain pool creation
        match token {
            StableToken::Solana(sol_token) => {
                verify_solana_pool_payment(args, amount, tx_id, signature, sol_token).await
            }
            StableToken::IC(_) => {
                Err("IC tokens don't require signature verification for pool creation".to_string())
            }
            StableToken::LP(_) => {
                Err("LP tokens cannot be used for pool creation".to_string())
            }
        }
    }
}

/// Verify Solana payment for pool creation
async fn verify_solana_pool_payment(
    args: &AddPoolArgs,
    amount: &Nat,
    tx_id: &TxId,
    signature: &str,
    sol_token: &crate::stable_token::solana_token::SolanaToken,
) -> Result<PoolPaymentVerification, String> {
    
    // Extract transaction signature
    let tx_signature_str = match tx_id {
        TxId::TransactionId(hash) => hash.clone(),
        TxId::BlockIndex(_) => return Err("BlockIndex not supported for Solana transactions".to_string()),
    };

    // Check if this is an SPL token (not native SOL)
    let is_spl_token = sol_token.is_spl_token;
    
    // Extract sender from the transaction
    let sender_pubkey = extract_solana_sender_from_transaction(&tx_signature_str, is_spl_token).await?;
    
    // Create canonical pool message and verify signature
    let canonical_message = CanonicalAddPoolMessage::from_add_pool_args(args);
    let message_to_verify = canonical_message.to_signing_message();
    
    
    verify_canonical_message(&message_to_verify, &sender_pubkey, signature)
        .map_err(|e| format!("Pool signature verification failed: {}", e))?;

    // Verify the actual Solana transaction
    verify_solana_transaction(&tx_signature_str, &sender_pubkey, amount, is_spl_token).await?;
    
    Ok(PoolPaymentVerification::SolanaPayment {
        tx_signature: tx_signature_str,
        from_address: sender_pubkey,
        amount: amount.clone(),
    })
}

