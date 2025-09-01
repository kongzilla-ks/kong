use crate::order_action::exec_orders::exec_order_in_orderbook_loop;
use crate::orderbook;
use crate::{order_action::limit_order_args::LimitOrderArgs, orderbook::order_id::OrderId};
use ic_cdk::update;
use kong_lib::swap::swap_args::SwapArgs;

#[update]
pub async fn add_order(swap_args: SwapArgs, limit_args: LimitOrderArgs) -> Result<OrderId, String> {
    let orderbook = orderbook::orderbook::get_orderbook(&swap_args.receive_token, &swap_args.pay_token)?;
    let order = orderbook.borrow_mut().add_order(swap_args, limit_args)?;

    let best_order_id = orderbook.borrow().get_first_bid_order().map(|o| o.id).unwrap_or(0.into());

    if best_order_id == order.id {
        ic_cdk::futures::spawn(async move {
            let _ = exec_order_in_orderbook_loop(orderbook).await;
        });
    }

    Ok(order.id)
}
