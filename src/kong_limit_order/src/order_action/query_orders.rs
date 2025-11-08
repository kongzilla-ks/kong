use std::{cell::RefCell, rc::Rc};

use crate::{
    order_action::query_orders_args::QueryOrdersArgs,
    orderbook::{
        book_name::BookName,
        order::Order,
        order_history::ORDER_HISTORY,
        order_id::OrderId,
        orderbook::{self, get_orderbook, SidedOrderBook},
        orderbook_path::is_available_token_path,
        price::Price,
    },
};
use candid::{CandidType, Nat, Principal};
use ic_cdk::query;
use kong_lib::{ic::id::caller, storable_rational::StorableRational};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct QueryOrdersResult {
    bookname: BookName,
    orders: Vec<Order>,
}

#[query]
pub fn query_active_orders(receive_token: String, send_token: String) -> Result<QueryOrdersResult, String> {
    let orderbook = orderbook::get_orderbook(&receive_token, &send_token)?;
    let user = caller();

    let orderbook = orderbook.borrow();
    let bookname = orderbook.name.clone();
    let orders = orderbook
        .get_user_orders(&user)
        .iter()
        .filter_map(|id| {
            let res = orderbook.get_order_by_order_id(id).cloned();
            if res.is_none() {
                ic_cdk::eprintln!("User order not found by id, user={}, order_id={}", user.to_text(), id);
            }
            res
        })
        .collect();

    Ok(QueryOrdersResult { bookname, orders })
}

#[query]
pub fn query_history_orders(args: QueryOrdersArgs) -> Result<QueryOrdersResult, String> {
    if !is_available_token_path(&args.receive_token, &args.send_token) {
        return Err(format!("Order path {}/{} does not exist", args.receive_token, args.send_token));
    }
    let user = caller();

    let bookname = BookName::new(&args.receive_token, &args.send_token);

    let orders = ORDER_HISTORY.with_borrow(|m| match m.get(&bookname) {
        Some(order_storage) => order_storage.get_user_orders_params(&user, args.start_ts, args.limit.unwrap_or(100).clamp(1, 100)),
        None => Vec::new(),
    });

    Ok(QueryOrdersResult { bookname, orders })
}

#[query]
pub fn query_order(receive_token: String, send_token: String, order_id: OrderId) -> Result<Order, String> {
    let user = caller();

    fn return_order(order: Order, user: Principal) -> Result<Order, String> {
        if order.user != user {
            return Err("Order does not belong to user".to_string());
        }
        return Ok(order);
    }

    let orderbook = get_orderbook(&receive_token, &send_token)?;
    let orderbook = orderbook.borrow();
    if let Some(order) = orderbook.get_order_by_order_id(&order_id).cloned() {
        return return_order(order, user);
    }

    ORDER_HISTORY.with_borrow(|m| {
        let orderbook = m.get(&orderbook.name).ok_or("Order not found".to_string())?;
        let order = orderbook.get_order(&order_id).ok_or("Order not found".to_string()).cloned()?;
        return_order(order, user)
    })
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct PriceAmount {
    price: Price,
    amount: Nat,
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct BestBidAsk {
    pub bookname: BookName,
    pub bid: Option<PriceAmount>,
    pub ask: Option<PriceAmount>,
}

fn get_level_amount(orderbook: Rc<RefCell<SidedOrderBook>>, level: Price) -> Nat {
    let orderbook = orderbook.borrow();
    let it = orderbook.bid_iter_by_price(level.clone());

    fn get_amount_from_order(o: &Order) -> Nat {
        let quote = {
            // if let Some(receive_amount) = o.limit_args.receive_amount.clone() {
            //     return receive_amount;
            // }
            o.pay_amount.clone()
        };

        // Current assumption is that amount_0 NAT / amount_1 NAT, which may differ if precision for token_0 and token_1 are different
        // TODO: is this assumption correct?

        let amount = StorableRational::new(quote, Nat::from(1u32)).unwrap() / o.price.0.clone();
        amount.round_to_nat()
    }

    let it = match it {
        Some(it) => it.map(|id| orderbook.get_order_by_order_id(id).map(get_amount_from_order).unwrap_or_default()),
        None => return Nat::default(),
    };

    it.fold(Nat::default(), |n0, n1| n0 + n1)
}

#[query]
pub async fn orderbook_l1(token_0: String, token_1: String) -> Result<BestBidAsk, String> {
    let orderbook = get_orderbook(&token_0, &token_1)?;

    let bid = orderbook.borrow().get_first_bid_order().map(|o| o.price.clone());
    let bid = bid.map(|p| PriceAmount {
        price: p.clone(),
        amount: get_level_amount(orderbook.clone(), p.clone()),
    });
    let reversed = orderbook.borrow().reversed();

    let ask = reversed.borrow().get_first_bid_order().map(|o| o.price.clone());
    let ask = ask.map(|p| PriceAmount {
        price: p.clone(),
        amount: get_level_amount(orderbook.clone(), p.clone()),
    });

    let bookname = orderbook.borrow().name.clone();
    Ok(BestBidAsk { bookname, bid, ask })
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookL2 {
    pub bookname: BookName,
    pub bids: Vec<PriceAmount>,
    pub asks: Vec<PriceAmount>,
}

#[query]
pub async fn orderbook_l2(token_0: String, token_1: String, depth: Option<usize>) -> Result<OrderbookL2, String> {
    let depth = depth.unwrap_or(10).clamp(1, 100);
    let orderbook = get_orderbook(&token_0, &token_1)?;

    let bid_prices = orderbook.borrow().bid_price_vec(depth);

    let bids: Vec<PriceAmount> = bid_prices
        .iter()
        .map(|p| PriceAmount {
            price: p.clone(),
            amount: get_level_amount(orderbook.clone(), p.clone()),
        })
        .collect();

    let reversed = orderbook.borrow().reversed();
    let asks: Vec<PriceAmount> = reversed
        .borrow()
        .bid_price_vec(depth)
        .iter()
        .map(|p| PriceAmount {
            price: p.clone(),
            amount: get_level_amount(orderbook.clone(), p.clone()),
        })
        .collect();

    let bookname = orderbook.borrow().name.clone();

    Ok(OrderbookL2 { bookname, bids, asks })
}
