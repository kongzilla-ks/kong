use crate::orderbook::order_side::OrderSide;
use crate::orderbook::{order_id::OrderId, price::Price};
use candid::{CandidType, Principal};
use kong_lib::swap::swap_args::SwapArgs;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, CandidType, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    Placed,
    Executing,
    Executed,
    Cancelled,
    Expired,
    Failed, // TODO: add message string?
}

impl OrderStatus {
    pub fn can_be_removed(&self) -> bool {
        *self == OrderStatus::Placed
    }

    pub fn is_terminated_status(&self) -> bool {
        match self {
            OrderStatus::Placed => false,
            OrderStatus::Executing => false,
            OrderStatus::Executed => true,
            OrderStatus::Cancelled => true,
            OrderStatus::Expired => true,
            OrderStatus::Failed => true,
        }
    }
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OrderStatus::Placed => write!(f, "Placed"),
            OrderStatus::Executing => write!(f, "Executing"),
            OrderStatus::Executed => write!(f, "Executed"),
            OrderStatus::Cancelled => write!(f, "Cancelled"),
            OrderStatus::Expired => write!(f, "Expired"),
            OrderStatus::Failed => write!(f, "Failed"),
        }
    }
}

#[derive(Debug, Clone, CandidType, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub price: Price,
    pub side: OrderSide,
    pub user: Principal,
    pub expired_at: Option<u64>, // epoch in nanos

    pub order_status: OrderStatus,
    pub created_at: u64,
    pub finsihed_at: u64,

    pub swap_args: SwapArgs,
}

impl Order {
    pub fn is_expired(&self) -> bool {
        self.is_expired_ts(ic_cdk::api::time())
    }

    pub fn is_expired_ts(&self, now: u64) -> bool {
        self.expired_at.map(|ts| ts <= now).unwrap_or(false)
    }
}
