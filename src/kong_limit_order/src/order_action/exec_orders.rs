use std::{cell::RefCell, collections::HashSet, rc::Rc, time::Duration};

use ic_cdk::update;
use kong_lib::{stable_token::stable_token::StableToken, storable_rational::StorableRational, swap::swap_reply::SwapReply};

use crate::{
    orderbook::{
        book_name::BookName, order::{self, Order, OrderStatus}, order_id::OrderId, orderbook::{self, get_orderbook_nocheck}, orderbook_path::{get_border_paths_by_id, Path, TOKEN_PATHS}, price::Price
    },
    price_observer::price_observer::{get_price, get_price_path},
    token_management::kong_interaction::{send_assets_and_swap, SendAndSwapErr},
};

const MAX_CONSECUTIVE_FAILURES: u32 = 5;
thread_local! {
    static RETRY_ORDERBOOK_EXEC: Rc<RefCell<HashSet<BookName>>> = Rc::new(RefCell::default());
}

fn add_delayed_exec_orders(mut bookname: BookName) {
    if !(bookname.receive_token() < bookname.send_token()) {
        bookname.reverse();
    }

    let new_element = RETRY_ORDERBOOK_EXEC.with(|retry| retry.borrow_mut().insert(bookname.clone()));

    if new_element {
        let delay = Duration::from_secs(60);
        ic_cdk_timers::set_timer(delay, move || {
            ic_cdk::futures::spawn(async move {
                let price = get_price(bookname.receive_token(), bookname.send_token());
                let price = match price {
                    Some(p) => p,
                    None => {
                        ic_cdk::eprintln!("Unknown price, orderbook={}", bookname);
                        return;
                    }
                };
                let _ = do_exec_orders(bookname.receive_token(), bookname.send_token(), None, price).await;
            });
        });
    }
}

// This function should be called in extraordinary cases, when user's limit order should be executed, but for some reasons it's not
#[update]
pub async fn exec_orders(receive_symbol: String, send_symbol: String, price: String) -> Result<(), String> {
    let price = Price::new(StorableRational::new_str(&price)?);
    do_exec_orders(&receive_symbol, &send_symbol, None, price).await
}

pub async fn exec_orders_on_new_best_bid(orderbook: Rc<RefCell<orderbook::SidedOrderBook>>) -> Result<(), String> {
    let res = exec_orders_on_changed_price_path(orderbook.clone(), None).await;
    match &res {
        Ok(_) => {
            ic_cdk::println!("do_exec_orders: Success");
        }
        Err(_) => {
            ic_cdk::println!("do_exec_orders: calling add_delayed_exec_orders");
            add_delayed_exec_orders(orderbook.borrow().name.clone());
        }
    };

    res
}

pub async fn do_exec_orders(receive_symbol: &str, send_symbol: &str, old_price: Option<Price>, price: Price) -> Result<(), String> {
    let res = do_exec_orders_impl(receive_symbol, send_symbol, old_price, price).await;

    match &res {
        Ok(_) => {
            ic_cdk::println!("do_exec_orders: Success");
        }
        Err(_) => {
            ic_cdk::println!("do_exec_orders: calling add_delayed_exec_orders");
            add_delayed_exec_orders(BookName::new(receive_symbol, send_symbol));
        }
    };

    res
}

