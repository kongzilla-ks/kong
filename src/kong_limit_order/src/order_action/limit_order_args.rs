use candid::{CandidType, Nat};
use kong_lib::stable_transfer::tx_id::TxId;
use serde::{Deserialize, Serialize};


#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct LimitOrderArgs {
    pub pay_symbol: String,
    pub pay_amount: Nat,
    pub receive_symbol: String,

    pub price: String,
    pub expires_at_epoch_seconds: Option<u64>,

    pub pay_tx_id: Option<TxId>,
    pub receive_address: Option<String>,
}

impl LimitOrderArgs {
    pub fn expires_at(&self) -> Option<u64> {
        // Use checked_mul in order not to panic on invalid user's input
        self.expires_at_epoch_seconds.and_then(|v| v.checked_mul(1_000_000_000))
    }
}
