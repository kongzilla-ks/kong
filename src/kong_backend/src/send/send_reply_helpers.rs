use candid::Nat;

use super::send_reply::SendReply;
use crate::stable_tx::send_tx::SendTx;

pub fn to_send_reply(send_tx: &SendTx) -> SendReply {
    SendReply::try_from(send_tx).unwrap_or_else(|_| SendReply {
        tx_id: send_tx.tx_id,
        request_id: send_tx.request_id,
        status: send_tx.status.to_string(),
        chain: "Token chain not found".to_string(),
        symbol: "Token symbol not found".to_string(),
        amount: send_tx.amount.clone(),
        to_address: "User principal id not found".to_string(),
        ts: send_tx.ts,
    })
}

pub fn to_send_reply_failed(request_id: u64, chain: &str, symbol: &str, amount: &Nat, to_address: &str, ts: u64) -> SendReply {
    SendReply {
        tx_id: 0,
        request_id,
        status: "Failed".to_string(),
        chain: chain.to_string(),
        symbol: symbol.to_string(),
        amount: amount.clone(),
        to_address: to_address.to_string(),
        ts,
    }
}
