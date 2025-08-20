use std::{cell::RefCell, rc::Rc};

use candid::Principal;
use ic_cdk::update;
use kong_lib::storable_rational::StorableRational;

use crate::{
    orderbook::{
        order::{self, Order, OrderStatus},
        order_side::OrderSide,
        orderbook,
        price::Price,
    },
    stable_memory::STABLE_LIMIT_ORDER_SETTINGS,
};

#[update]
pub async fn exec_orders(token_0: String, token_1: String, price: String) -> Result<(), String> {
    let price = Price::new(StorableRational::new_str(&price)?);

    let orderbook = orderbook::get_orderbook(token_0, token_1)?;
    orderbook.borrow_mut().update_last_price(price);

    exec_order_in_orderbook_loop(orderbook.clone()).await
}

pub async fn exec_order_in_orderbook_loop(orderbook: Rc<RefCell<orderbook::OrderBook>>) -> Result<(), String> {
    loop {
        if !exec_order_in_orderbook(orderbook.clone()).await? {
            break;
        }
    }

    Ok(())
}

async fn exec_order_in_orderbook(orderbook: Rc<RefCell<orderbook::OrderBook>>) -> Result<bool, String> {
    let current_price: Price = match orderbook.borrow().get_last_price() {
        Some(v) => v,
        None => {
            ic_cdk::println!("No price found for {}", orderbook.borrow().name);
            return Ok(false);
        }
    };

    fn filter_out_executing(o: &mut Order) -> Option<&mut Order> {
        if o.order_status == OrderStatus::Executing {
            // Order is already being executed
            None
        } else {
            Some(o)
        }
    }

    fn filter_by_price(o: &mut order::Order, current_price: Price) -> Option<&mut order::Order> {
        let is_ok = match o.side {
            OrderSide::Buy => current_price <= o.price,
            OrderSide::Sell => current_price >= o.price,
        };

        if is_ok {
            Some(o)
        } else {
            None
        }
    }

    fn mark_executed(orderbook: Rc<RefCell<orderbook::OrderBook>>, o: order::Order, success: Option<bool>) {
        let new_status = match success {
            Some(success) => {
                if success {
                    OrderStatus::Executed
                } else {
                    OrderStatus::Failed
                }
            }
            // some network failure
            None => match orderbook.borrow_mut().get_order_by_order_id_mut(&o.id) {
                Some(o) => {
                    if o.is_expired() {
                        OrderStatus::Expired
                    } else {
                        o.order_status = OrderStatus::Placed;
                        return;
                    }
                }
                None => {
                    ic_cdk::eprintln!("Mark order placed error: not found, id={}", o.id);
                    return;
                }
            },
        };
        match orderbook.borrow_mut().on_finished_order(o.id, new_status) {
            Ok(_) => {}
            Err(e) => ic_cdk::eprintln!("Mark order executed error: {e}"),
        }
    }

    fn take_order_to_process(orderbook: Rc<RefCell<orderbook::OrderBook>>, side: OrderSide, current_price: Price) -> Option<Order> {
        let order = orderbook
            .borrow_mut()
            .get_first_sided_order_mut(side)
            .and_then(|o| filter_by_price(o, current_price.clone()))
            .and_then(filter_out_executing)
            .cloned();

        match order {
            Some(order) => {
                if order.is_expired() {
                    let _ = orderbook.borrow_mut().on_finished_order(order.id, OrderStatus::Expired);
                    return take_order_to_process(orderbook.clone(), side, current_price.clone());
                } else {
                    Some(order)
                }
            }
            None => None,
        }
    }

    async fn take_and_process_order(orderbook: Rc<RefCell<orderbook::OrderBook>>, side: OrderSide, current_price: Price) -> bool {
        match take_order_to_process(orderbook.clone(), side, current_price) {
            Some(o) => {
                orderbook
                    .borrow_mut()
                    .get_order_by_order_id_mut(&o.id)
                    .map(|o| o.order_status = OrderStatus::Executing);
                let changed = do_order_exec(&o).await;
                mark_executed(orderbook.clone(), o, changed.clone().ok());
                changed.unwrap_or(false)
            }
            None => false,
        }
    }

    let bid_changed = take_and_process_order(orderbook.clone(), OrderSide::Buy, current_price.clone()).await;
    let ask_changed = take_and_process_order(orderbook.clone(), OrderSide::Sell, current_price.clone()).await;

    // ic_cdk::println!("exec_order_in_orderbook result={}", bid_changed || ask_changed);
    Ok(bid_changed || ask_changed)
}

pub async fn very_long_op(secs: u32, res: bool) -> Result<bool, String> {
    let kong_backend = STABLE_LIMIT_ORDER_SETTINGS.with_borrow(|s| s.get().kong_backend.clone());
    let kong_backend = Principal::from_text(kong_backend).map_err(|e| e.to_string())?;

    let res = ic_cdk::call::Call::unbounded_wait(kong_backend, "very_long_op")
        .with_args(&(secs, res))
        .await
        .map_err(|e| format!("{:?}", e))?
        .candid::<bool>()
        .map_err(|e| format!("{:?}", e))?;

    Ok(res)
}

async fn do_order_exec(_order: &order::Order) -> Result<bool, String> {
    let res = true; // _order.id.0 % 2 == 1;
    very_long_op(1, res).await
}
