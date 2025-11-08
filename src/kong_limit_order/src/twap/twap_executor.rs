use std::{cell::RefCell, collections::HashMap, time::Duration};

use candid::{Nat, Principal};
use ic_stable_structures::{storable::Bound, Storable};
use kong_lib::{
    stable_token::{stable_token::StableToken, token::Token},
    swap::swap_reply::SwapReply,
};
use serde::{ser::SerializeTuple, Deserialize, Serialize};

use crate::{
    orderbook::{book_name::BookName, orderbook_path::TOKEN_PATHS},
    price_observer::price_observer::get_price_path,
    stable_memory_helpers::get_twap_default_seconds_delay_after_failure,
    token_management::{
        self,
        kond_refund_helpers::add_kong_refund,
        kong_interaction::{send_assets_and_swap, SendAndSwapErr},
        kong_refund::KongRefund,
    },
    twap::twap::{Twap, TwapArgs, TwapStatus},
};

const MAX_CONSECUTIVE_FAILURES: u32 = 5;
thread_local! {
    pub static TWAP_EXECUTOR: RefCell<TwapExecutor> = RefCell::default();
}

#[derive(Debug, Clone)]
pub struct TwapExecutor {
    next_twap_id: u64,
    twaps: HashMap<u64, Twap>,
    finsihed_twaps: HashMap<u64, Twap>,
    active_user_twap_ids: HashMap<Principal, Vec<u64>>,
}

impl Default for TwapExecutor {
    fn default() -> Self {
        Self {
            next_twap_id: 1,
            twaps: HashMap::new(),
            finsihed_twaps: HashMap::new(),
            active_user_twap_ids: HashMap::new(),
        }
    }
}

impl TwapExecutor {
    fn get_next_twap_id(&mut self) -> u64 {
        let res = self.next_twap_id;
        self.next_twap_id += 1;
        res
    }

    fn to_vec(&self) -> (u64, Vec<Twap>, Vec<Twap>) {
        (
            self.next_twap_id,
            self.twaps.values().cloned().collect(),
            self.finsihed_twaps.values().cloned().collect(),
        )
    }

    fn from_vec(next_twap_id: u64, twaps: Vec<Twap>, finished_twaps: Vec<Twap>) -> Self {
        let mut twap_executor = Self::default();
        twap_executor.next_twap_id = next_twap_id;

        for twap in twaps {
            twap_executor.add_twap_impl(twap);
        }

        for finished_twap in finished_twaps {
            twap_executor.finsihed_twaps.insert(finished_twap.id, finished_twap);
        }

        twap_executor
    }

    fn check_is_finished(twap_id: u64) -> bool {
        TWAP_EXECUTOR.with_borrow_mut(
            |twap_executor| match twap_executor.get_active_twap(twap_id).map(|twap| twap.is_finished()) {
                Some(finished) => {
                    if finished {
                        twap_executor.move_to_finished(twap_id);
                    }
                    finished
                }
                None => {
                    ic_cdk::eprintln!("Can't find twap id={}", twap_id);
                    true
                }
            },
        )
    }

    async fn twap_step_and_schedule(twap_id: u64) {
        if Self::check_is_finished(twap_id) {
            return;
        }
        let twap = match Self::twap_step(twap_id).await {
            Some(twap) => twap,
            None => return,
        };
        if Self::check_is_finished(twap_id) {
            return;
        }

        let delay = if twap.consecutive_failures > 0 && twap.consecutive_skipped == 0 {
            get_twap_default_seconds_delay_after_failure() * 1_000_000_000
        } else {
            twap.order_period
        };

        let delay = Duration::from_nanos(delay);

        let callback = move || {
            ic_cdk::futures::spawn(async move {
                Self::twap_step_and_schedule(twap_id).await;
            });
        };

        let _ = ic_cdk_timers::set_timer(delay, callback);
    }

    fn get_active_twap(&mut self, id: u64) -> Option<&mut Twap> {
        self.twaps.get_mut(&id)
    }

    fn add_twap_impl(&mut self, twap: Twap) {
        let twap_id = twap.id;
        let user: Principal = twap.user.clone();

        self.twaps.insert(twap.id, twap);
        self.active_user_twap_ids.entry(user).or_insert_with(Vec::new).push(twap_id);

        // This timer is reuired for proper post upgrade.
        ic_cdk_timers::set_timer(Duration::from_secs(1), move || {
            ic_cdk::futures::spawn(async move {
                Self::twap_step_and_schedule(twap_id).await;
            })
        });
    }

    pub fn add_twap(&mut self, args: TwapArgs, twap_notional: f64, pay_token: StableToken, receive_token: StableToken) -> u64 {
        let user = ic_cdk::api::msg_caller();
        let twap = Twap::new(args, self.get_next_twap_id(), user, twap_notional, pay_token, receive_token);
        let twap_id = twap.id;

        self.add_twap_impl(twap);

        twap_id
    }

