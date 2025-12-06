use candid::Nat;
use kong_lib::{ic::address::Address, stable_token::stable_token::StableToken, stable_transfer::tx_id::TxId, token_management::receive};

pub async fn receive_common(token: &StableToken, amount: &Nat, to_address: &Address, from_address: &Address, txid: Option<TxId>) -> Result<Nat, String> {
    match txid {
        Some(txid) => receive::receive_tx(token, amount, to_address, txid).await,
        None => receive::receive(token, amount, from_address, to_address).await,
    }
}
