use crate::{
    order_action::limit_order_args::LimitOrderArgs,
    orderbook::{order_id::OrderId, price::Price},
};
use candid::{CandidType, Nat, Principal};
use kong_lib::{ic::address::Address, stable_transfer::tx_id::TxId};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, CandidType, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    Placed,
    Executing,
    Executed(u64), // request id
    Cancelled,
    Expired,
    Failed(String),
}

impl OrderStatus {
    pub fn can_be_removed(&self) -> bool {
        *self == OrderStatus::Placed
    }

    pub fn is_terminated_status(&self) -> bool {
        match self {
            OrderStatus::Placed => false,
            OrderStatus::Executing => false,
            OrderStatus::Executed(_) => true,
            OrderStatus::Cancelled => true,
            OrderStatus::Expired => true,
            OrderStatus::Failed(_) => true,
        }
    }

    pub fn need_refund(&self) -> bool {
        match self {
            OrderStatus::Placed => false,
            OrderStatus::Executing => false,
            OrderStatus::Executed(_) => false,
            OrderStatus::Cancelled => true,
            OrderStatus::Expired => true,
            OrderStatus::Failed(_) => true,
        }
    }
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OrderStatus::Placed => write!(f, "Placed"),
            OrderStatus::Executing => write!(f, "Executing"),
            OrderStatus::Executed(id) => write!(f, "Executed: {}", id),
            OrderStatus::Cancelled => write!(f, "Cancelled"),
            OrderStatus::Expired => write!(f, "Expired"),
            OrderStatus::Failed(reason) => write!(f, "Failed: {}", reason),
        }
    }
}

#[derive(Debug, Clone, CandidType, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub price: Price,
    pub user: Principal,
    pub expired_at: Option<u64>, // epoch in nanos

    pub order_status: OrderStatus,
    pub created_at: u64,
    pub finsihed_at: u64,

    pub pay_symbol: String,
    pub pay_amount: Nat,
    pub receive_symbol: String,

    pub receive_address: Address,

    pub reuse_kong_backend_pay_tx_id: Option<TxId>,
}

pub fn is_expired_ts(expires_at: Option<u64>, now: u64) -> bool {
    expires_at.map(|ts| ts <= now).unwrap_or(false)
}

impl Order {
    pub fn new(args: LimitOrderArgs, id: OrderId, user: Principal, price: Price, receive_address: Address) -> Self {
        Self {
            id: id,
            price: price,
            user: user,
            expired_at: args.expires_at(),
            order_status: OrderStatus::Placed,
            created_at: ic_cdk::api::time(),
            finsihed_at: 0,
            pay_symbol: args.pay_symbol,
            pay_amount: args.pay_amount,
            receive_symbol: args.receive_symbol,
            receive_address: receive_address,
            reuse_kong_backend_pay_tx_id: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.is_expired_ts(ic_cdk::api::time())
    }

    pub fn is_expired_ts(&self, now: u64) -> bool {
        is_expired_ts(self.expired_at, now)
    }
}
