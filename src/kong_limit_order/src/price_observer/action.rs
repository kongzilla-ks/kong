use candid::{CandidType, Nat};
use ic_cdk::{query, update};
use kong_lib::stable_token::token::Token;
use serde::{Deserialize, Serialize};

use crate::{orderbook::price::Price, price_observer::price_observer::PRICE_OBSERVER, stable_memory_helpers::get_token_by_symbol};

#[derive(Clone, Debug, CandidType, Serialize, Deserialize, Default)]
pub struct UpdateVolumeArgs {
    token_0: String,
    token_0_amount: Nat,
    token_1: String,
    token_1_amount: Nat,
}

#[update]
pub fn update_volumes(args: UpdateVolumeArgs) -> Result<(), String> {
    let _ = get_token_by_symbol(&args.token_0).ok_or(format!("Unknown token {}", args.token_0))?;
    let _ = get_token_by_symbol(&args.token_1).ok_or(format!("Unknown token {}", args.token_1))?;

    ic_cdk::println!("Update volumes called: {:?}", args);
    // TODO: add kong/controller only
    PRICE_OBSERVER.with(|price_observer| {
        price_observer
            .borrow_mut()
            .update_volumes(&args.token_0, args.token_0_amount, &args.token_1, args.token_1_amount);
    });

    Ok(())
}

#[query]
pub fn get_direct_price(receive_symbol: String, send_symbol: String) -> Result<Price, String> {
    PRICE_OBSERVER.with(|price_observer| {
        price_observer
            .borrow()
            .get_price(&receive_symbol, &send_symbol)
            .ok_or("Price unknown".to_string())
    })
}

#[query]
pub fn get_price_f64(receive_symbol: String, send_symbol: String) -> Result<f64, String> {
    let res = get_direct_price(receive_symbol.clone(), send_symbol.clone())?;

    let receive_token = get_token_by_symbol(&receive_symbol).ok_or(format!("Unknown token {}", receive_symbol))?;
    let send_token = get_token_by_symbol(&send_symbol).ok_or(format!("Unknown token {}", send_symbol))?;

    Ok(res.0.to_f64_decimals(send_token.decimals(), receive_token.decimals()))
}
