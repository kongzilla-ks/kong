use candid::CandidType;
use serde::{Deserialize, Serialize};

/// Arguments for updating a token.
#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTokenArgs {
    pub token: String,
    // Optional fields for updating Solana token metadata
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
}
