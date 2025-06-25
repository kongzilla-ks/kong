//! Liquidity-specific payment verification module
//! Handles verification of incoming payments for liquidity provision

use anyhow::Result;
use candid::{Nat, Principal};

use crate::stable_token::stable_token::StableToken;
use crate::stable_transfer::tx_id::TxId;
use crate::swap::verify_canonical_message::verify_canonical_message;
use crate::solana::payment_verification::{
    extract_solana_sender_from_transaction,
    verify_solana_transaction,
    verify_solana_timestamp_freshness,
};

use super::message_builder::CanonicalAddLiquidityMessage;
use super::add_liquidity_args::AddLiquidityArgs;

/// Result of liquidity payment verification
pub enum LiquidityPaymentVerification {
    /// Solana/SPL payment verified via signature and transaction
    SolanaPayment {
        tx_signature: String,
        from_address: String,
        amount: Nat,
    },
}

/// Liquidity payment verifier that handles cross-chain liquidity provision payments
pub struct LiquidityPaymentVerifier {
    _caller: Principal,
}

impl LiquidityPaymentVerifier {
    /// Create a new LiquidityPaymentVerifier instance
    pub fn new(caller: Principal) -> Self {
        Self { _caller: caller }
    }

    /// Verify payment for liquidity provision with signature
    pub async fn verify_liquidity_payment(
        &self,
        args: &AddLiquidityArgs,
        token: &StableToken,
        amount: &Nat,
        tx_id: &TxId,
        signature: &str,
    ) -> Result<LiquidityPaymentVerification, String> {
        // Only Solana tokens are supported for cross-chain liquidity provision
        match token {
            StableToken::Solana(sol_token) => {
                verify_solana_liquidity_payment(args, amount, tx_id, signature, sol_token).await
            }
            _ => {
                Err("Only Solana tokens require signature verification for liquidity provision".to_string())
            }
        }
    }
}

/// Verify Solana payment for liquidity provision
async fn verify_solana_liquidity_payment(
    args: &AddLiquidityArgs,
    amount: &Nat,
    tx_id: &TxId,
    signature: &str,
    sol_token: &crate::stable_token::solana_token::SolanaToken,
) -> Result<LiquidityPaymentVerification, String> {
    // Extract transaction signature
    let tx_signature_str = match tx_id {
        TxId::TransactionId(hash) => hash.clone(),
        TxId::BlockIndex(_) => return Err("BlockIndex not supported for Solana transactions".to_string()),
    };

    // Check if this is an SPL token (not native SOL)
    let is_spl_token = sol_token.is_spl_token;
    
    // Extract sender from the transaction
    let sender_pubkey = extract_solana_sender_from_transaction(&tx_signature_str, is_spl_token).await?;
    
    // Create canonical liquidity message and verify signature
    let canonical_message = CanonicalAddLiquidityMessage::from_add_liquidity_args(args);
    let message_to_verify = canonical_message.to_signing_message();
    
    verify_canonical_message(&message_to_verify, &sender_pubkey, signature)
        .map_err(|e| format!("Liquidity signature verification failed: {}", e))?;
    
    // Check timestamp freshness (5 minutes)
    verify_solana_timestamp_freshness(args.timestamp)?;

    // Verify the actual Solana transaction
    verify_solana_transaction(&tx_signature_str, &sender_pubkey, amount, is_spl_token).await?;
    
    Ok(LiquidityPaymentVerification::SolanaPayment {
        tx_signature: tx_signature_str,
        from_address: sender_pubkey,
        amount: amount.clone(),
    })
}

