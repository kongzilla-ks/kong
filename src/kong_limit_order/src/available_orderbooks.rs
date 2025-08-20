use candid::CandidType;
use ic_cdk::{query, update};
use kong_lib::ic::id::is_caller_controller;
use serde::{Deserialize, Serialize};

use crate::stable_memory_helpers::{add_available_token_pair, get_available_orderbook_name};
use crate::stable_memory::{STABLE_AVAILABLE_ORDERBOOKS};

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct OrderbookTokens {
    token_0: String,
    token_1: String,
}

#[update(hidden = true)]
fn add_available_orderbook(orderbook_tokens: OrderbookTokens) -> Result<(), String> {
    if !is_caller_controller() {
        return Err("Only controller is allowed to add orderboks".to_string());
    }

    match get_available_orderbook_name(&orderbook_tokens.token_0, &orderbook_tokens.token_1) {
        Ok(book_name) => return Err(format!("Orderbook {}/{} already exists", book_name.symbol_0(),book_name.symbol_1())),
        Err(_) => {},
    }

    add_available_token_pair(orderbook_tokens.token_0, orderbook_tokens.token_1)
}


// TODO: update available orderbooks from kong_backend
#[query]
fn list_available_orderbooks() -> Vec<OrderbookTokens> {
    STABLE_AVAILABLE_ORDERBOOKS.with_borrow(|m| {
        m.iter()
            .map(|v| OrderbookTokens {
                token_0: v.symbol_0().to_string(),
                token_1: v.symbol_1().to_string(),
            })
            .collect()
    })
}
