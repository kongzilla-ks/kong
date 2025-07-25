use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

use crate::chains::chains::SOL_CHAIN;

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct SolanaToken {
    pub token_id: u32,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub fee: Nat,
    pub mint_address: String,      // Solana mint address
    pub program_id: String,        // SPL Token program ID
    #[serde(default = "default_is_spl_token")]
    pub is_spl_token: bool, // True for SPL tokens, false for native SOL
}

fn default_is_spl_token() -> bool {
    true // Default to SPL token for backward compatibility
}

impl SolanaToken {
    pub fn chain(&self) -> String {
        SOL_CHAIN.to_string()
    }
}
