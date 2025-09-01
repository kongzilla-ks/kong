use crate::{
    order_action::remove_order_args::RemoveOrderArgs,
    orderbook::{order::Order, orderbook},
};
use ic_cdk::update;

#[update]
pub async fn cancel_order(args: RemoveOrderArgs) -> Result<Order, String> {
    let orderbook = orderbook::get_orderbook(&args.symbol_0, &args.symbol_1)?;


    let order = orderbook.borrow_mut().cancel_order(args.order_id)?;

    Ok(order)
}


#[update]
pub async fn cancel_all_user_orders(receive_token: String, send_token: String) -> Result<(), String> {
    let orderbook = orderbook::get_orderbook(&receive_token, &send_token)?;
    let orders = orderbook.borrow().get_user_orders(&ic_cdk::api::msg_caller());

    for order_id in orders {
        orderbook.borrow_mut().cancel_order(order_id)?;
    }

    Ok(())
}