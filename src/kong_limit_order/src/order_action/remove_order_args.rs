use candid::{CandidType};
use serde::{Deserialize, Serialize};

use crate::orderbook::order_id::OrderId;

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct RemoveOrderArgs {
    pub symbol_0: String,
    pub symbol_1: String,
    pub order_id: OrderId,
}

