use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

/// Arguments for adding a token.
#[derive(CandidType, Debug, Clone, Serialize, Deserialize, Default)]
pub struct AddTokenArgs {
    pub token: String,
    // Optional fields for Solana tokens
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub decimals: Option<u8>,
    #[serde(default)]
    pub fee: Option<Nat>,
    #[serde(default)]
    pub solana_program_id: Option<String>,
    #[serde(default)]
    pub solana_mint_address: Option<String>,
}
