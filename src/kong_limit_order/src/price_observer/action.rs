use candid::{CandidType, Nat};
use ic_cdk::{query, update};
use kong_lib::stable_token::token::Token;
use serde::{Deserialize, Serialize};

use crate::{
    order_action::exec_orders::do_exec_orders,
    orderbook::price::Price,
    price_observer::price_observer::{get_price, get_price_from_volumes, PRICE_OBSERVER},
    stable_memory_helpers::{get_kong_backend, get_token_by_symbol},
};

#[derive(Clone, Debug, CandidType, Serialize, Deserialize, Default)]
pub struct UpdateVolumeArgs {
    token_0: String,
    token_0_amount: Nat,
    token_1: String,
    token_1_amount: Nat,
}

#[update]
pub fn update_volumes(args: UpdateVolumeArgs) -> Result<(), String> {
    let msg_caller = ic_cdk::api::msg_caller();
    if !ic_cdk::api::is_controller(&msg_caller) && msg_caller.to_string() != get_kong_backend() {
        return Err("Only controller or kong_backend is allowed to update_volumes".to_string());
    }

    let _ = get_token_by_symbol(&args.token_0).ok_or(format!("Unknown token {}", args.token_0))?;
    let _ = get_token_by_symbol(&args.token_1).ok_or(format!("Unknown token {}", args.token_1))?;

    ic_cdk::println!("Update volumes called: {:?}", args);

    let volume_book_info = PRICE_OBSERVER.with(|price_observer| {
        price_observer.borrow_mut().update_volumes(
            &args.token_0,
            args.token_0_amount.clone(),
            &args.token_1,
            args.token_1_amount.clone(),
        )
    });

    ic_cdk::futures::spawn(async move {
        let exec_orders_result = match volume_book_info {
            Some((book_name, prev_volumes, cur_volumes)) => {
                do_exec_orders(
                    book_name.receive_token(),
                    book_name.send_token(),
                    Some(prev_volumes.get_price()),
                    cur_volumes.get_price(),
                )
                .await
            }
            None => {
                do_exec_orders(
                    &args.token_0,
                    &args.token_1,
                    None,
                    get_price_from_volumes(args.token_0_amount, args.token_1_amount),
                )
                .await
            }
        };
        if let Err(e) = exec_orders_result {
            ic_cdk::eprintln!("do_exec_orders err={}", e);
        }
    });

    Ok(())
}

#[query]
pub fn get_direct_price(receive_symbol: String, send_symbol: String) -> Result<Price, String> {
    let res = get_price(&receive_symbol, &send_symbol).ok_or("Price unknown".to_string());

    ic_cdk::println!("get_direct_price: {}", res.clone().unwrap_or(Price::default()));
    res
}

#[query]
pub fn get_price_f64(receive_symbol: String, send_symbol: String) -> Result<f64, String> {
    let res = get_direct_price(receive_symbol.clone(), send_symbol.clone())?;

    let receive_token = get_token_by_symbol(&receive_symbol).ok_or(format!("Unknown token {}", receive_symbol))?;
    let send_token = get_token_by_symbol(&send_symbol).ok_or(format!("Unknown token {}", send_symbol))?;

    Ok(res.0.to_f64_decimals(send_token.decimals(), receive_token.decimals()))
}