    async fn twap_call_kong_swap(twap: Twap, pay_amount: Nat) -> Result<SwapReply, SendAndSwapErr> {
        send_assets_and_swap(
            pay_amount,
            &twap.pay_token,
            twap.receive_token.symbol(),
            twap.receive_address.to_string(),
            twap.reuse_kong_backend_pay_tx_id_amount.map(|v| v.0),
        )
        .await
    }

    fn get_current_pay_amount(twap: &Twap) -> Option<Nat> {
        if let Some((_, amount)) = &twap.reuse_kong_backend_pay_tx_id_amount {
            return Some(amount.clone());
        }

        ic_cdk::println!("get_current_pay_amount for twap={}", twap.id);
        if twap.orders_executed >= twap.order_amount {
            ic_cdk::eprintln!("Unexpectedly finished twap, id={}", twap.id);
            return None;
        }

        let amount_to_be_processed = twap.pay_amount.clone() - twap.total_payed_amount.clone();
        if amount_to_be_processed <= twap.pay_token.fee() {
            ic_cdk::eprintln!("Unexpectedly finished twap, id={}", twap.id);
            return None;
        }

        let orders_left = twap.order_amount - twap.orders_executed;

        if orders_left <= 1 {
            return Some(amount_to_be_processed);
        }

        return Some(amount_to_be_processed / orders_left);
    }

    // Returns true if twap is finished
    async fn twap_step(twap_id: u64) -> Option<Twap> {
        let (twap, next_amount) = match TWAP_EXECUTOR.with_borrow_mut(|twap_executor| -> Option<(Twap, Nat)> {
            let twap = match twap_executor.get_active_twap(twap_id) {
                Some(twap) => twap,
                None => {
                    ic_cdk::eprintln!("No twap found, id={}", twap_id);
                    return None;
                }
            };

            if twap.twap_status == TwapStatus::None {
                twap.twap_status = TwapStatus::Executing;
            }

            match twap.twap_status {
                TwapStatus::Executing => {}
                TwapStatus::Executed | TwapStatus::Cancelled | TwapStatus::Failed => return None,
                _ => {
                    ic_cdk::eprintln!("Status not executing, twap_id={}, status={:?}", twap.id, twap.twap_status);
                    return None;
                }
            };

            let next_amount = match Self::get_current_pay_amount(twap) {
                Some(next_amount) => next_amount,
                None => {
                    ic_cdk::eprintln!("Can't get pay amount, twap_id={}", twap.id);
                    twap.twap_status = TwapStatus::Failed;
                    return None;
                }
            };
            Some((twap.clone(), next_amount))
        }) {
            Some(twap_amount) => twap_amount,
            None => return None,
        };

        if Self::is_twap_available_by_price(&twap) {
            Self::twap_step_on_kong_swap(
                twap.id,
                next_amount.clone(),
                Self::twap_call_kong_swap(twap.clone(), next_amount).await,
            );
        } else {
            TWAP_EXECUTOR.with_borrow_mut(|twap_executor| match twap_executor.get_active_twap(twap_id) {
                Some(twap) => {
                    twap.consecutive_skipped += 1;
                    twap.total_skipped += 1;
                }
                None => ic_cdk::eprintln!("No twap found, id={}", twap_id),
            });
        }

        Some(twap)
    }

    fn is_twap_available_by_price(twap: &Twap) -> bool {
        let max_price = match &twap.max_price {
            Some(price) => price,
            None => return true,
        };

        let book_name = BookName::new(&twap.receive_token.symbol(), &twap.pay_token.symbol());
        TOKEN_PATHS.with_borrow(|token_paths| {
            let paths = match token_paths.get(&book_name) {
                Some(paths) => paths,
                None => return false, // no price exists
            };

            paths
                .iter()
                .any(|path| get_price_path(path).map(|p| &p <= max_price).unwrap_or(false))
        })
    }

