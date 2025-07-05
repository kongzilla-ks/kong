use candid::CandidType;
use serde::{Deserialize, Serialize};

/// Arguments for updating a token.
#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTokenArgs {
    pub token: String,
    // Optional fields for updating Solana token metadata
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub decimals: Option<u8>,
}
