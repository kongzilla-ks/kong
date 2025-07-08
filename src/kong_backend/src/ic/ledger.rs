use candid::{CandidType, Nat, Principal};
use icrc_ledger_types::icrc1::account::Account;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct StandardRecord {
    pub url: String,
    pub name: String,
}

pub async fn get_balance(principal_id: Account, ledger: &Principal) -> Result<Nat, String> {
    ic_cdk::call::Call::unbounded_wait(*ledger, "icrc1_balance_of")
        .with_arg(principal_id)
        .await
        .map_err(|e| format!("{:?}", e))?
        .candid::<Nat>()
        .map_err(|e| format!("{:?}", e))
}

pub async fn get_name(ledger: &Principal) -> Result<String, String> {
    ic_cdk::call::Call::unbounded_wait(*ledger, "icrc1_name")
        .await
        .map_err(|e| format!("{:?}", e))?
        .candid::<String>()
        .map_err(|e| format!("{:?}", e))
}

pub async fn get_symbol(ledger: &Principal) -> Result<String, String> {
    ic_cdk::call::Call::unbounded_wait(*ledger, "icrc1_symbol")
        .await
        .map_err(|e| format!("{:?}", e))?
        .candid::<String>()
        .map_err(|e| format!("{:?}", e))
}

pub async fn get_decimals(ledger: &Principal) -> Result<u8, String> {
    ic_cdk::call::Call::unbounded_wait(*ledger, "icrc1_decimals")
        .await
        .map_err(|e| format!("{:?}", e))?
        .candid::<u8>()
        .map_err(|e| format!("{:?}", e))
}

pub async fn get_fee(ledger: &Principal) -> Result<Nat, String> {
    ic_cdk::call::Call::unbounded_wait(*ledger, "icrc1_fee")
        .await
        .map_err(|e| format!("{:?}", e))?
        .candid::<Nat>()
        .map_err(|e| format!("{:?}", e))
}

/// try icrc10_supported_standards first, if it fails, try icrc1_supported_standards
pub async fn get_supported_standards(ledger: &Principal) -> Result<Vec<StandardRecord>, String> {
    match ic_cdk::call::Call::unbounded_wait(*ledger, "icrc10_supported_standards")
        .await
    {
        Ok(response) => response.candid::<Vec<StandardRecord>>()
            .map_err(|e| format!("{:?}", e)),
        Err(_) => ic_cdk::call::Call::unbounded_wait(*ledger, "icrc1_supported_standards")
            .await
            .map_err(|e| format!("{:?}", e))?
            .candid::<Vec<StandardRecord>>()
            .map_err(|e| format!("{:?}", e)),
    }
}
