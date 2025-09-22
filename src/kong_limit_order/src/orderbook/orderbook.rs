use crate::order_action::limit_order_args::LimitOrderArgs;
use crate::orderbook::book_name::BookName;
use crate::orderbook::order::{Order, OrderStatus};
use crate::orderbook::order_history::add_order_to_history;
use crate::orderbook::order_id::OrderId;
use crate::orderbook::order_storage::OrderStorage;
use crate::orderbook::orderbook_path::is_available_token_path;
use crate::orderbook::price::Price;
use crate::stable_memory_helpers::{get_max_orders_per_instruments, get_token_by_symbol};
use crate::token_management;
use candid::Principal;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use kong_lib::ic::address::Address;
use kong_lib::stable_token::stable_token::StableToken;
use kong_lib::stable_token::token::Token;
use serde::ser::SerializeTuple;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::time::Duration;

thread_local! {
    pub static ORDERBOOKS: RefCell<HashMap<BookName, Rc<RefCell<SidedOrderBook>>>> = RefCell::default();
}

pub fn get_orderbook(receive_symbol: &str, send_symbol: &str) -> Result<Rc<RefCell<SidedOrderBook>>, String> {
    let bookname = BookName::new(&receive_symbol, &send_symbol);

    if is_available_token_path(receive_symbol, send_symbol) {
        return Ok(get_orderbook_nocheck(bookname));
    }

    return Err(format!("Token path {}/{} does not exist", receive_symbol, send_symbol));
}

pub fn get_orderbook_nocheck(bookname: BookName) -> Rc<RefCell<SidedOrderBook>> {
    ORDERBOOKS.with_borrow_mut(|m| {
        let orderbook = m.entry(bookname.clone()).or_insert_with(|| {
            ic_cdk::println!("Creating new orderbook: {}", bookname);
            Rc::new(RefCell::new(SidedOrderBook::new(bookname)))
        });
        orderbook.clone()
    })
}

#[derive(Debug, Clone)]
pub struct SidedOrderBook {
    pub name: BookName,
    pub send_token: StableToken,    // User's perspective
    pub receive_token: StableToken, // User's perspective

    bids: BTreeMap<Reverse<Price>, VecDeque<OrderId>>,
    next_order_id: OrderId,

    active_order_storage: OrderStorage,

    orders_to_be_expired: BTreeMap<u64, HashSet<OrderId>>,
    expiration_timer_id: Option<(ic_cdk_timers::TimerId, u64)>,
}

impl SidedOrderBook {
    pub fn new(bookname: BookName) -> Self {
        let receive_token = get_token_by_symbol(bookname.receive_token()).unwrap();
        let send_token = get_token_by_symbol(bookname.send_token()).unwrap();

        SidedOrderBook {
            name: bookname,
            send_token: send_token,
            receive_token: receive_token,
            bids: BTreeMap::new(),
            next_order_id: 1.into(),

            active_order_storage: OrderStorage::new(),
            orders_to_be_expired: BTreeMap::new(),
            expiration_timer_id: None,
        }
    }

    pub fn reversed(&self) -> Rc<RefCell<SidedOrderBook>> {
        get_orderbook_nocheck(self.name.reversed())
    }

    fn to_vec(&self) -> (BookName, OrderId, OrderStorage) {
        (
            self.name.clone(),
            self.next_order_id,
            self.active_order_storage.clone().with_sort_orders_before_serialization(),
        )
    }

    fn from_vec(bookname: BookName, next_order_id: OrderId, storage: OrderStorage) -> Self {
        let mut res = Self::new(bookname);
        res.next_order_id = next_order_id;

        // Fill active_order_storage, bids, asks
        for order in storage.orders_it() {
            res.add_order_impl(order.clone());
        }

        res
    }

    pub fn is_able_to_add_order(&self, user: &Principal) -> Result<(), String> {
        let max_orders_per_instruments = get_max_orders_per_instruments();
        if self.active_order_storage.get_user_order_number(&user) >= max_orders_per_instruments {
            return Err(format!("Max orders per instrument exceeded, limit={}", max_orders_per_instruments));
        }
        Ok(())
    }

