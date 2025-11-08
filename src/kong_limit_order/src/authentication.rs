use candid::{decode_one, CandidType, Deserialize};
use ic_cdk::api::msg_caller;
use ic_cdk::{query, update};

use icrc_ledger_types::icrc21::errors::ErrorInfo;
use icrc_ledger_types::icrc21::requests::{ConsentMessageMetadata, ConsentMessageRequest};
use icrc_ledger_types::icrc21::responses::{ConsentInfo, ConsentMessage};

use crate::delegation::{
    get_current_time, hash_principals, Delegation, DelegationError, DelegationRequest, DelegationResponse, RevokeDelegationRequest,
};
use crate::stable_memory::DELEGATIONS;

#[derive(CandidType, Deserialize, Debug)]
pub struct ICRC21ConsentMessageRequest {
    pub canister: candid::Principal,
    pub method: String,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct ICRC21ConsentMessageResponse {
    pub consent_message: String,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct CreateMessageArgs {
    pub message: String,
    pub channel: Option<String>,
}

/// Generates a consent message for ICRC-21 canister calls
/// This follows the specification from https://github.com/dfinity/wg-identity-authentication/blob/main/topics/ICRC-21/icrc_21_consent_msg.md
#[query]
pub fn icrc21_canister_call_consent_message(consent_msg_request: ConsentMessageRequest) -> Result<ConsentInfo, ErrorInfo> {
    let caller_principal = msg_caller();

    let consent_message = match consent_msg_request.method.as_str() {
        "create_message" => {
            let message = decode_one::<String>(&consent_msg_request.arg).map_err(|e| ErrorInfo {
                description: format!("Failed to decode message: {}", e),
            })?;

            ConsentMessage::GenericDisplayMessage(format!(
                "# Approve Kong Limit Order Message\n\nMessage: {}\n\nFrom: {}",
                message, caller_principal
            ))
        }
        // Add other method matches here as needed
        _ => ConsentMessage::GenericDisplayMessage(format!("Approve Kong Limit Order to execute {}?", consent_msg_request.method,)),
    };

    let metadata = ConsentMessageMetadata {
        language: "en".to_string(),
        utc_offset_minutes: None,
    };

    Ok(ConsentInfo { metadata, consent_message })
}

/// Returns current delegations for the caller that match the requested targets
#[query]
pub fn icrc_34_get_delegation(request: DelegationRequest) -> Result<DelegationResponse, DelegationError> {
    request.validate()?;

    let caller_principal = msg_caller();
    let targets_hash = request.compute_targets_hash();

    let delegations = DELEGATIONS.with(|store| {
        store
            .borrow()
            .get(&caller_principal)
            .map(|d| d.as_vec().clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|d| !d.is_expired() && d.targets_list_hash == targets_hash)
            .collect::<Vec<_>>()
    });

    Ok(DelegationResponse { delegations })
}

/// Creates a new delegation for the specified targets
#[update]
pub fn icrc_34_delegate(request: DelegationRequest) -> Result<DelegationResponse, DelegationError> {
    request.validate()?;

    let caller_principal = msg_caller();
    let current_time = get_current_time();
    let targets_hash = request.compute_targets_hash();

    let delegation = Delegation {
        target: caller_principal,
        created: current_time,
        expiration: request.expiration,
        targets_list_hash: targets_hash,
    };

    DELEGATIONS.with(|store| {
        let mut store = store.borrow_mut();
        let mut user_delegations = store.get(&caller_principal).unwrap_or_default();

        // Remove expired delegations
        user_delegations.retain(|d| !d.is_expired());

        // Add new delegation
        user_delegations.push(delegation.clone());

        store.insert(caller_principal, user_delegations);
        Ok(DelegationResponse {
            delegations: vec![delegation],
        })
    })
}

/// Revokes delegations for the specified targets
#[update]
pub fn icrc_34_revoke_delegation(request: RevokeDelegationRequest) -> Result<(), DelegationError> {
    if request.targets.is_empty() {
        return Err(DelegationError::InvalidRequest("No targets specified".to_string()));
    }

    let caller_principal = msg_caller();
    let targets_hash = {
        let mut targets = request.targets;
        targets.sort();
        hash_principals(&targets)
    };

    DELEGATIONS.with(|store| {
        let mut store = store.borrow_mut();
        let mut user_delegations = store.get(&caller_principal).unwrap_or_default();

        // Remove delegations with matching hash
        user_delegations.retain(|d| d.targets_list_hash != targets_hash);

        store.insert(caller_principal, user_delegations);
        Ok(())
    })
}

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct Icrc28TrustedOriginsResponse {
    pub trusted_origins: Vec<String>,
}

// list every base URL that users will authenticate to your app from
#[query]
fn icrc28_trusted_origins() -> Icrc28TrustedOriginsResponse {
    let trusted_origins = vec![
        format!("https://edoy4-liaaa-aaaar-qakha-cai.localhost:5173"), // svelte FE
        format!("http://localhost:5173"),
        String::from("https://kongswap.io"),
        String::from("https://www.kongswap.io"),
        String::from("https://edoy4-liaaa-aaaar-qakha-cai.icp0.io"),
        String::from("https://dev.kongswap.io"),
    ];

    Icrc28TrustedOriginsResponse { trusted_origins }
}
