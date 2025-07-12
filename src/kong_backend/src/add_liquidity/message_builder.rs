use candid::Nat;
use serde::{Deserialize, Serialize};

use super::add_liquidity_args::AddLiquidityArgs;

/// Custom serializer to convert Nat to string (matching frontend format)
fn serialize_amount_as_string<S>(amount: &Nat, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    // Remove underscores from Nat serialization to match frontend format
    let amount_str = amount.to_string().replace('_', "");
    serializer.serialize_str(&amount_str)
}

/// A structure representing the canonical message format for signing liquidity additions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalAddLiquidityMessage {
    pub token_0: String,
    #[serde(serialize_with = "serialize_amount_as_string")]
    pub amount_0: Nat,
    pub token_1: String,
    #[serde(serialize_with = "serialize_amount_as_string")]
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