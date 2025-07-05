use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

use crate::stable_transfer::tx_id::TxId;

/// Data structure for the arguments of the `add_liquidity` function.
/// Used in StableRequest
#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct AddLiquidityArgs {
    pub token_0: String,
    pub amount_0: Nat,
    pub tx_id_0: Option<TxId>,
    pub token_1: String,
    pub amount_1: Nat,
    pub tx_id_1: Option<TxId>,
    // Cross-chain signature support (following issue #6 spec)
    #[serde(default)]
    pub signature_0: Option<String>,     // Ed25519 signature for token_0 transfer
    #[serde(default)]
    pub signature_1: Option<String>,     // Ed25519 signature for token_1 transfer
}

impl Default for AddLiquidityArgs {
    fn default() -> Self {
        Self {
            token_0: String::new(),
            amount_0: Nat::from(0u64),
            tx_id_0: None,
            token_1: String::new(),
            amount_1: Nat::from(0u64),
            tx_id_1: None,
            signature_0: None,
            signature_1: None,
        }
    }
}
