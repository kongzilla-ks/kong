use candid::Principal;
use ic_cdk::update;

use crate::ic::guards::caller_is_kingkong;
use crate::{
    ic::{ledger::get_name, transfer::InternalTransferError},
    stable_token::{
        stable_token::StableToken,
        token::Token,
        token_map::{get_disabled, update},
    },
};

fn enable_token_impl(token: &mut StableToken, value: bool) {
    let action = if value { "enabling" } else { "disabling" };
    ic_cdk::println!("{} token, id={}", action, token.token_id());

    let is_removed = !value;

    match token {
        StableToken::LP(lptoken) => lptoken.is_removed = is_removed,
        StableToken::IC(ictoken) => ictoken.is_removed = is_removed,
        StableToken::Solana(_) => {}
    }

    update(&token);
}

fn disable_token(token: &StableToken) {
    if !token.is_removed() {
        // remove(token.id())
        let mut token = token.clone();
        enable_token_impl(&mut token, false);
    }
}

pub fn handle_failed_transfer(token: &StableToken, err: InternalTransferError) {
    match err {
        InternalTransferError::General(_) => return,
        InternalTransferError::CallFailure(call_failed) => match call_failed {
            ic_cdk::call::CallFailed::InsufficientLiquidCycleBalance(b) => {
                ic_cdk::println!(
                    "DBG: InsufficientLiquidCycleBalance, balance available={}, required={}, e={}",
                    b.available,
                    b.required,
                    b.to_string()
                );
                // Not sure if need to disable token in this case
                // disable_token(token);
            }
            ic_cdk::call::CallFailed::CallPerformFailed(call_perform_failed) => {
                ic_cdk::println!("DBG: CallPerformFailed, e={}", call_perform_failed.to_string());
                // Not sure if need to disable token in this case
                // disable_token(token);
            }
            ic_cdk::call::CallFailed::CallRejected(call_rejected) => {
                let reject_code = match call_rejected.reject_code() {
                    Ok(r) => r.to_string(),
                    Err(c) => c.to_string(),
                };
                ic_cdk::println!("DBG: CallRejected, code={}, e={}", reject_code, call_rejected.to_string());
                disable_token(token);
            }
        },
    }
}

#[update(hidden = true, guard = "caller_is_kingkong")]
pub async fn check_disabled_tokens() -> Vec<u32> {
    let mut enabled_tokens = Vec::new();

    let tokens = get_disabled();

    async fn check_alive(ledger: &Principal) -> bool {
        get_name(ledger).await.is_ok()
    }

    for mut token in tokens {
        let is_alive = match &token {
            StableToken::LP(_) => {
                // Don't manually enable lptoken
                false
            }
            StableToken::IC(ictoken) => check_alive(&ictoken.canister_id).await,
            StableToken::Solana(_) => false,
        };

        if is_alive {
            enable_token_impl(&mut token, true);
            enabled_tokens.push(token.token_id());
        }
    }

    enabled_tokens
}
