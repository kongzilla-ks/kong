use candid::Nat;
use serde::{Deserialize, Serialize};

use crate::helpers::nat_helpers::serialize_amount_as_string;
use crate::add_pool::add_pool_args::AddPoolArgs;

/// A structure representing the canonical message format for signing pool additions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalAddPoolMessage {
    pub token_0: String,
    #[serde(serialize_with = "serialize_amount_as_string")]
    pub amount_0: Nat,
    pub token_1: String,
    #[serde(serialize_with = "serialize_amount_as_string")]
    pub amount_1: Nat,
    pub lp_fee_bps: u8,
}

impl CanonicalAddPoolMessage {
    /// Create a canonical message from AddPoolArgs
    pub fn from_add_pool_args(args: &AddPoolArgs) -> Self {
        Self {
            token_0: args.token_0.clone(),
            amount_0: args.amount_0.clone(),
            token_1: args.token_1.clone(),
            amount_1: args.amount_1.clone(),
            lp_fee_bps: args.lp_fee_bps.unwrap_or(30), // Default 30 bps if not specified
        }
    }

    /// Serialize to JSON string for signing
    pub fn to_signing_message(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize message")
    }
}