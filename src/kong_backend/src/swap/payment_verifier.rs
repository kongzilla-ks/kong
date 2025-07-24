//! Payment verification module
//! Handles verification of incoming payments for swaps

use anyhow::Result;
use candid::{Nat, Principal};

use crate::ic::verify_transfer::verify_transfer;
use crate::solana::payment_verification::{extract_solana_sender_from_transaction, verify_solana_transaction};
use crate::solana::signature_verification::verify_canonical_message;
use crate::stable_token::stable_token::StableToken;
use crate::stable_transfer::tx_id::TxId;

use super::message_builder::CanonicalSwapMessage;
use super::swap_args::SwapArgs;

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
    pub async fn verify_payment(&self, args: &SwapArgs, pay_token: &StableToken, pay_amount: &Nat) -> Result<PaymentVerification, String> {
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
            StableToken::LP(_) => Err("LP tokens cannot be used as payment".to_string()),
        }
    }

    /// Verify IC token payment by checking the transfer
    async fn verify_ic_payment(&self, args: &SwapArgs, pay_token: &StableToken, pay_amount: &Nat) -> Result<PaymentVerification, String> {
        let pay_tx_id = args
            .pay_tx_id
            .as_ref()
            .ok_or_else(|| "Transaction ID (pay_tx_id) is required for token payments".to_string())?;

        let block_index = match pay_tx_id {
            TxId::BlockIndex(index) => index.clone(),
            TxId::TransactionId(_) => return Err("IC tokens require BlockIndex, not TransactionId".to_string()),
        };

        // Verify the transfer on the token's ledger (includes amount validation)
        verify_transfer(pay_token, &block_index, pay_amount)
            .await
            .map_err(|e| format!("Transfer verification failed: {}", e))?;

        Ok(PaymentVerification::IcpPayment {
            block_index,
            from_principal: self.caller,
            amount: pay_amount.clone(),
        })
    }
}

/// Verify Solana payment by checking signature and transaction data
async fn verify_solana_payment(
    args: &SwapArgs,
    pay_amount: &Nat,
    sol_token: &crate::stable_token::solana_token::SolanaToken,
) -> Result<PaymentVerification, String> {
    // Solana tokens require signature
    let signature = args
        .pay_signature
        .as_ref()
        .ok_or_else(|| "Payment signature is required for Solana/SPL tokens".to_string())?;

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
    let sender_pubkey = extract_solana_sender_from_transaction(&tx_signature_str, is_spl_token).await?;

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
    let canonical_message = CanonicalSwapMessage::from_swap_args(args).with_sender(sender_pubkey.clone());
    let message_to_verify = canonical_message.to_signing_message();

    verify_canonical_message(&message_to_verify, &sender_pubkey, signature).map_err(|e| format!("Signature verification failed: {}", e))?;

    // Verify the actual Solana transaction from storage
    verify_solana_transaction(&tx_signature_str, &sender_pubkey, pay_amount, is_spl_token).await?;

    Ok(PaymentVerification::SolanaPayment {
        tx_signature: tx_signature_str,
        from_address: sender_pubkey,
        amount: pay_amount.clone(),
    })
}
