use std::{cell::RefCell, rc::Rc};

use candid::Principal;
use ic_cdk::{query, update};
use kong_lib::storable_rational::StorableRational;

use crate::{
    orderbook::{
        book_name::BookName,
        order::{self, Order, OrderStatus},
        orderbook::{self, get_orderbook_nocheck, PricePath},
        orderbook_path::{get_border_paths_by_id, Path, BORDER_PATHS},
        price::Price,
    },
    stable_memory::STABLE_LIMIT_ORDER_SETTINGS,
};

#[query]
pub async fn get_prices(receive_token: String, send_token: String) -> Result<Vec<PricePath>, String> {
    let direct_orderbook = orderbook::get_orderbook(&receive_token, &send_token)?;

    let all_price_paths = direct_orderbook.borrow().get_all_price_paths();

    Ok(all_price_paths)
}

#[update]
pub async fn exec_orders(receive_token: String, send_token: String, price: String) -> Result<(), String> {
    let direct_orderbook = orderbook::get_orderbook(&receive_token, &send_token)?;
    let price = Price::new(StorableRational::new_str(&price)?);
    let old_price = direct_orderbook.borrow().get_own_price();
    update_prices(direct_orderbook.clone(), price.clone());

    let rev_orderbook = direct_orderbook.borrow().reversed();
    let rev_price = price.reversed();
    update_prices(rev_orderbook.clone(), rev_price);

    match old_price {
        None => {
            exec_orders_in_orderbooks(direct_orderbook).await?;
            exec_orders_in_orderbooks(rev_orderbook).await?;
        }
        Some(old_price) => {
            if price < old_price {
                exec_orders_in_orderbooks(direct_orderbook).await?;
            } else if price > old_price {
                exec_orders_in_orderbooks(rev_orderbook).await?;
            }
        }
    }

    Ok(())
}

fn calculate_path_price(path: &Path, borrowed_orderbook_name: &BookName, borrowed_orderbook_price: Price) -> Option<Price> {
    if path.0.first().unwrap() == borrowed_orderbook_name {
        let path_vec = Path(path.0.split_first().unwrap().1.to_vec());
        return orderbook::calculate_path_price(&path_vec).map(|p| Price(p.0 * borrowed_orderbook_price.0));
    }

    if path.0.last().unwrap() == borrowed_orderbook_name {
        let path_vec = Path(path.0.split_last().unwrap().1.to_vec());
        return orderbook::calculate_path_price(&path_vec).map(|p| Price(p.0 * borrowed_orderbook_price.0));
    }

    assert!(false, "Invalid borrowed orderbook param");
    None
}

fn update_prices(orderbook: Rc<RefCell<orderbook::SidedOrderBook>>, price: Price) {
    let mut orderbook = orderbook.borrow_mut();
    BORDER_PATHS.with_borrow(|border_paths| {
        let border_paths = match border_paths.get(&orderbook.name) {
            Some(border_paths) => border_paths,
            None => {
                ic_cdk::eprintln!("Unexpectedly empty border paths, book={}", orderbook.name);
                return;
            }
        };

        for border_path in border_paths {
            if border_path.0.len() == 1 {
                orderbook.update_price_path(price.clone(), border_path);
            } else {
                match calculate_path_price(border_path, &orderbook.name, price.clone()) {
                    Some(price) => {
                        let orderbook_for_path = get_orderbook_nocheck(border_path.get_book_name());
                        orderbook_for_path.borrow_mut().update_price_path(price, border_path);
                    }
                    None => {}
                }
            };
        }
    })
}

async fn exec_orders_in_orderbooks(changed_orderbook: Rc<RefCell<orderbook::SidedOrderBook>>) -> Result<(), String> {
    let mut id: usize = 0;
    loop {
        let path = match get_border_paths_by_id(&changed_orderbook.borrow().name, id) {
            Some(p) => p,
            None => break,
        };
        id += 1;

        let orderbook_to_exec = get_orderbook_nocheck(path.get_book_name());
        exec_order_in_orderbook_loop_changed(orderbook_to_exec, &path).await?;
    }

    Ok(())
}

async fn exec_order_in_orderbook_loop_changed(
    orderbook: Rc<RefCell<orderbook::SidedOrderBook>>,
    changed_path: &Path,
) -> Result<(), String> {
    // Go into the loop if price of changed path is the best
    let changed_price = match orderbook.borrow().get_price_by_path(changed_path) {
        Some(price) => price,
        None => {
            ic_cdk::eprintln!("Can't get price of path {}", changed_path);
            return Ok(());
        }
    };

    if orderbook
        .borrow()
        .get_best_price_path()
        .map(|p| p.0 == changed_price)
        .unwrap_or(false)
    {
        return exec_order_in_orderbook_loop(orderbook.clone()).await;
    }

    Ok(())
}

pub async fn exec_order_in_orderbook_loop(orderbook: Rc<RefCell<orderbook::SidedOrderBook>>) -> Result<(), String> {
    loop {
        if !exec_order_in_orderbook(orderbook.clone()).await? {
            break;
        }
    }

    Ok(())
}

async fn exec_order_in_orderbook(orderbook: Rc<RefCell<orderbook::SidedOrderBook>>) -> Result<bool, String> {
    let best_price: Price = match orderbook.borrow().get_best_price_path() {
        Some(v) => v.0.clone(),
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
        if current_price <= o.price {
            Some(o)
        } else {
            None
        }
    }

    fn mark_executed(orderbook: Rc<RefCell<orderbook::SidedOrderBook>>, o: order::Order, success: Option<bool>) {
        ic_cdk::println!("Mark executed, id={}, success={}", o.id, success.unwrap_or(false));
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

    fn take_order_to_process(orderbook: Rc<RefCell<orderbook::SidedOrderBook>>, current_price: Price) -> Option<Order> {
        let order = orderbook
            .borrow_mut()
            .get_first_order_mut()
            .and_then(|o| filter_by_price(o, current_price.clone()))
            .and_then(filter_out_executing)
            .cloned();

        match order {
            Some(order) => {
                if order.is_expired() {
                    let _ = orderbook.borrow_mut().on_finished_order(order.id, OrderStatus::Expired);
                    return take_order_to_process(orderbook.clone(), current_price.clone());
                } else {
                    Some(order)
                }
            }
            None => None,
        }
    }

    async fn take_and_process_order(orderbook: Rc<RefCell<orderbook::SidedOrderBook>>, current_price: Price) -> bool {
        match take_order_to_process(orderbook.clone(), current_price) {
            Some(o) => {
                ic_cdk::println!("Take order to process={}", o.id);
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

    let bid_changed = take_and_process_order(orderbook.clone(), best_price.clone()).await;

    ic_cdk::println!("exec_order_in_orderbook, best_price={}, result={}", best_price, bid_changed);
    Ok(bid_changed)
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