    pub fn add_order(
        &mut self,
        limit_args: LimitOrderArgs,
        user: Principal,
        price: Price,
        receive_address: Address,
    ) -> Order {
        assert!(limit_args.receive_symbol == self.name.receive_token());
        assert!(limit_args.pay_symbol == self.name.send_token());

        let order = Order::new(limit_args, self.next_order_id, user, price, receive_address);

        self.next_order_id.next();

        self.add_order_impl(order.clone());

        if let Some(expired_at) = order.expired_at {
            self.add_expired_order_id(expired_at, order.id);
        }

        order
    }

    fn add_order_impl(&mut self, order: Order) {
        self.add_bid(order.clone());
    }

    fn add_bid(&mut self, order: Order) {
        self.bids
            .entry(Reverse(order.price.clone()))
            .or_insert_with(VecDeque::new)
            .push_back(order.id);
        self.active_order_storage.add_order(order);
    }

    fn expiration_timer_callback(&mut self) {
        ic_cdk::println!("expiration_timer_callback");
        self.expiration_time_do_action();
        self.update_expiration_timer_id();
    }

    fn expiration_time_do_action(&mut self) {
        let now = ic_cdk::api::time();
        self.expiration_timer_id = None; // trigger scheduling next timer

        let (ts_to_remove, v) = match self.orders_to_be_expired.iter().next() {
            Some((k, v)) => (k.clone(), v.clone()),
            None => return,
        };

        if ts_to_remove > now {
            return;
        }

        for id in v {
            if self
                .get_order_by_order_id(&id)
                .map(|o| o.order_status != OrderStatus::Executing)
                .unwrap_or(true)
            {
                match self.on_finished_order(id, OrderStatus::Expired) {
                    Ok(_) => ic_cdk::println!("Expired order={}", id),
                    Err(e) => ic_cdk::println!("Failed to expire order={}, err={}", id, e),
                }
            }
        }
        self.orders_to_be_expired.remove(&ts_to_remove);
    }

    fn update_expiration_timer_id(&mut self) {
        let next_ts = match self.orders_to_be_expired.iter().next().map(|v| v.0.clone()) {
            Some(ts) => {
                if let Some((timer_id, timer_ts)) = self.expiration_timer_id {
                    if timer_ts <= ts {
                        return;
                    } else {
                        ic_cdk_timers::clear_timer(timer_id);
                        self.expiration_timer_id = None;
                    }
                }
                ts
            }
            None => {
                if let Some((timer_id, _)) = self.expiration_timer_id {
                    ic_cdk_timers::clear_timer(timer_id);
                    self.expiration_timer_id = None;
                }
                return;
            }
        };

        match next_ts.checked_sub(ic_cdk::api::time()) {
            Some(nanos) => {
                let book_name = self.name.clone();
                let timer_id = ic_cdk_timers::set_timer(Duration::from_nanos(nanos), move || {
                    get_orderbook_nocheck(book_name).borrow_mut().expiration_timer_callback()
                });
                self.expiration_timer_id = Some((timer_id, next_ts));
            }
            None => {
                self.expiration_timer_callback();
            }
        };
    }

    fn add_expired_order_id(&mut self, ts: u64, order_id: OrderId) {
        self.orders_to_be_expired.entry(ts).or_insert_with(HashSet::new).insert(order_id);
        self.update_expiration_timer_id();
    }

    fn cancel_expired_order_id(&mut self, ts: u64, order_id: OrderId) {
        let mut entry = match self.orders_to_be_expired.entry(ts) {
            std::collections::btree_map::Entry::Vacant(_) => return,
            std::collections::btree_map::Entry::Occupied(occupied_entry) => occupied_entry,
        };

        entry.get_mut().remove(&order_id);
        if entry.get().len() == 0 {
            entry.remove();
        }

        self.update_expiration_timer_id();
    }

    pub fn cancel_order(&mut self, order_id: OrderId) -> Result<Order, String> {
        let order = self
            .active_order_storage
            .get_order(&order_id)
            .ok_or(format!("Order not found, id={}", order_id))?;
        if !order.order_status.can_be_removed() {
            return Err(format!(
                "Order {} can't be removed because of status={}",
                order_id, order.order_status
            ));
        }

        if order.user != ic_cdk::api::msg_caller() {
            return Err(format!("User order not found, id={}", order_id));
        }

        self.on_finished_order(order_id, OrderStatus::Cancelled)
    }

