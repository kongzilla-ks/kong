use candid::{CandidType, Nat, Principal};
use ic_stable_structures::{storable::Bound, Storable};
use kong_lib::{
    ic::{address::Address, address_helpers::get_address, network::ICNetwork},
    stable_token::stable_token::StableToken,
    stable_transfer::tx_id::TxId,
};
use serde::{Deserialize, Serialize};

use crate::orderbook::price::Price;

#[derive(Debug, Clone, CandidType, Serialize, Deserialize, PartialEq, Eq)]
pub enum TwapStatus {
    None,
    Executing,
    Executed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, CandidType, Serialize, Deserialize)]
pub struct Twap {
    pub id: u64,
    pub user: Principal,
    pub order_period: u64, // delay between orders in nanos
    pub order_amount: usize,
    pub max_price: Option<Price>,

    pub pay_token: StableToken,
    pub pay_amount: Nat,
    pub receive_token: StableToken,

    pub pay_tx_id: Option<TxId>,
    pub receive_address: Address,

    pub twap_notional: f64,

    pub twap_status: TwapStatus,
    pub created_at: u64,
    pub total_failures: u32,
    pub consecutive_failures: u32,
    pub total_payed_amount: Nat,
    pub orders_executed: usize,
    pub received_amount: Nat,
    pub total_skipped: u32,
    pub consecutive_skipped: usize,

    pub swap_reply_request_ids: Vec<u64>,

    pub reuse_kong_backend_pay_tx_id_amount: Option<(TxId, Nat)>,

}

impl Twap {
    pub fn new(args: TwapArgs, id: u64, user: Principal, twap_notional: f64, pay_token: StableToken, receive_token: StableToken) -> Twap {
        let now = ic_cdk::api::time();
        let receive_address = args.get_receive_address().unwrap();
        let max_price = args.max_price.map(|p| Price::new_str(&p).unwrap());
        Self {
            id: id,
            user: user,
            order_period: args.order_period_ts * 1_000_000_000,
            order_amount: args.order_amount,
            max_price,

            pay_token: pay_token,
            pay_amount: args.pay_amount,
            receive_token: receive_token,

            pay_tx_id: args.pay_tx_id,
            receive_address: receive_address,

            twap_notional: twap_notional,
            twap_status: TwapStatus::None,
            created_at: now,
            total_failures: 0,
            consecutive_failures: 0,
            total_payed_amount: Nat::default(),
            orders_executed: 0,
            received_amount: Nat::default(),
            total_skipped: 0,
            consecutive_skipped: 0,

            swap_reply_request_ids: Vec::new(),
            reuse_kong_backend_pay_tx_id_amount: None,
        }
    }

    pub fn is_finished(&self) -> bool {
        match self.twap_status {
            TwapStatus::Executed | TwapStatus::Cancelled | TwapStatus::Failed => true,
            _ => false,
        }
    }
}

impl Storable for Twap {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode Twap").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode Twap")
    }

    const BOUND: ic_stable_structures::storable::Bound = Bound::Unbounded;
}

#[derive(Debug, Clone, CandidType, Serialize, Deserialize)]
pub struct TwapArgs {
    pub pay_symbol: String,
    pub pay_amount: Nat,
    pub receive_symbol: String,
    pub order_period_ts: u64, // order period in seconds
    pub order_amount: usize,
    pub max_price: Option<String>,

    pub pay_tx_id: Option<TxId>,
    pub receive_address: Option<String>,
}

impl TwapArgs {
    pub fn get_receive_address(&self) -> Result<Address, String> {
        match &self.receive_address {
            Some(address) => get_address(address).ok_or(format!("Can't get address from {}", address)),
            None => Ok(Address::PrincipalId(ICNetwork::caller_id())),
        }
    }
}




pub struct TwapReply {
    
}