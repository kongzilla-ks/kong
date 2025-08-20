use candid::Principal;
use ic_stable_structures::{storable::Bound, Storable};
use kong_lib::storable_hashmap::StorableHashMap;
use serde::{Deserialize, Serialize};

use crate::orderbook::{order::Order, order_id::OrderId};

#[derive(Debug, Clone)]
pub struct OrderStorage {
    orders: StorableHashMap<OrderId, Order>,
    user_orders: StorableHashMap<Principal, Vec<OrderId>>,
    sort_orders_before_serialization: bool, // sorting orders by created at is needed by active orders so users keep their positions on restarts
}

impl OrderStorage {
    pub fn new() -> Self {
        Self {
            orders: StorableHashMap::new(),
            user_orders: StorableHashMap::new(),
            sort_orders_before_serialization: false,
        }
    }

    pub fn with_sort_orders_before_serialization(mut self) -> Self {
        self.sort_orders_before_serialization = true;
        self
    }

    fn to_orders_vec(&self) -> Vec<Order> {
        self.orders.data.values().cloned().collect()
    }

    fn from_orders_vec(orders: Vec<Order>) -> Self {
        let mut res = Self::new();
        for order in orders {
            res.add_order(order);
        }
        res
    }

    pub fn orders_it(&self) -> impl Iterator<Item = &Order> {
        self.orders.data.values()
    }

    pub fn get_order(&self, order_id: &OrderId) -> Option<&Order> {
        self.orders.data.get(order_id)
    }

    pub fn get_mut_order(&mut self, order_id: &OrderId) -> Option<&mut Order> {
        self.orders.data.get_mut(order_id)
    }

    pub fn get_user_order_ids(&self, user: &Principal) -> Vec<OrderId> {
        self.user_orders.data.get(user).cloned().unwrap_or_default()
    }

    pub fn get_user_order_number(&self, user: &Principal) -> usize {
        self.user_orders.data.get(user).map(|v| v.len()).unwrap_or_default()
    }

    pub fn get_user_orders_params(&self, user: &Principal, start_ts: Option<u64>, limit: usize) -> Vec<Order> {
        let data = match self.user_orders.data.get(user) {
            Some(data) => data.iter().flat_map(|id| match self.get_order(id) {
                Some(o) => Some(o),
                None => {
                    ic_cdk::eprintln!("order not found, id={}", id);
                    None
                }
            }),
            None => return Vec::new(),
        };

        if let Some(start_ts) = start_ts {
            data.filter(|&o| o.created_at < start_ts).take(limit).cloned().collect()
        } else {
            data.take(limit).cloned().collect()
        }
    }

    pub fn add_order(&mut self, order: Order) {
        self.user_orders.data.entry(order.user).or_insert_with(Vec::new).push(order.id);
        self.orders.data.insert(order.id, order);
    }

    pub fn remove_order(&mut self, order_id: OrderId) -> Option<Order> {
        let order = self.orders.data.remove(&order_id)?;
        self.remove_user_order(&order.user, order_id);

        Some(order)
    }

    fn remove_user_order(&mut self, user: &Principal, order_id: OrderId) {
        if let Some(user_orders) = self.user_orders.data.get_mut(&user) {
            if let Some(pos) = user_orders.iter().position(|&x| x == order_id) {
                user_orders.remove(pos);

                if user_orders.is_empty() {
                    self.user_orders.data.remove(&user);
                }
            } else {
                ic_cdk::eprintln!(
                    "order found in active orders, but not found in user_orders, user={}, order_id={}",
                    user.to_text(),
                    order_id
                );
            }
        } else {
            ic_cdk::eprintln!(
                "order found in active orders, but user_orders not found, user={}, order_id={}",
                user.to_text(),
                order_id
            );
        }
    }
}

impl Default for OrderStorage {
    fn default() -> Self {
        OrderStorage::new()
    }
}

impl Storable for OrderStorage {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(&self).expect("Failed to encode OrderStorage").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode OrderStorage")
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl Serialize for OrderStorage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut orders = self.to_orders_vec();
        if self.sort_orders_before_serialization {
            orders.sort_by_key(|o| o.created_at);
        }
        orders.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OrderStorage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let orders: Vec<Order> = Vec::deserialize(deserializer)?;
        Ok(Self::from_orders_vec(orders))
    }
}
