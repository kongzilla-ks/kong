use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

use crate::stable_transfer::tx_id::TxId;

/// Data structure for the arguments of the `swap` function.
/// Used in StableRequest
/// 
/// For cross-chain swaps: pay_signature field is required for Solana/SPL tokens
/// - IC tokens: No signature needed (standard IC transfer)
/// - Solana tokens: Signature required for verification
#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct SwapArgs {
    pub pay_token: String,
    pub pay_amount: Nat,
    pub pay_tx_id: Option<TxId>,        // None for IC approve+transfer, Some for cross-chain
    pub receive_token: String,
    pub receive_amount: Option<Nat>,
    pub receive_address: Option<String>, // Required for non-IC receive tokens
    pub max_slippage: Option<f64>,
    pub referred_by: Option<String>,
    // Cross-chain fields
    pub pay_signature: Option<String>,   // Ed25519 signature of canonical message for payment verification
}

