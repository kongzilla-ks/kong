use candid::Principal;
use ic_cdk::{query, update};
use kong_lib::{stable_token::{ic_token::ICToken, stable_token::StableToken, token::Token}};

use crate::{stable_memory::TOKEN_MAP, stable_memory_helpers::{get_token_by_address, get_token_by_symbol}};


#[update]
pub async fn add_ic_token(token_address: &str) -> Result<StableToken, String> {
    let token_address = match token_address.split_once('.') {
        Some((_, addr)) => addr,
        None => token_address,
    };

    // Retrieves the address of the token
    // Valid and converts the address to a Principal ID
    if let Some(token) = get_token_by_address(token_address) {
        return Ok(token);
    }

    let canister_id = Principal::from_text(token_address).map_err(|e| format!("Invalid canister id {}: {}", token_address, e))?;


    // no token id is used
    let ic_token = StableToken::IC(ICToken::new(&canister_id).await?);

    if let Some(token) = get_token_by_symbol(&ic_token.symbol()) {
        return Err(format!("Token with name {} already exists", token.symbol()));
    }
    
    TOKEN_MAP.with_borrow_mut(|token_map| token_map.insert(ic_token.symbol(), ic_token.clone()));

    Ok(ic_token)
}

#[query]
pub async fn all_token_symbols() -> Vec<String> {
    TOKEN_MAP.with_borrow(|token_map| token_map.values().map(|t| t.symbol()).collect())
}