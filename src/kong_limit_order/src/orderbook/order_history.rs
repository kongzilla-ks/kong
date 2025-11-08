use std::{cell::RefCell, collections::HashMap};

use candid::Principal;

use crate::{
    orderbook::{book_name::BookName, order::Order, order_id::OrderId, order_storage::OrderStorage},
};

thread_local! {
    pub static ORDER_HISTORY: RefCell<HashMap<BookName, OrderStorage>> =
        RefCell::new(HashMap::new());
}

pub fn add_order_to_history(book_name: &BookName, order: Order) {
    ORDER_HISTORY.with_borrow_mut(|m| {
        let order_storage = m.entry(book_name.clone()).or_insert_with(OrderStorage::new);
        order_storage.add_order(order);
    })
}

pub fn get_history_order(book_name: &BookName, order_id: &OrderId) -> Option<Order> {
    ORDER_HISTORY.with_borrow(|m| m.get(book_name).and_then(|s| s.get_order(&order_id).cloned()))
}

pub fn get_history_user_order_ids(book_name: &BookName, user: &Principal) -> Vec<OrderId> {
    ORDER_HISTORY
        .with_borrow(|m| m.get(book_name).map(|s| s.get_user_order_ids(user)))
        .unwrap_or(Vec::new())
}
