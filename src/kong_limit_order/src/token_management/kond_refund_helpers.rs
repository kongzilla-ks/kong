use std::{cell::RefCell, rc::Rc, time::Duration};

use candid::Principal;

use crate::{stable_memory::KONG_REFUND_MAP, stable_memory_helpers::{get_and_inc_next_refund_id, get_kong_backend}, token_management::kong_refund::KongRefund};

const KONG_BACKEND_ERROR_PREFIX: &str = "Kong backend error:";
thread_local! {
    static REFUND_THREAD_RUNNING: Rc<RefCell<bool>> = Rc::new(RefCell::default());
}

fn is_refund_thread_running() -> bool {
    REFUND_THREAD_RUNNING.with(|r| r.borrow().clone())
}

fn set_refund_thread_running(v: bool) {
    REFUND_THREAD_RUNNING.with(|r| *r.borrow_mut() = v)
}

async fn refund_call(kong_refund: KongRefund) -> Result<(), String> {
    let kong_backend = Principal::from_text(get_kong_backend()).unwrap();

    ic_cdk::call::Call::unbounded_wait(kong_backend, "refund_transfer")
        .with_args(&(kong_refund.symbol, kong_refund.amount, kong_refund.sent_tx_id))
        .await
        .map_err(|e| e.to_string())?
        .candid::<Result<(), String>>()
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{} {}", KONG_BACKEND_ERROR_PREFIX, e))
}

pub async fn kong_refund_thread() {
    fn sleep_and_run_again(secs: u64) {
        let delay = Duration::from_secs(secs);
        ic_cdk_timers::set_timer(delay, || {
            ic_cdk::futures::spawn(async {
                kong_refund_thread().await;
            });
        });
    }

    while let Some((refund_id, kong_refund)) = KONG_REFUND_MAP.with_borrow(|r| if r.is_empty() { None } else { r.first_key_value() }) {
        let err = match refund_call(kong_refund.clone()).await {
            Ok(_) => {
                KONG_REFUND_MAP.with_borrow_mut(|r| r.remove(&refund_id));
                ic_cdk::println!("kong refund, refund={:?}, success", kong_refund);
                continue;
            }
            Err(e) => e,
        };

        ic_cdk::eprintln!("kong refund, refund={:?}, error: {}", kong_refund, err);
        if !err.starts_with(KONG_BACKEND_ERROR_PREFIX) {
            sleep_and_run_again(60);
            return;
        }

        if err.contains("transfer id has already been transferred")
        {
            KONG_REFUND_MAP.with_borrow_mut(|r| r.remove(&refund_id));
            continue;
        }

        // Unknown error
        sleep_and_run_again(60);
        return;
    }
}

pub fn schedule_kong_refund_if_needed() {
    if is_refund_thread_running() {
        return;
    }

    if KONG_REFUND_MAP.with_borrow(|r| r.is_empty()) {
        return;
    }

    set_refund_thread_running(true);

    ic_cdk::futures::spawn(async {
        kong_refund_thread().await;
        set_refund_thread_running(false);
    });
}

pub fn add_kong_refund(kong_refund: KongRefund) {
    let _ = KONG_REFUND_MAP.with_borrow_mut(|r| r.insert(get_and_inc_next_refund_id(), kong_refund));
    schedule_kong_refund_if_needed();
}
