use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

use crate::stable_token::token::Token;
use crate::stable_token::token_map;
use crate::stable_tx::send_tx::SendTx;
use crate::stable_user::user_map;

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct SendReply {
    pub tx_id: u64,
    pub request_id: u64,
    pub status: String,
    pub chain: String,
    pub symbol: String,
    pub amount: Nat,
    pub to_address: String,
    pub ts: u64,
}

impl TryFrom<&SendTx> for SendReply {
    type Error = String;
    
    fn try_from(send_tx: &SendTx) -> Result<Self, Self::Error> {
        let token = token_map::get_by_token_id(send_tx.token_id)
            .ok_or_else(|| "Token not found".to_string())?;
        let to_user = user_map::get_by_user_id(send_tx.to_user_id)
            .ok_or_else(|| "User not found".to_string())?;
        
        Ok(SendReply {
            tx_id: send_tx.tx_id,
            request_id: send_tx.request_id,
            status: send_tx.status.to_string(),
            chain: token.chain(),
            symbol: token.symbol(),
            amount: send_tx.amount.clone(),
            to_address: to_user.principal_id,
            ts: send_tx.ts,
        })
    }
}
