//! Payment verification module
//! Handles verification of incoming payments for swaps

use anyhow::Result;
use candid::{Nat, Principal};
use num_traits::ToPrimitive;

use crate::ic::network::ICNetwork;
use crate::ic::verify_transfer::verify_transfer;
use crate::stable_transfer::tx_id::TxId;
use crate::stable_token::stable_token::StableToken;
use crate::stable_memory::get_solana_transaction;

use super::swap_args::SwapArgs;
use super::message_builder::CanonicalSwapMessage;
use super::verify_canonical_message::verify_canonical_message;

/// Result of payment verification
pub enum PaymentVerification {
    /// ICP payment verified via transfer
    IcpPayment {
        block_index: Nat,
        from_principal: Principal,
        amount: Nat,
    },
    /// Solana/SPL payment verified via signature and transaction
    SolanaPayment {
        tx_signature: String,
        from_address: String,
        amount: Nat,
    },
}

/// Payment verifier that handles both ICP and Solana payments
pub struct PaymentVerifier {
    caller: Principal,
}

impl PaymentVerifier {
    /// Create a new PaymentVerifier instance
    pub fn new(caller: Principal) -> Self {
        Self { caller }
    }

    /// Verify and process payment for a swap
    pub async fn verify_payment(
        &self,
        args: &SwapArgs,
        pay_token: &StableToken,
        pay_amount: &Nat,
    ) -> Result<PaymentVerification, String> {
        // Payment verification based on token type
        match pay_token {
            StableToken::IC(_) => {
                // IC tokens use standard transfer verification, no signature needed
                self.verify_ic_payment(args, pay_token, pay_amount).await
            }
            StableToken::Solana(sol_token) => {
                // Solana tokens require signature verification
                verify_solana_payment(args, pay_amount, sol_token).await
            }
            StableToken::LP(_) => {
                Err("LP tokens cannot be used as payment".to_string())
            }
        }
    }

    /// Verify IC token payment by checking the transfer
    async fn verify_ic_payment(
        &self,
        args: &SwapArgs,
        pay_token: &StableToken,
        pay_amount: &Nat,
    ) -> Result<PaymentVerification, String> {
        let pay_tx_id = args.pay_tx_id.as_ref()
            .ok_or_else(|| "Transaction ID (pay_tx_id) is required for token payments".to_string())?;

        let block_index = match pay_tx_id {
            TxId::BlockIndex(index) => index.clone(),
            TxId::TransactionId(_) => return Err("IC tokens require BlockIndex, not TransactionId".to_string()),
        };

        // Verify the transfer on the token's ledger
        verify_transfer(pay_token, &block_index, pay_amount).await
            .map_err(|e| format!("Transfer verification failed: {}", e))?;

        Ok(PaymentVerification::IcpPayment {
            block_index,
            from_principal: self.caller,
            amount: pay_amount.clone(),
        })
    }
}

/// Verify Solana payment by checking signature and transaction data
async fn verify_solana_payment(args: &SwapArgs, pay_amount: &Nat, sol_token: &crate::stable_token::solana_token::SolanaToken) -> Result<PaymentVerification, String> {
    // Solana tokens require signature
    let signature = args.signature.as_ref()
        .ok_or_else(|| "Signature is required for Solana/SPL tokens".to_string())?;
    
    // Get transaction ID
    let tx_id = args.pay_tx_id.as_ref()
        .ok_or_else(|| {
            "Transaction ID (pay_tx_id) is required for Solana/SPL payments. Please transfer tokens directly and provide the transaction signature.".to_string()
        })?;

    let tx_signature_str = match tx_id {
        TxId::TransactionId(hash) => hash.clone(),
        TxId::BlockIndex(_) => return Err("BlockIndex not supported for Solana transactions".to_string()),
    };

    // Check if this is an SPL token (not native SOL)
    let is_spl_token = sol_token.is_spl_token;
    
    // First, extract the sender from the transaction
    ICNetwork::info_log(&format!("Extracting sender from transaction: {} (is_spl_token: {})", tx_signature_str, is_spl_token));
    let sender_pubkey = extract_sender_from_transaction(&tx_signature_str, is_spl_token).await?;
    ICNetwork::info_log(&format!("Extracted sender: {}", sender_pubkey));
    
    // Create canonical message with extracted sender and verify signature
    // TODO: Important integration note for Solana swaps:
    // The canonical message uses token symbols from SwapArgs (e.g., "SOL", "ksUSDT")
    // NOT the full chain-prefixed format (e.g., "SOL.11111111...", "IC.zdzgz-siaaa...")
    // 
    // Integrators must sign the message with the exact format:
    // - pay_token: "SOL" (not "SOL.11111111111111111111111111111111")
    // - receive_token: "ksUSDT" (not "IC.zdzgz-siaaa-aaaar-qaiba-cai")
    // - pay_address: The actual Solana wallet address that sent the transaction
    // 
    // The pay_address is extracted from the Solana transaction by kong_rpc,
    // so the signature must match this extracted address exactly.
    // 
    // Future improvement: Consider accepting both formats or providing a 
    // signature helper endpoint that returns the exact message to sign.
    ICNetwork::info_log(&format!("DEBUG: Creating canonical message from SwapArgs: {:?}", args));
    let canonical_message = CanonicalSwapMessage::from_swap_args(args)
        .with_sender(sender_pubkey.clone());
    let message_to_verify = canonical_message.to_signing_message();
    ICNetwork::info_log(&format!("DEBUG: Backend canonical message: {:?}", canonical_message));
    ICNetwork::info_log(&format!("DEBUG: Backend message to verify: {}", message_to_verify));
    ICNetwork::info_log(&format!("DEBUG: Extracted sender from transaction: {}", sender_pubkey));
    ICNetwork::info_log(&format!("DEBUG: Signature to verify: {}", signature));
    
    verify_canonical_message(&message_to_verify, &sender_pubkey, signature)
        .map_err(|e| format!("Signature verification failed: {}", e))?;
    
    // Check timestamp freshness
    let current_time_ms = ICNetwork::get_time() / 1_000_000;
    let message_timestamp = args.timestamp.ok_or("Timestamp required for Solana payments")?;
    let age_ms = current_time_ms.saturating_sub(message_timestamp);
    if age_ms > 300_000 {
        return Err(format!("Signature timestamp too old: {} ms", age_ms));
    }

    // Verify the actual Solana transaction from storage
    verify_solana_transaction(&tx_signature_str, &sender_pubkey, pay_amount, is_spl_token).await?;
    
    Ok(PaymentVerification::SolanaPayment {
        tx_signature: tx_signature_str,
        from_address: sender_pubkey,
        amount: pay_amount.clone(),
    })
}

