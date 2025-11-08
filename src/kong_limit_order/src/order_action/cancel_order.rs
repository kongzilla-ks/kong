use crate::{
    order_action::remove_order_args::RemoveOrderArgs,
    orderbook::{order::Order, orderbook},
};
use ic_cdk::update;

#[update]
pub async fn cancel_order(args: RemoveOrderArgs) -> Result<Order, String> {
    let orderbook = orderbook::get_orderbook(&args.receive_symbol, &args.send_symbol)?;

    let order = orderbook.borrow_mut().cancel_order(args.order_id)?;

    Ok(order)
}


#[update]
pub async fn cancel_all_user_orders(receive_symbol: String, send_symbol: String) -> Result<(), String> {
    let orderbook = orderbook::get_orderbook(&receive_symbol, &send_symbol)?;
    let orders = orderbook.borrow().get_user_orders(&ic_cdk::api::msg_caller());

    for order_id in orders {
        orderbook.borrow_mut().cancel_order(order_id)?;
    }

    Ok(())
}