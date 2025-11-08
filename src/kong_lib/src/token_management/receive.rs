use candid::Nat;

use crate::{
    ic::address::Address,
    stable_token::stable_token::StableToken,
    stable_transfer::tx_id::TxId,
    token_management::{ic_receive::icrc2_transfer_from, verify_transfer},
};

pub async fn receive(token: &StableToken, amount: &Nat, from_address: &Address, to_address: &Address) -> Result<Nat, String> {
    match from_address {
        Address::AccountId(_) => Err("Invalid from address type".to_string()),
        Address::PrincipalId(account) => {
            let to_address = match to_address {
                Address::AccountId(_) => return Err("Invalid to address type".to_string()),
                Address::PrincipalId(account) => account,
            };
            icrc2_transfer_from(token, amount, account, to_address)
                .await
                .map_err(|e| e.to_string())
        }
    }
}

// TODO: remove nat parameter
pub async fn receive_tx(token: &StableToken, amount: &Nat, to_address: &Address, txid: TxId) -> Result<Nat, String> {
    match txid {
        TxId::BlockIndex(block_id) => verify_transfer::verify_transfer(token, to_address, &block_id, amount).await,
        TxId::TransactionHash(_) => Err("Unimplemented".to_string()),
    }
}
