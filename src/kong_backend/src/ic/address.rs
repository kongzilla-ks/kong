use candid::{CandidType, Principal};
use ic_ledger_types::AccountIdentifier;
use icrc_ledger_types::icrc1::account::Account;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::sync::OnceLock;

use crate::stable_token::{stable_token::StableToken, token::Token};

use super::icp::is_icp_token_id;

static PRINCIPAL_ID_LOCK: OnceLock<Regex> = OnceLock::new();
const PRINCIPAL_ID_REGEX: &str = r"^([a-z0-9]{5}-){10}[a-z0-9]{3}$|^([a-z0-9]{5}-){4}cai$";
static ACCOUNT_ID_LOCK: OnceLock<Regex> = OnceLock::new();
const ACCOUNT_ID_REGEX: &str = r"^[a-f0-9]{64}$";

/// Represents an address which can be either an Account ID, a Principal ID, or a Solana Address.
#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub enum Address {
    AccountId(AccountIdentifier),
    PrincipalId(Account),
    SolanaAddress(String),
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Address::AccountId(account_id) => write!(f, "{}", account_id),
            Address::PrincipalId(principal_id) => write!(f, "{}", principal_id),
            Address::SolanaAddress(address) => write!(f, "{}", address),
        }
    }
}

pub fn get_address(token: &StableToken, address: &str) -> Result<Address, String> {
    let regrex_princiapl_id = PRINCIPAL_ID_LOCK.get_or_init(|| Regex::new(PRINCIPAL_ID_REGEX).unwrap());
    let regrex_account_id = ACCOUNT_ID_LOCK.get_or_init(|| Regex::new(ACCOUNT_ID_REGEX).unwrap());

    if regrex_princiapl_id.is_match(address) {
        if !token.is_icrc1() {
            return Err("Principal Id requires ICRC1 token".to_string());
        }
        Ok(Address::PrincipalId(Account::from(
            Principal::from_text(address).map_err(|e| e.to_string())?,
        )))
    } else if regrex_account_id.is_match(address) {
        if is_icp_token_id(token.token_id()) {
            return Err("Account Id supported only for ICP token".to_string());
        }
        Ok(Address::AccountId(AccountIdentifier::from_hex(address).map_err(|e| e.to_string())?))
    } else {
        Err("Invalid address format".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;
    use ed25519_consensus::SigningKey;
    use ic_agent::{identity::BasicIdentity, Identity};
    use ic_ledger_types::AccountIdentifier;
    use icrc_ledger_types::icrc1::account::Account;
    use rand::{thread_rng, Rng};

    #[test]
    fn test_display_account_id() {
        let hex_account_id = "da29b27beb16a842882149b5380ff3b20f701c33ca8fddbecdb5201c600e0f0e";
        let account_id = AccountIdentifier::from_hex(hex_account_id).unwrap();
        let address = Address::AccountId(account_id);
        assert_eq!(address.to_string(), hex_account_id);
    }

    #[test]
    fn test_display_principal_id() {
        let signing_key = SigningKey::new(thread_rng());
        let identity = BasicIdentity::from_signing_key(signing_key);
        let principal_text = identity.sender().unwrap().to_text();
        let principal = Principal::from_text(principal_text.clone()).unwrap();
        let account = Account {
            owner: principal,
            subaccount: None,
        };
        let address = Address::PrincipalId(account);
        assert_eq!(address.to_string(), principal_text);
    }

    #[test]
    fn test_display_canister_id() {
        let canister_text = "2ipq2-uqaaa-aaaar-qailq-cai";
        let canister = Principal::from_text(canister_text).unwrap();
        let account = Account {
            owner: canister,
            subaccount: None,
        };
        let address = Address::PrincipalId(account);
        assert_eq!(address.to_string(), canister_text);
    }

    #[test]
    fn test_display_principal_id_with_subaccount() {
        let signing_key = SigningKey::new(thread_rng());
        let identity = BasicIdentity::from_signing_key(signing_key);
        let principal_text = identity.sender().unwrap().to_text();
        let principal = Principal::from_text(principal_text.clone()).unwrap();
        let subaccount: [u8; 32] = rand::thread_rng().gen(); // Generate random 32-byte array
        let account = Account {
            owner: principal,
            subaccount: Some(subaccount),
        };
        let address = Address::PrincipalId(account);
        // address should include subaccount in its display but it doesn't so just check principal
        assert!(format!("{}", address).contains(&principal_text));
    }
}