async fn do_exec_orders_impl(receive_token: &str, send_token: &str, old_price: Option<Price>, price: Price) -> Result<(), String> {
    ic_cdk::println!(
        "do_exec_orders_impl: receive={}, send={}, new_price={}",
        receive_token,
        send_token,
        price
    );
    let direct_orderbook = orderbook::get_orderbook(receive_token, send_token)?;
    let rev_orderbook = direct_orderbook.borrow().reversed();

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

async fn exec_orders_in_orderbooks(changed_orderbook: Rc<RefCell<orderbook::SidedOrderBook>>) -> Result<(), String> {
    let mut id: usize = 0;
    loop {
        let path = match get_border_paths_by_id(&changed_orderbook.borrow().name, id) {
            Some(p) => p,
            None => break,
        };
        id += 1;

        let orderbook_to_exec = get_orderbook_nocheck(path.get_book_name());
        exec_orders_on_changed_price_path(orderbook_to_exec, Some(&path)).await?;
    }

    Ok(())
}

fn get_orderbook_price_path(orderbook: &Rc<RefCell<orderbook::SidedOrderBook>>, changed_path: Option<&Path>) -> Option<Price> {
    match changed_path {
        Some(changed_path) => get_price_path(changed_path),
        None => TOKEN_PATHS.with_borrow(|token_paths| {
            token_paths
                .get(&orderbook.borrow().name)?
                .iter()
                .flat_map(|path| get_price_path(path))
                .min()
        }),
    }
}

async fn exec_orders_on_changed_price_path(
    orderbook: Rc<RefCell<orderbook::SidedOrderBook>>,
    mut changed_path: Option<&Path>,
) -> Result<(), String> {
    loop {
        let price = match get_orderbook_price_path(&orderbook, changed_path) {
            Some(price) => price,
            None => {
                match changed_path {
                    Some(changed_path) => ic_cdk::eprintln!("Can't get price of path {}", changed_path),
                    None => ic_cdk::eprintln!("Can't get price of orderbook {}", orderbook.borrow().name),
                }

                return Ok(());
            }
        };

        ic_cdk::println!(
            "exec_orders_on_changed_price_path, path={:?}, price={}",
            changed_path.map(|p| p.to_string()),
            price
        );

        if !exec_order_in_orderbook_price(orderbook.clone(), &price).await? {
            break;
        }

        // While this path changed is executing, other path changes may also happen and we need to search them all. In fututre we may cache changed paths
        changed_path = None;
    }
    Ok(())
}

fn take_order_to_process(orderbook: Rc<RefCell<orderbook::SidedOrderBook>>, price: &Price) -> Option<Order> {
    let order = orderbook
        .borrow_mut()
        .get_first_order_mut()
        .filter(|o| price <= &o.price)
        .and_then(|o| {
            if o.order_status == OrderStatus::Executing {
                // Order is already being executed. There is already running loop for this orderbook, no need to have a second one
                None
            } else {
                Some(o)
            }
        })
        .cloned();

    let order = match order {
        Some(order) => order,
        None => return None,
    };

    if order.is_expired() {
        let _ = orderbook.borrow_mut().on_finished_order(order.id, OrderStatus::Expired);
        return take_order_to_process(orderbook, price);
    }

    return Some(order);
}


// Function Should remove status executing by setting new status
// Returns error if order needs to be re-executed in future
fn on_order_executed_called(
    orderbook: Rc<RefCell<orderbook::SidedOrderBook>>,
    order_id: OrderId,
    kong_response: Result<SwapReply, SendAndSwapErr>,
) -> Result<(), String> {
    let new_status = match kong_response {
        Ok(reply) => OrderStatus::Executed(reply.request_id),
        Err(send_swap_err) => {
            if send_swap_err.is_kong_error {
                match orderbook.borrow_mut().get_order_by_order_id_mut(&order_id) {
                    Some(o) => {
                        o.reuse_kong_backend_pay_tx_id = None;
                        o.total_failures += 1;
                        if o.total_failures >= MAX_CONSECUTIVE_FAILURES {
                            OrderStatus::Failed(send_swap_err.error)
                        }
                        else {
                            o.order_status = OrderStatus::Placed;
                            return Err(send_swap_err.error);
                        }
                    }
                    None => {
                        ic_cdk::eprintln!("Mark order placed error: not found, id={}", order_id);
                        return Ok(());
                    }
                }
            } else {
                // Some network error
                match orderbook.borrow_mut().get_order_by_order_id_mut(&order_id) {
                    Some(o) => {
                        o.reuse_kong_backend_pay_tx_id = send_swap_err.used_txid;
                        o.order_status = OrderStatus::Placed;
                        return Err(send_swap_err.error);
                    }
                    None => {
                        ic_cdk::eprintln!("Mark order placed error: not found, id={}", order_id);
                        return Ok(());
                    }
                }
            }
        }
    };

    ic_cdk::println!("Mark executed, id={}, status={}", order_id, new_status);

    match orderbook.borrow_mut().on_finished_order(order_id, new_status) {
        Ok(_) => {}
        Err(e) => ic_cdk::eprintln!("Mark order executed error: {e}"),
    }

    return Ok(());
}

async fn exec_order_in_orderbook_price(orderbook: Rc<RefCell<orderbook::SidedOrderBook>>, price: &Price) -> Result<bool, String> {
    let order = match take_order_to_process(orderbook.clone(), price) {
        Some(order) => order,
        None => return Ok(false),
    };

    ic_cdk::println!(
        "Take to process, orderbook={}, order={}, price={}",
        orderbook.borrow().name,
        order.id,
        price
    );

    orderbook
        .borrow_mut()
        .get_order_by_order_id_mut(&order.id)
        .map(|o| o.order_status = OrderStatus::Executing);

    let kong_response = do_order_exec(&order, &orderbook.borrow().send_token).await;
    ic_cdk::println!("exec_order_in_orderbook, best_price={}, result={:?}", price, kong_response);
    let res = on_order_executed_called(orderbook.clone(), order.id, kong_response);
    ic_cdk::println!("exec_order_in_orderbook, res={:?}", res);
    return res.map(|_| true);
}

async fn do_order_exec(order: &order::Order, pay_token: &StableToken) -> Result<SwapReply, SendAndSwapErr> {
    send_assets_and_swap(
        order.pay_amount.clone(),
        pay_token,
        order.receive_symbol.clone(),
        order.receive_address.to_string(),
        order.reuse_kong_backend_pay_tx_id.clone(),
    )
    .await
}
