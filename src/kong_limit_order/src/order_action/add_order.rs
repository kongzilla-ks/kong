use crate::order_action::exec_orders::exec_order_in_orderbook_loop;
use crate::orderbook;
use crate::orderbook::order_side::OrderSide;
use crate::{order_action::limit_order_args::LimitOrderArgs, orderbook::order_id::OrderId};
use ic_cdk::update;
use kong_lib::swap::swap_args::SwapArgs;

#[update]
pub async fn add_order(swap_args: SwapArgs, limit_args: LimitOrderArgs) -> Result<OrderId, String> {
    let orderbook = orderbook::orderbook::get_orderbook(swap_args.pay_token.clone(), swap_args.receive_token.clone())?;
    let order = orderbook.borrow_mut().add_order(swap_args, limit_args)?;

    let best_order_id = {
        if order.side == OrderSide::Buy {
            orderbook.borrow().get_first_bid_order().map(|o| o.id)
        } else {
            orderbook.borrow().get_first_ask_order().map(|o| o.id)
        }
    }
    .unwrap_or(0.into());

    ic_cdk::futures::spawn(async move {
        if best_order_id == order.id {
            let _ = exec_order_in_orderbook_loop(orderbook).await;
        }
    });

    Ok(order.id)
}