    fn twap_step_on_kong_swap(twap_id: u64, next_amount: Nat, result: Result<SwapReply, SendAndSwapErr>) {
        TWAP_EXECUTOR.with_borrow_mut(|twap_executor| {
            let twap = match twap_executor.get_active_twap(twap_id) {
                Some(twap) => twap,
                None => {
                    ic_cdk::eprintln!("No twap found, id={}", twap_id);
                    return;
                }
            };
            twap.consecutive_skipped = 0;

            let swap_reply = match result {
                Ok(swap_reply) => swap_reply,
                Err(send_swap_err) => {
                    ic_cdk::eprintln!("Twap failed, error={:?}", send_swap_err);
                    // no need to send assets again in case of network issues
                    if send_swap_err.is_kong_error {
                        // Kong always sends assets back
                        twap.total_payed_amount += twap.pay_token.fee() * 2u32; // Expect kong_backend to send tokens back. Kong pays fee by himself, but I think it's users responsibility
                        twap.reuse_kong_backend_pay_tx_id_amount = None
                    } else {
                        if twap.reuse_kong_backend_pay_tx_id_amount.is_none() {
                            twap.total_payed_amount += twap.pay_token.fee() * 2u32; // Will call kong_backend refund
                            twap.reuse_kong_backend_pay_tx_id_amount = send_swap_err.used_txid.map(|txid| (txid, next_amount))
                        }
                    }
                    twap.total_failures += 1;
                    twap.consecutive_failures += 1;
                    if twap.consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                        ic_cdk::eprintln!("Twap failed, last error={}", send_swap_err.error);
                        twap.twap_status = TwapStatus::Failed;
                    }
                    return;
                }
            };

            twap.reuse_kong_backend_pay_tx_id_amount = None;
            twap.consecutive_failures = 0;
            twap.total_payed_amount += swap_reply.pay_amount + twap.pay_token.fee();
            twap.received_amount += swap_reply.receive_amount;
            twap.orders_executed += 1;
            twap.swap_reply_request_ids.push(swap_reply.request_id);

            if twap.pay_amount == twap.total_payed_amount {
                twap.twap_status = TwapStatus::Executed;
            }
        })
    }

    fn move_to_finished(&mut self, twap_id: u64) {
        let twap = match self.twaps.remove(&twap_id) {
            Some(twap) => twap,
            None => {
                ic_cdk::eprintln!("No twap found, id={}", twap_id);
                return;
            }
        };

        let amount_left = twap.pay_amount.clone() - twap.total_payed_amount.clone();
        if amount_left > twap.pay_token.fee() {
            token_management::claim_map::create_insert_and_try_to_execute(
                twap.user,
                twap.pay_token.symbol(),
                amount_left.clone() - twap.pay_token.fee(),
                Some(twap.receive_address.clone()),
            );

            match &twap.reuse_kong_backend_pay_tx_id_amount {
                Some((txid, amount)) => {
                    ic_cdk::println!(
                        "Refunding assets, {} - {} = {}",
                        amount.clone(),
                        twap.pay_token.fee(),
                        amount.clone() - twap.pay_token.fee()
                    );
                    add_kong_refund(KongRefund {
                        symbol: twap.pay_token.symbol(),
                        amount: amount.clone() - twap.pay_token.fee(),
                        sent_tx_id: txid.clone(),
                    });
                }
                None => {}
            }
        }

        self.finsihed_twaps.insert(twap.id, twap.clone());

        let active_twap_ids = match self.active_user_twap_ids.get_mut(&twap.user) {
            Some(twap_ids) => twap_ids,
            None => {
                ic_cdk::eprintln!("failed to find active user twap id, id={}", twap.id);
                return;
            }
        };

        match active_twap_ids.iter().position(|id| id == &twap.id) {
            Some(pos) => active_twap_ids.remove(pos),
            None => {
                ic_cdk::eprintln!("failed to find active user twap id, id={}", twap.id);
                return;
            }
        };

        ic_cdk::println!("Twap {} finished, status={:?}", twap.id, twap.twap_status);
    }

    pub fn cancel_twap(&mut self, user: Principal, twap_id: u64) -> Option<Twap> {
        let user_twap = self.active_user_twap_ids.get(&user).map(|v| v.contains(&twap_id)).unwrap_or(false);
        if !user_twap {
            return None;
        }

        let twap = match self.twaps.get_mut(&twap_id) {
            Some(twap) => twap,
            None => return None,
        };

        twap.twap_status = TwapStatus::Cancelled;

        // Will be moved to finished in timer thread

        return Some(twap.clone());
    }

    pub fn get_twap(&self, twap_id: u64) -> Option<Twap> {
        match self.twaps.get(&twap_id) {
            Some(twap) => return Some(twap.clone()),
            None => {}
        };

        match self.finsihed_twaps.get(&twap_id) {
            Some(twap) => return Some(twap.clone()),
            None => {}
        }

        return None;
    }

    pub fn get_active_user_twap_ids(&self, user: &Principal) -> Vec<u64> {
        self.active_user_twap_ids.get(&user).cloned().unwrap_or(Vec::new())
    }
}

impl Storable for TwapExecutor {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode TwapExecutor").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode TwapExecutor")
    }

    const BOUND: ic_stable_structures::storable::Bound = Bound::Unbounded;
}

impl Serialize for TwapExecutor {
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

impl<'de> Deserialize<'de> for TwapExecutor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TwapExecutorVisitor;

        impl<'de> serde::de::Visitor<'de> for TwapExecutorVisitor {
            type Value = TwapExecutor;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tuple of TwapExecutor fields")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let next_twap_id = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let twaps = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                let finished_twaps = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
                Ok(TwapExecutor::from_vec(next_twap_id, twaps, finished_twaps))
            }
        }

        deserializer.deserialize_tuple(4, TwapExecutorVisitor)
    }
}
