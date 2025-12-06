use candid::{Nat, Principal};
use ic_ledger_types::{AccountIdentifier, Subaccount};
use icrc_ledger_types::icrc1::account::Account;

use crate::{
    ic::address::Address,
    stable_token::{stable_token::StableToken, token::Token},
    token_management::ic_receive::{icp_transfer, icrc1_transfer},
};

pub async fn send(amount: &Nat, dst_address: &Address, token: &StableToken, created_at_time: Option<u64>) -> Result<Nat, String> {
    match dst_address {
        Address::AccountId(account_identifier) => icp_transfer(amount, account_identifier, token, created_at_time)
            .await
            .map_err(|e| e.to_string()),
        Address::PrincipalId(account) => icrc1_transfer(amount, account, token, created_at_time)
            .await
            .map_err(|e| e.to_string()),
    }
}

fn is_icp_token(token: &StableToken) -> bool {
    token.symbol() == "icp" || token.symbol() == "ICP"
}

fn get_account_id(user: Principal) -> AccountIdentifier {
    let account = Account::from(user);
    let subaccount = Subaccount(account.subaccount.unwrap_or([0; 32]));
    AccountIdentifier::new(&account.owner, &subaccount)
}

pub fn get_address_by_user_and_token(user: Principal, token: &StableToken) -> Address {
    if is_icp_token(token) {
        Address::AccountId(get_account_id(user))
    } else {
        Address::PrincipalId(Account::from(user))
    }
}

// fn is_address_token_valid(address: &Address, token: &StableToken) -> bool {
//     let is_icp_token = token.symbol() == "icp" || token.symbol() == "ICP";
//     match address {
//         Address::AccountId(_) => is_icp_token,
//         Address::PrincipalId(_) => todo!(),
//     }
// }

pub fn get_address_to_send(user: Principal, dst_address: Option<Address>, token: &StableToken) -> Address {
    match dst_address {
        Some(address) => return address,
        None => {}
    };

    get_address_by_user_and_token(user, token)
}
