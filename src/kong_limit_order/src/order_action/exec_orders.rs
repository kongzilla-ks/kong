use std::{cell::RefCell, rc::Rc};

use candid::Principal;
use ic_cdk::update;
use kong_lib::{
    ic::address::Address,
    stable_token::{stable_token::StableToken, token::Token},
    stable_transfer::tx_id::TxId,
    storable_rational::StorableRational,
    swap::{swap_args::SwapArgs, swap_reply::SwapReply},
    token_management::send,
};

use crate::{
    orderbook::{
        order::{self, Order, OrderStatus},
        orderbook::{self, get_orderbook_nocheck},
        orderbook_path::{get_border_paths_by_id, Path, TOKEN_PATHS},
        price::Price,
    },
    price_observer::price_observer::get_price_path,
    stable_memory::STABLE_LIMIT_ORDER_SETTINGS,
    stable_memory_helpers::get_kong_backend,
};

const KONG_BACKEND_ERROR_PREFIX: &str = "Kong backend error:";

// This function should be called in extraordinary cases, when user's limit order should be executed, but for some reasons it's not
#[update]
pub async fn exec_orders(receive_token: String, send_token: String, price: String) -> Result<(), String> {
    let price = Price::new(StorableRational::new_str(&price)?);
    do_exec_orders(&receive_token, &send_token, None, price).await
}

pub async fn do_exec_orders(receive_token: &str, send_token: &str, old_price: Option<Price>, price: Price) -> Result<(), String> {
    ic_cdk::println!("do_exec_orders: receive={}, send={}, new_price={}", receive_token, send_token, price);
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

    // TODO: in case of network error schedule this function again?

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

        ic_cdk::println!("exec_orders_on_changed_price_path, path={:?}, price={}", changed_path.map(|p| p.to_string()), price);

        if !exec_order_in_orderbook_price(orderbook.clone(), &price).await? {
            break;
        }

        // While this path changed is executing, other path changes may also happen and we need to search them all. In fututre we may cache changed paths
        changed_path = None;
    }
    Ok(())
}

pub async fn exec_orders_on_new_best_bid(orderbook: Rc<RefCell<orderbook::SidedOrderBook>>) -> Result<(), String> {
    exec_orders_on_changed_price_path(orderbook, None).await
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

fn on_order_executed_called(
    orderbook: Rc<RefCell<orderbook::SidedOrderBook>>,
    o: order::Order,
    kong_response: Result<SwapReply, (String, Option<TxId>)>,
) -> Result<(), String> {
    let new_status = match kong_response {
        Ok(reply) => OrderStatus::Executed(reply.request_id),
        Err((e, txid)) => {
            match e.strip_prefix(KONG_BACKEND_ERROR_PREFIX) {
                Some(e) => {
                    match orderbook.borrow_mut().get_order_by_order_id_mut(&o.id) {
                        Some(o) => {
                            o.reuse_kong_backend_pay_tx_id = txid;
                        }
                        None => {
                            ic_cdk::eprintln!("Mark order placed error, txid is lost for order: not found, id={}", o.id);
                            return Ok(());
                        }
                    }

                    OrderStatus::Failed(e.to_string())
                }
                None => {
                    // Some network error
                    match orderbook.borrow_mut().get_order_by_order_id_mut(&o.id) {
                        Some(o) => {
                            o.reuse_kong_backend_pay_tx_id = None;
                            if o.is_expired() {
                                OrderStatus::Expired
                            } else {
                                o.order_status = OrderStatus::Placed;
                                return Err(e);
                            }
                        }
                        None => {
                            ic_cdk::eprintln!("Mark order placed error: not found, id={}", o.id);
                            return Ok(());
                        }
                    }
                }
            }
        }
    };

    ic_cdk::println!("Mark executed, id={}, status={}", o.id, new_status);

    match orderbook.borrow_mut().on_finished_order(o.id, new_status) {
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

    ic_cdk::println!("Take to process, orderbook={}, order={}, price={}", orderbook.borrow().name, order.id, price);

    orderbook
        .borrow_mut()
        .get_order_by_order_id_mut(&order.id)
        .map(|o| o.order_status = OrderStatus::Executing);

    let changed = do_order_exec(&order, &orderbook.borrow().send_token).await;
    ic_cdk::println!("exec_order_in_orderbook, best_price={}, result={:?}", price, changed);
    let res = on_order_executed_called(orderbook.clone(), order, changed);

    return res.map(|_| true);
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

async fn do_order_exec(order: &order::Order, pay_token: &StableToken) -> Result<SwapReply, (String, Option<TxId>)> {
    let kong_backend = Principal::from_text(get_kong_backend()).unwrap();
    let kong_backend_address: Address = Address::PrincipalId(kong_backend.into());

    let pay_amount = order.pay_amount.clone() - pay_token.fee();

    let block_id = if let Some(block_id) = &order.reuse_kong_backend_pay_tx_id {
        block_id.clone()
    } else {
        let block_id = send::send(&pay_amount, &kong_backend_address, pay_token, None)
            .await
            .map_err(|e| (e, None))?;
        TxId::BlockIndex(block_id)
    };

    // Need closure so ? exits from the closure, not function
    let kong_backend_call = async || {
        ic_cdk::call::Call::unbounded_wait(kong_backend, "swap")
            .with_arg(SwapArgs {
                pay_token: order.pay_symbol.clone(),
                pay_amount: pay_amount,
                pay_tx_id: Some(block_id.clone()),
                receive_token: order.receive_symbol.clone(),
                receive_amount: None,
                receive_address: Some(order.receive_address.to_string()),
                max_slippage: Some(100.0), // Default kong backend slippage is 2, which may fail
                referred_by: None,
                pay_signature: None,
            })
            .await
            .map_err(|e| e.to_string())?
            .candid::<Result<SwapReply, String>>()
            .map_err(|e| e.to_string())?
            .map_err(|e| format!("{} {}", KONG_BACKEND_ERROR_PREFIX, e))
    };

    kong_backend_call().await.map_err(|e| (e, Some(block_id)))
}
