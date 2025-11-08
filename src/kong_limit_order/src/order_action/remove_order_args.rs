use candid::{CandidType};
use serde::{Deserialize, Serialize};

use crate::orderbook::order_id::OrderId;

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct RemoveOrderArgs {
    pub receive_symbol: String,
    pub send_symbol: String,
    pub order_id: OrderId,
}

