use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::orderbook::order::Order;

// TODO: deprecated

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct OrdersReply {
    pub orders: Vec<Order>,
    pub error: String,
}

impl OrdersReply {
    pub fn new(data: Result<Vec<Order>, String>) -> OrdersReply {
        match data {
            Ok(o) => OrdersReply {
                orders: o,
                error: String::new(),
            },
            Err(e) => OrdersReply {
                orders: Vec::new(),
                error: e,
            },
        }
    }
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct OrderReply {
    pub order: Option<Order>,
    pub error: String,
}

impl OrderReply {
    pub fn new(data: Result<Order, String>) -> OrderReply {
        match data {
            Ok(o) => OrderReply {
                order: Some(o),
                error: String::new(),
            },
            Err(e) => OrderReply { order: None, error: e },
        }
    }
}
