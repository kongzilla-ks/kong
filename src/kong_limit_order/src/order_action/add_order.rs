use crate::order_action::exec_orders::exec_orders_on_new_best_bid;
use crate::orderbook;
use crate::orderbook::order::is_expired_ts;
use crate::orderbook::price::Price;
use crate::stable_memory_helpers::get_token_by_symbol;
use crate::token_management::transfer;
use crate::{order_action::limit_order_args::LimitOrderArgs, orderbook::order_id::OrderId};
use ic_cdk::update;
use icrc_ledger_types::icrc1::account::Account;
use kong_lib::ic::address::Address;
use kong_lib::ic::address_helpers::get_address;
use kong_lib::ic::network::ICNetwork;
use kong_lib::storable_rational::StorableRational;

fn get_receive_address(address: &Option<String>) -> Result<Address, String> {
    match address {
        Some(address) => get_address(address).ok_or(format!("Can't get address from {}", address)),
        None => Ok(Address::PrincipalId(ICNetwork::caller_id())),
    }
}

#[update]
pub async fn add_order(limit_args: LimitOrderArgs) -> Result<OrderId, String> {
    let orderbook = orderbook::orderbook::get_orderbook(&limit_args.receive_symbol, &limit_args.pay_symbol)?;

    // Input validation
    let pay_token = get_token_by_symbol(&limit_args.pay_symbol).ok_or(format!("Unknown token {}", limit_args.pay_symbol))?;
    let price = Price(StorableRational::new_str(&limit_args.price)?);
    let receive_address = get_receive_address(&limit_args.receive_address)?;
    if is_expired_ts(limit_args.expires_at(), ic_cdk::api::time()) {
        return Err("Order is already expired".to_string());
    }

    let user = ic_cdk::api::msg_caller();
    orderbook.borrow().is_able_to_add_order(&user)?;

    // Receiving assets
    let from_address = Address::PrincipalId(ICNetwork::caller_id());
    let kong_limit = Address::PrincipalId(Account::from(ic_cdk::api::canister_self()));
    let _received = transfer::receive_common(
        &pay_token,
        &limit_args.pay_amount,
        &kong_limit,
        &from_address,
        limit_args.pay_tx_id.clone(),
    )
    .await?;

    // Adding order
    let order = orderbook.borrow_mut().add_order(limit_args, user, price, receive_address);

    let best_order_id = orderbook.borrow().get_first_bid_order().map(|o| o.id).unwrap_or(0.into());

    if best_order_id == order.id {
        ic_cdk::futures::spawn(async move {
            let _ = exec_orders_on_new_best_bid(orderbook).await;
        });
    }

    Ok(order.id)
}
