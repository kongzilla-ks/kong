use ic_cdk::{query, update};
use icrc_ledger_types::icrc1::account::Account;
use kong_lib::{ic::{address::Address, network::ICNetwork}, stable_token::token::Token};

use crate::{
    orderbook::price::Price, stable_memory_helpers::{get_min_twap_notional, get_token_by_symbol}, token_management::transfer, twap::{
        twap::{Twap, TwapArgs},
        twap_executor::TWAP_EXECUTOR,
        usd_notional::usd_notional,
    }
};

#[update]
pub async fn add_twap(args: TwapArgs) -> Result<u64, String> {
    if args.order_period_ts < 10 {
        return Err("Delay between orders should be minimum 10 seconds".to_string());
    }
    if args.order_period_ts > 30 * 24 * 60 * 60 {
        return Err("Delay between orders should be maximum 30 days".to_string());
    }

    if args.order_amount == 0 {
        return Err("Invalid order amount".to_string());
    }

    // check tokens exist
    let pay_token = get_token_by_symbol(&args.pay_symbol).ok_or(format!("Unknown token {}", args.pay_symbol))?;
    let receive_token = get_token_by_symbol(&args.receive_symbol).ok_or(format!("Unknown token {}", args.receive_symbol))?;
    // check price is valid
    if let Some(price) = &args.max_price {
        let _ = Price::new_str(price)?;
    }

    if pay_token.fee() * args.order_amount * 2u32 >= args.pay_amount {
        return Err("Pay amount is too low for mentioned order amount".to_string());
    }

    let notional_price =
        usd_notional(args.pay_symbol.clone(), args.pay_amount.clone()).ok_or(format!("Can not get notional of token {}", args.pay_symbol))?;

    let min_twap_notional = get_min_twap_notional();
    if notional_price < min_twap_notional {
        return Err(format!("Notional price is too low, {}<{}", notional_price, min_twap_notional));
    }
    if notional_price > 1_000_000.0 {
        return Err(format!("Notional price is too large, {}>1_000_000", notional_price));
    }

    let from_address = Address::PrincipalId(ICNetwork::caller_id());
    let _ = args.get_receive_address()?; // validate address

    // let limit_backend = get_limit_backend();
    // let to_address = get_address(&limit_backend).ok_or(format!("Can't get address from {}", limit_backend))?;
    
    let kong_limit = Address::PrincipalId(Account::from(ic_cdk::api::canister_self()));
    let _received = transfer::receive_common(&pay_token, &args.pay_amount, &kong_limit, &from_address, args.pay_tx_id.clone()).await?;
    // transfer::receive_common()

    let id = TWAP_EXECUTOR.with_borrow_mut(|twap_executor| twap_executor.add_twap(args, notional_price, pay_token, receive_token));

    Ok(id)
}

#[update]
pub fn cancel_twap(twap_id: u64) -> Result<Twap, String> {
    TWAP_EXECUTOR.with_borrow_mut(|twap_executor| {
        let user = ic_cdk::api::msg_caller();
        twap_executor.cancel_twap(user, twap_id).ok_or("Twap not found".to_string())
    })
}

#[query]
pub fn active_user_twaps() -> Vec<u64> {
    TWAP_EXECUTOR.with_borrow_mut(|twap_executor| {
        let user = ic_cdk::api::msg_caller();
        twap_executor.get_active_user_twap_ids(&user)
    })
}

#[query]
pub fn get_twap(twap_id: u64) -> Option<Twap> {
    TWAP_EXECUTOR.with_borrow_mut(|twap_executor| {
        let user = ic_cdk::api::msg_caller();

        let twap = match twap_executor.get_twap(twap_id) {
            Some(twap) => twap,
            None => return None,
        };

        if twap.user != user {
            return None;
        }

        Some(twap)
    })
}
