use candid::CandidType;
use ic_cdk::api::msg_caller;
use ic_cdk::{query, update};
use kong_lib::ic::id::is_caller_controller;
use kong_lib::stable_token::token::Token;
use serde::{Deserialize, Serialize};

use crate::orderbook::orderbook_path::{BORDER_PATHS, TOKEN_PATHS};
use crate::orderbook::orderbook_path_helper::{add_to_synth_path, add_token_pair, remove_from_synth_path, remove_token_pair};
use crate::stable_memory::{STABLE_AVAILABLE_TOKEN_POOLS, STABLE_LIMIT_ORDER_SETTINGS};
use crate::stable_memory_helpers::{add_available_token_pair_impl, get_kong_backend, get_token_by_address, is_available_token_pair, remove_available_token_pair_impl};

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct OrderbookTokens {
    token_0: String,
    token_1: String,
}

#[update(hidden = true)]
pub fn add_available_token_pair(token_pair: OrderbookTokens) -> Result<(), String> {
    if msg_caller().to_string() != get_kong_backend() && !is_caller_controller() {
        return Err("Only controller is allowed to add token pairs".to_string());
    }

    let token_0 = get_token_by_address(&token_pair.token_0).ok_or(format!("Unknown token {}", token_pair.token_0))?;
    let token_1 = get_token_by_address(&token_pair.token_1).ok_or(format!("Unknown token {}", token_pair.token_1))?;

    let symbol_0 = token_0.symbol();
    let symbol_1 = token_1.symbol();

    if is_available_token_pair(&symbol_0, &symbol_1) {
        return Err(format!( "Token pair {}/{} already exists", symbol_0, symbol_1))
    }

    add_available_token_pair_impl(symbol_0.clone(), symbol_1.clone())?;

    add_token_pair(symbol_0.clone(), symbol_1.clone())?;

    let max_hops = STABLE_LIMIT_ORDER_SETTINGS.with_borrow(|s| s.get().synthetic_orderbook_max_hops);
    add_to_synth_path(&symbol_0, &symbol_1, max_hops);

    Ok(())
}

#[update(hidden = true)]
pub fn remove_available_token_pair(token_pair: OrderbookTokens) -> Result<(), String> {
    if !is_caller_controller() {
        return Err("Only controller is allowed to remove token pairs".to_string());
    }

    if !remove_available_token_pair_impl(&token_pair.token_0, &token_pair.token_1) {
        return Err(format!("Token pair {}/{} does not exist", token_pair.token_0, token_pair.token_1));
    }

    remove_token_pair(token_pair.token_0.clone(), token_pair.token_1.clone());

    remove_from_synth_path(&token_pair.token_0, &token_pair.token_1);

    Ok(())
}

// TODO: update available orderbooks from kong_backend
#[query]
fn list_available_orderbooks() -> Vec<OrderbookTokens> {
    STABLE_AVAILABLE_TOKEN_POOLS.with_borrow(|m| {
        m.iter()
            .map(|v| OrderbookTokens {
                token_0: v.receive_token().to_string(),
                token_1: v.send_token().to_string(),
            })
            .collect()
    })
}

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct OrderbookPath {
    buy_token: String,
    sell_token: String,
    paths: Vec<Vec<String>>,
}

#[query]
fn list_available_token_paths(remove_reverse: Option<bool>) -> Vec<OrderbookPath> {
    let remove_reverse = remove_reverse.unwrap_or(true);

    TOKEN_PATHS.with_borrow(|token_paths| {
        token_paths
            .iter()
            .map(|(book_name, token_path)| OrderbookPath {
                buy_token: book_name.receive_token().to_string(),
                sell_token: book_name.send_token().to_string(),
                paths: token_path.iter().map(|o| o.to_symbol_sequence()).collect(),
            })
            .filter(|op| if remove_reverse { op.buy_token < op.sell_token } else { true })
            .collect()
    })
}

#[query]
fn list_available_border_paths(remove_reverse: Option<bool>) -> Vec<OrderbookPath> {
    let remove_reverse = remove_reverse.unwrap_or(true);

    BORDER_PATHS.with_borrow(|border_paths| {
        border_paths
            .iter()
            .map(|(book_name, token_path)| OrderbookPath {
                buy_token: book_name.receive_token().to_string(),
                sell_token: book_name.send_token().to_string(),
                paths: token_path.iter().map(|o| o.to_symbol_sequence()).collect(),
            })
            .filter(|op| if remove_reverse { op.buy_token < op.sell_token } else { true })
            .collect()
    })
}