/// Extract sender public key from a Solana transaction
async fn extract_sender_from_transaction(tx_signature: &str, is_spl_token: bool) -> Result<String, String> {
    let transaction = get_solana_transaction(tx_signature.to_string())
        .ok_or_else(|| format!("Solana transaction {} not found. Make sure kong_rpc has processed this transaction.", tx_signature))?;
    
    // Parse metadata to extract sender
    if let Some(metadata_json) = &transaction.metadata {
        let metadata: serde_json::Value = serde_json::from_str(metadata_json)
            .map_err(|e| format!("Failed to parse transaction metadata: {}", e))?;
        
        // Extract sender based on token type
        let sender = if is_spl_token {
            // For SPL tokens: use "authority" or "sender_wallet" (the actual wallet that signed)
            metadata.get("authority")
                .or_else(|| metadata.get("sender_wallet"))
                .and_then(|v| v.as_str())
                .ok_or("SPL transaction metadata missing authority/sender_wallet information")?
        } else {
            // For native SOL: use "sender" (the wallet address)
            metadata.get("sender")
                .and_then(|v| v.as_str())
                .ok_or("SOL transaction metadata missing sender information")?
        };
        
        ICNetwork::info_log(&format!("DEBUG: Extracted sender: {} (is_spl_token: {})", sender, is_spl_token));
        Ok(sender.to_string())
    } else {
        Err("Transaction metadata is missing".to_string())
    }
}

/// Verify a Solana transaction exists and matches expected parameters
async fn verify_solana_transaction(
    tx_signature: &str,
    expected_sender: &str,
    expected_amount: &Nat,
    is_spl_token: bool,
) -> Result<(), String> {
    let transaction = get_solana_transaction(tx_signature.to_string())
        .ok_or_else(|| format!("Solana transaction {} not found. Make sure kong_rpc has processed this transaction.", tx_signature))?;
    
    // Check transaction status
    match transaction.status.as_str() {
        "confirmed" | "finalized" => {}, // Good statuses
        "failed" => return Err(format!("Solana transaction {} failed", tx_signature)),
        status => return Err(format!("Solana transaction {} has unexpected status: {}", tx_signature, status)),
    }
    
    // Parse metadata to verify transaction details
    if let Some(metadata_json) = &transaction.metadata {
        let metadata: serde_json::Value = serde_json::from_str(metadata_json)
            .map_err(|e| format!("Failed to parse transaction metadata: {}", e))?;
        
        // Check sender matches based on token type
        let actual_sender = if is_spl_token {
            // For SPL tokens: use "authority" or "sender_wallet"
            metadata.get("authority")
                .or_else(|| metadata.get("sender_wallet"))
                .and_then(|v| v.as_str())
                .ok_or("SPL transaction metadata missing authority/sender_wallet information")?
        } else {
            // For native SOL: use "sender"
            metadata.get("sender")
                .and_then(|v| v.as_str())
                .ok_or("SOL transaction metadata missing sender information")?
        };
            
        if actual_sender != expected_sender {
            return Err(format!(
                "Transaction sender mismatch. Expected: {}, Got: {}",
                expected_sender, actual_sender
            ));
        }
        
        // Check amount matches
        let actual_amount = metadata.get("amount")
            .and_then(|v| v.as_u64())
            .ok_or("Transaction metadata missing amount")?;
            
        // API boundary: Solana returns u64 amounts, so we must convert for comparison
        let expected_amount_u64 = expected_amount.0.to_u64().ok_or("Expected amount too large for Solana (max ~18.4e18)")?;
        if actual_amount != expected_amount_u64 {
            return Err(format!(
                "Transaction amount mismatch. Expected: {}, Got: {}",
                expected_amount_u64, actual_amount
            ));
        }
    } else {
        return Err("Transaction metadata is missing".to_string());
    }
    
    Ok(())
}

