use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};
use std::convert::From;

use crate::helpers::nat_helpers::nat_zero;
use crate::stable_claim::stable_claim::StableClaim;
use crate::stable_token::token::Token;
use crate::stable_token::token_map;
use crate::stable_user::user_map;

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct ClaimsReply {
    pub claim_id: u64,
    pub status: String,
    pub chain: String,
    pub symbol: String,
    pub canister_id: Option<String>,
    pub amount: Nat,
    pub fee: Nat,
    pub to_address: String,
    pub desc: String,
    pub ts: u64,
}

impl From<&StableClaim> for ClaimsReply {
    fn from(claim: &StableClaim) -> Self {
        let (chain, symbol, canister_id, fee) = match token_map::get_by_token_id(claim.token_id) {
            Some(token) => (
                token.chain().to_string(),
                token.symbol().to_string(),
                token.canister_id().map(|id| id.to_text()),
                token.fee(),
            ),
            None => ("Chain not found".to_string(), "Symbol not found".to_string(), None, nat_zero()),
        };
        
        let to_address = match &claim.to_address {
            Some(address) => address.to_string(),
            None => match user_map::get_by_user_id(claim.user_id) {
                Some(user) => user.principal_id,
                None => "To address not found".to_string(),
            },
        };
        
        ClaimsReply {
            claim_id: claim.claim_id,
            status: claim.status.to_string(),
            chain,
            symbol,
            canister_id,
            amount: claim.amount.clone(),
            fee,
            to_address,
            desc: claim.desc.as_ref().map_or_else(String::new, |desc| desc.to_string()),
            ts: claim.ts,
        }
    }
}
