use crate::order_action::limit_order_args::LimitOrderArgs;
use crate::orderbook::book_name::BookName;
use crate::orderbook::order::{Order, OrderStatus};
use crate::orderbook::order_history::add_order_to_history;
use crate::orderbook::order_id::OrderId;
use crate::orderbook::order_side::OrderSide;
use crate::orderbook::order_storage::OrderStorage;
use crate::orderbook::price::Price;
use crate::stable_memory::STABLE_LIMIT_ORDER_SETTINGS;
use crate::stable_memory_helpers::get_available_orderbook_name;
use candid::Principal;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use kong_lib::ic::network::ICNetwork;
use kong_lib::storable_rational::StorableRational;
use kong_lib::swap::swap_args::SwapArgs;
use serde::ser::SerializeTuple;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::time::Duration;

thread_local! {
    pub static ORDERBOOKS: RefCell<HashMap<BookName, Rc<RefCell<OrderBook>>>> = RefCell::default();
}

pub fn get_orderbook(symbol_0: String, symbol_1: String) -> Result<Rc<RefCell<OrderBook>>, String> {
    let book_name = get_available_orderbook_name(&symbol_0, &symbol_1)?;
    get_orderbook_impl(book_name)
}

fn get_orderbook_impl(bookname: BookName) -> Result<Rc<RefCell<OrderBook>>, String> {
    ORDERBOOKS.with_borrow_mut(|m| {
        let orderbook = m
            .entry(bookname.clone())
            .or_insert_with(|| Rc::new(RefCell::new(OrderBook::new(bookname))));
        Ok(orderbook.clone())
    })
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    pub name: BookName,
    last_price: Option<Price>,

    bids: BTreeMap<Reverse<Price>, VecDeque<OrderId>>,
    asks: BTreeMap<Price, VecDeque<OrderId>>,
    next_order_id: OrderId,

    active_order_storage: OrderStorage,

    orders_to_be_expired: BTreeMap<u64, HashSet<OrderId>>,
    expiration_timer_id: Option<(ic_cdk_timers::TimerId, u64)>,
}

impl OrderBook {
    pub fn new(bookname: BookName) -> Self {
        OrderBook {
            name: bookname,
            last_price: None,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            next_order_id: 1.into(),

            active_order_storage: OrderStorage::new(),
            orders_to_be_expired: BTreeMap::new(),
            expiration_timer_id: None,
        }
    }

    fn to_vec(&self) -> (BookName, Option<Price>, OrderId, OrderStorage) {
        (
            self.name.clone(),
            self.last_price.clone(),
            self.next_order_id,
            self.active_order_storage.clone().with_sort_orders_before_serialization(),
        )
    }

    fn from_vec(bookname: BookName, last_price: Option<Price>, next_order_id: OrderId, storage: OrderStorage) -> Self {
        let mut res = Self::new(bookname);
        res.last_price = last_price;
        res.next_order_id = next_order_id;

        // Fill active_order_storage, bids, asks
        for order in storage.orders_it() {
            res.add_order_impl(order.clone());
        }

        res
    }

    pub fn add_order(&mut self, swap_args: SwapArgs, limit_args: LimitOrderArgs) -> Result<Order, String> {
        let side = match self.name.symbols() {
            (s0, s1) if s0 == swap_args.pay_token.as_str() && s1 == swap_args.receive_token.as_str() => OrderSide::Sell,
            (s0, s1) if s0 == swap_args.receive_token.as_str() && s1 == swap_args.pay_token.as_str() => OrderSide::Buy,
            _ => {
                return Err(format!(
                    "Invalid orderbook, pay/receive tokens={}/{}, orderbook={}",
                    swap_args.pay_token, swap_args.receive_token, self.name
                ))
            }
        };

        let user = ICNetwork::caller();
        let max_orders_per_instruments = STABLE_LIMIT_ORDER_SETTINGS.with_borrow(|s| s.get().max_orders_per_instrument);
        if self.active_order_storage.get_user_order_number(&user) >= max_orders_per_instruments {
            return Err(format!("Max orders per instrument exceeded, limit={}", max_orders_per_instruments));
        }

        let price = StorableRational::new_str(&limit_args.price_str)?;
        // Use checked_mul in order not to panic on invalid user's input
        let expired_at = limit_args.expired_at_epoch_seconds.and_then(|v| v.checked_mul(1_000_000_000));
        let now = ic_cdk::api::time();
        let order = Order {
            id: self.next_order_id,
            price: Price::new(price),
            side: side,
            user: user,
            expired_at: expired_at,
            order_status: OrderStatus::Placed,
            created_at: now,
            finsihed_at: 0,
            swap_args: swap_args,
        };

        if order.is_expired_ts(now) {
            return Err(format!("Order is expired"));
        }

        self.next_order_id.next();

        self.add_order_impl(order.clone());

        if let Some(expired_at) = order.expired_at {
            self.add_expired_order_id(expired_at, order.id);
        }

        Ok(order)
    }

    fn add_order_impl(&mut self, order: Order) {
        match order.side {
            OrderSide::Buy => self.add_bid(order.clone()),
            OrderSide::Sell => self.add_ask(order.clone()),
        }
    }