    fn remove_order_from_bids(&mut self, order: &Order) {
        if let Some(level_orders) = self.bids.get_mut(&Reverse(order.price.clone())) {
            if let Some(pos) = level_orders.iter().position(|&x| x == order.id) {
                level_orders.remove(pos);

                if level_orders.is_empty() {
                    self.bids.remove(&Reverse(order.price.clone()));
                }
            } else {
                ic_cdk::eprintln!(
                    "order found in active orders, but not found in level, name={}, order_id={}, price={}",
                    self.name,
                    order.id,
                    order.price,
                )
            }
        } else {
            ic_cdk::eprintln!(
                "order found in active orders, but level not found, name={}, order_id={}, price={}",
                self.name,
                order.id,
                order.price,
            )
        }
    }

    fn get_first_bid_order_id(&self) -> Option<OrderId> {
        self.bids.iter().next().map(|v| v.1.iter().next()).flatten().cloned()
    }

    pub fn get_first_order_mut(&mut self) -> Option<&mut Order> {
        let order_id = self.get_first_bid_order_id()?;
        self.active_order_storage.get_mut_order(&order_id)
    }

    pub fn get_first_bid_order(&self) -> Option<&Order> {
        let order_id = self.get_first_bid_order_id()?;
        self.active_order_storage.get_order(&order_id)
    }

    pub fn get_user_orders(&self, user: &Principal) -> Vec<OrderId> {
        self.active_order_storage.get_user_order_ids(user)
    }

    pub fn get_order_by_order_id(&self, order_id: &OrderId) -> Option<&Order> {
        self.active_order_storage.get_order(&order_id)
    }

    pub fn get_order_by_order_id_mut(&mut self, order_id: &OrderId) -> Option<&mut Order> {
        self.active_order_storage.get_mut_order(&order_id)
    }

    pub fn on_finished_order(&mut self, order_id: OrderId, status: OrderStatus) -> Result<Order, String> {
        if status.can_be_removed() {
            return Err(format!("Order, id={} status={} can't be removed", order_id, status));
        }
        let mut order = self
            .active_order_storage
            .get_order(&order_id)
            .ok_or(format!("Order not found, id={}", order_id))?
            .clone();
        if order.order_status == OrderStatus::Executing && (status == OrderStatus::Expired || status == OrderStatus::Cancelled) {
            return Err(format!("Can't cancel/expire executing order, id={}", order_id));
        }

        if status.need_refund() {
            let assets_to_return = match &order.reuse_kong_backend_pay_tx_id {
                Some(_txid) => {
                    // TODO: ask to send txid from kong_backend back to canister
                    order.pay_amount.clone() - self.send_token.fee() * 3u32
                },
                None => {
                    order.pay_amount.clone() - self.send_token.fee()
                },
            };

            if assets_to_return > 0u64 {
                token_management::claim_map::create_insert_and_try_to_execute(
                    order.user.clone(),
                    order.pay_symbol.clone(),
                    assets_to_return,
                    Some(order.receive_address.clone()),
                );
            }
        }

        self.active_order_storage.remove_order(order_id);
        self.remove_order_from_bids(&order);

        order.order_status = status;
        order.finsihed_at = ic_cdk::api::time();
        add_order_to_history(&self.name, order.clone());

        if let Some(expired_at) = order.expired_at {
            self.cancel_expired_order_id(expired_at, order.id);
        }

        Ok(order)
    }

    pub fn bid_iter_by_price(&self, price: Price) -> Option<std::collections::vec_deque::Iter<OrderId>> {
        self.bids.get(&Reverse(price)).map(|v| v.iter())
    }

    pub fn bid_price_vec(&self, limit: usize) -> Vec<Price> {
        self.bids.keys().map(|r| r.0.clone()).take(limit).collect()
    }
}

impl Storable for SidedOrderBook {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(&self).expect("Failed to encode Orderbook").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode Orderbook")
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl Serialize for SidedOrderBook {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let vec = self.to_vec();
        let mut tup = serializer.serialize_tuple(3)?;
        tup.serialize_element(&vec.0)?;
        tup.serialize_element(&vec.1)?;
        tup.serialize_element(&vec.2)?;
        tup.end()
    }
}

impl<'de> Deserialize<'de> for SidedOrderBook {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct OrderBookVisitor;

        impl<'de> serde::de::Visitor<'de> for OrderBookVisitor {
            type Value = SidedOrderBook;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tuple of Orderbook fields")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let book_name = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let next_order_id = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                let storage = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;

                Ok(SidedOrderBook::from_vec(book_name, next_order_id, storage))
            }
        }

        deserializer.deserialize_tuple(4, OrderBookVisitor)
    }
}
