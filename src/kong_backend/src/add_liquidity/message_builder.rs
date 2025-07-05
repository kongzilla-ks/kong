use candid::Nat;
use serde::{Deserialize, Serialize};

use super::add_liquidity_args::AddLiquidityArgs;

/// A structure representing the canonical message format for signing liquidity additions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalAddLiquidityMessage {
    pub token_0: String,
    pub amount_0: Nat,
    pub token_1: String,
    pub amount_1: Nat,
}

impl CanonicalAddLiquidityMessage {
    /// Create a canonical message from AddLiquidityArgs
    pub fn from_add_liquidity_args(args: &AddLiquidityArgs) -> Self {
        Self {
            token_0: args.token_0.clone(),
            amount_0: args.amount_0.clone(),
            token_1: args.token_1.clone(),
            amount_1: args.amount_1.clone(),
        }
    }

    /// Serialize to JSON string for signing
    pub fn to_signing_message(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize message")
    }
}