    fn add_bid(&mut self, order: Order) {
        self.bids
            .entry(Reverse(order.price.clone()))
            .or_insert_with(VecDeque::new)
            .push_back(order.id);
        self.active_order_storage.add_order(order);
    }

    fn add_ask(&mut self, order: Order) {
        self.asks
            .entry(order.price.clone())
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
                let timer_id = ic_cdk_timers::set_timer(Duration::from_nanos(nanos), || match get_orderbook_impl(book_name) {
                    Ok(orderbook) => orderbook.borrow_mut().expiration_timer_callback(),
                    Err(_) => {}
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
                    "order found in active orders, but not found in level, order_id={}, price={}, side={}",
                    order.id,
                    order.price,
                    order.side
                )
            }
        } else {
            ic_cdk::eprintln!(
                "order found in active orders, but level not found, order_id={}, price={}, side={}",
                order.id,
                order.price,
                order.side
            )
        }
    }

    fn remove_order_from_asks(&mut self, order: &Order) {
        if let Some(level_orders) = self.asks.get_mut(&order.price) {
            if let Some(pos) = level_orders.iter().position(|&x| x == order.id) {
                level_orders.remove(pos);

                if level_orders.is_empty() {
                    self.asks.remove(&order.price);
                }
            } else {
                ic_cdk::eprintln!(
                    "order found in active orders, but not found in level, order_id={}, price={}, side={:?}",
                    order.id,
                    order.price,
                    order.side
                )
            }
        } else {
            ic_cdk::eprintln!(
                "order found in active orders, but level not found, order_id={}, price={}, side={:?}",
                order.id,
                order.price,
                order.side
            )
        }
    }

    fn get_first_bid_order_id(&self) -> Option<OrderId> {
        self.bids.iter().next().map(|v| v.1.iter().next()).flatten().cloned()
    }

    fn get_first_ask_order_id(&self) -> Option<OrderId> {
        self.asks.iter().next().map(|v| v.1.iter().next()).flatten().cloned()
    }

    pub fn get_first_sided_order_mut(&mut self, side: OrderSide) -> Option<&mut Order> {
        let order_id = match side {
            OrderSide::Buy => self.get_first_bid_order_id()?,
            OrderSide::Sell => self.get_first_ask_order_id()?,
        };
        self.active_order_storage.get_mut_order(&order_id)
    }

    pub fn get_first_bid_order(&self) -> Option<&Order> {
        let order_id = self.get_first_bid_order_id()?;
        self.active_order_storage.get_order(&order_id)
    }

    pub fn get_first_ask_order(&self) -> Option<&Order> {
        let order_id = self.get_first_ask_order_id()?;
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

        self.active_order_storage.remove_order(order_id);
        match order.side {
            OrderSide::Buy => self.remove_order_from_bids(&order),
            OrderSide::Sell => self.remove_order_from_asks(&order),
        }

        order.order_status = status;
        order.finsihed_at = ic_cdk::api::time();
        add_order_to_history(&self.name, order.clone());

        if let Some(expired_at) = order.expired_at {
            self.cancel_expired_order_id(expired_at, order.id);
        }

        Ok(order)
    }

    pub fn get_last_price(&self) -> Option<Price> {
        self.last_price.clone()
    }

    pub fn update_last_price(&mut self, new_price: Price) {
        self.last_price = Some(new_price);
    }

    pub fn bid_iter_by_price(&self, price: Price) -> Option<std::collections::vec_deque::Iter<OrderId>> {
        self.bids.get(&Reverse(price)).map(|v| v.iter())
    }

    pub fn ask_iter_by_price(&self, price: Price) -> Option<std::collections::vec_deque::Iter<OrderId>> {
        self.asks.get(&price).map(|v| v.iter())
    }

    pub fn bid_price_vec(&self, limit: usize) -> Vec<Price> {
        self.bids.keys().map(|r| r.0.clone()).take(limit).collect()
    }

    pub fn ask_price_vec(&self, limit: usize) -> Vec<Price> {
        self.asks.keys().take(limit).cloned().collect()
    }
}

impl Storable for OrderBook {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(&self).expect("Failed to encode Orderbook").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode Orderbook")
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl Serialize for OrderBook {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let vec = self.to_vec();
        let mut tup = serializer.serialize_tuple(4)?;
        tup.serialize_element(&vec.0)?;
        tup.serialize_element(&vec.1)?;
        tup.serialize_element(&vec.2)?;
        tup.serialize_element(&vec.3)?;
        tup.end()
    }
}

impl<'de> Deserialize<'de> for OrderBook {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct OrderBookVisitor;

        impl<'de> serde::de::Visitor<'de> for OrderBookVisitor {
            type Value = OrderBook;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tuple of Orderbook fields")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let book_name = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let last_price = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                let next_order_id = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
                let storage = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;

                Ok(OrderBook::from_vec(book_name, last_price, next_order_id, storage))
            }
        }

        deserializer.deserialize_tuple(4, OrderBookVisitor)
    }
}
