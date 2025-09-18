use candid::{CandidType, Nat, Principal};
use serde::{Deserialize, Serialize};

use crate::{stable_memory::KONG_SETTINGS, stable_token::{stable_token::StableToken, token::Token}};

fn get_kong_limit_canister() -> Option<Principal> {
    KONG_SETTINGS.with_borrow(|kong_settings| kong_settings.get().kong_limit)
}

pub fn add_kong_limit_token(address: &str) -> Result<(), String> {
    let kong_limit = match get_kong_limit_canister() {
        Some(kong_limit) => kong_limit,
        None => return Ok(()),
    };

    let address = address.to_string();

    ic_cdk::futures::spawn(async move {
        match ic_cdk::call::Call::unbounded_wait(kong_limit, "add_ic_token")
            .with_arg(address)
            .await
            // Omit response
            .map_err(|e| format!("{:?}", e))
            .map(|_| ())
        {
            Ok(_) => {}
            Err(e) => ic_cdk::eprintln!("kong limit call, err={}", e),
        }
    });

    Ok(())
}

// TODO: share in kong_lib
#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct OrderbookTokens {
    token_0: String,
    token_1: String,
}

pub fn add_kong_limit_pool(token_0: &StableToken, amount_0: Nat, token_1: &StableToken, amount_1: Nat) -> Result<(), String> {
    let kong_limit = match get_kong_limit_canister() {
        Some(kong_limit) => kong_limit,
        None => return Ok(()),
    };

    let symbol_0 = token_0.symbol();
    let symbol_1 = token_1.symbol();
    let address_0 = token_0.address();
    let address_1 = token_1.address();

    ic_cdk::futures::spawn(async move {
        match ic_cdk::call::Call::unbounded_wait(kong_limit, "add_available_token_pair")
            .with_arg(OrderbookTokens {
                token_0: address_0,
                token_1: address_1,
            })
            .await
            .map_err(|e| format!("{:?}", e))
        {
            Ok(_) => {
                let _ = update_kong_limit_volumes(symbol_0, amount_0, symbol_1, amount_1);
            }
            Err(e) => ic_cdk::eprintln!("kong limit call, err={}", e),
        }
    });

    Ok(())
}

// TODO: share in kong_lib
#[derive(Clone, Debug, CandidType, Serialize, Deserialize, Default)]
pub struct UpdateVolumeArgs {
    token_0: String,
    token_0_amount: Nat,
    token_1: String,
    token_1_amount: Nat,
}

pub fn update_kong_limit_volumes(symbol_0: String, amount_0: Nat, symbol_1: String, amount_1: Nat) -> Result<(), String> {
    let kong_limit = match get_kong_limit_canister() {
        Some(kong_limit) => kong_limit,
        None => return Ok(()),
    };

    ic_cdk::futures::spawn(async move {
        match ic_cdk::call::Call::unbounded_wait(kong_limit, "update_volumes")
            .with_arg(UpdateVolumeArgs {
                token_0: symbol_0,
                token_0_amount: amount_0,
                token_1: symbol_1,
                token_1_amount: amount_1,
            })
            .await
            .map_err(|e| format!("{:?}", e))
        {
            Ok(response) => match response.candid::<Result<(), String>>().map_err(|e| format!("{:?}", e)) {
                Ok(_) => {}
                Err(e) => ic_cdk::eprintln!("kong limit response, err={}", e),
            },
            Err(e) => ic_cdk::eprintln!("kong limit call, err={}", e),
        }
    });

    Ok(())
}
