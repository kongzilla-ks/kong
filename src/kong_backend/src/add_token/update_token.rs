use candid::Principal;
use ic_cdk::update;

use crate::ic::guards::{caller_is_kingkong, not_in_maintenance_mode};
use crate::stable_token::ic_token::ICToken;
use crate::stable_token::solana_token::SolanaToken;
use crate::stable_token::stable_token::StableToken;
use crate::stable_token::token;
use crate::stable_token::token::Token;
use crate::stable_token::token_map;

use super::update_token_args::UpdateTokenArgs;
use super::update_token_reply::UpdateTokenReply;

/// updates the token
/// also updates
#[update(guard = "not_in_maintenance_mode")]
async fn update_token(args: UpdateTokenArgs) -> Result<UpdateTokenReply, String> {
    let stable_token = token_map::get_by_token(&args.token)?;
    
    match stable_token {
        StableToken::IC(ic_token) => UpdateTokenReply::try_from(&update_ic_token(ic_token).await?),
        StableToken::Solana(solana_token) => {
            UpdateTokenReply::try_from(&update_solana_token(solana_token, &args).await?)
        }
        StableToken::LP(_) => Err("Cannot update LP tokens directly".to_string()),
    }
}

pub async fn update_ic_token(existing_ic_token: ICToken) -> Result<StableToken, String> {
    let address = existing_ic_token.canister_id.to_text();
    let token_id = existing_ic_token.token_id;
    let symbol = existing_ic_token.symbol.clone();

    let canister_id = Principal::from_text(&address).map_err(|e| format!("Invalid canister id {}: {}", address, e))?;

    let mut ic_token = ICToken::new(&canister_id).await?;
    ic_token.token_id = token_id;

    token_map::update(&StableToken::IC(ic_token.clone()));

    // update _ckUSDT pool for symbol
    let ckusdt = token_map::get_ckusdt()?;
    if let Ok(StableToken::LP(mut lp_token)) = token_map::get_by_token(&format!("LP.{}_{}", symbol, ckusdt.symbol())) {
        lp_token.symbol = token::symbol(&StableToken::IC(ic_token.clone()), &ckusdt);
        token_map::update(&StableToken::LP(lp_token));
    }

    // update _ICP pool for symbol
    let icp = token_map::get_icp()?;
    if let Ok(StableToken::LP(mut lp_token)) = token_map::get_by_token(&format!("LP.{}_{}", symbol, icp.symbol())) {
        lp_token.symbol = token::symbol(&StableToken::IC(ic_token), &icp);
        token_map::update(&StableToken::LP(lp_token));
    }

    token_map::get_by_token_id(token_id).ok_or_else(|| format!("Failed to update token with id {}", token_id))
}

/// Updates a Solana token's metadata
/// Only callable by King Kong to ensure metadata changes are authorized
pub async fn update_solana_token(mut solana_token: SolanaToken, args: &UpdateTokenArgs) -> Result<StableToken, String> {
    // Solana token updates are only allowed from King Kong
    caller_is_kingkong()?;
    
    // Store the old symbol before updating (needed to find existing LP tokens)
    let old_symbol = solana_token.symbol.clone();
    
    // Update metadata fields if provided
    if let Some(name) = &args.name {
        solana_token.name = name.clone();
    }
    if let Some(symbol) = &args.symbol {
        solana_token.symbol = symbol.clone();
    }
    if args.decimals.is_some() {
        return Err("Token decimals cannot be changed after creation".to_string());
    }
    
    // Update the token in storage
    token_map::update(&StableToken::Solana(solana_token.clone()));
    
    // Update LP token symbols if they exist (using old symbol to find them)
    update_lp_token_symbols(&solana_token, &old_symbol)?;
    
    token_map::get_by_token_id(solana_token.token_id)
        .ok_or_else(|| format!("Failed to update Solana token with id {}", solana_token.token_id))
}

/// Updates LP token symbols that include the updated Solana token
fn update_lp_token_symbols(solana_token: &SolanaToken, old_symbol: &str) -> Result<(), String> {
    // Update _ckUSDT pool for symbol
    let ckusdt = token_map::get_ckusdt()?;
    if let Ok(StableToken::LP(mut lp_token)) = token_map::get_by_token(&format!("LP.{}_{}", old_symbol, ckusdt.symbol())) {
        lp_token.symbol = token::symbol(&StableToken::Solana(solana_token.clone()), &ckusdt);
        token_map::update(&StableToken::LP(lp_token));
    }
    
    // Update _ICP pool for symbol
    let icp = token_map::get_icp()?;
    if let Ok(StableToken::LP(mut lp_token)) = token_map::get_by_token(&format!("LP.{}_{}", old_symbol, icp.symbol())) {
        lp_token.symbol = token::symbol(&StableToken::Solana(solana_token.clone()), &icp);
        token_map::update(&StableToken::LP(lp_token));
    }
    
    Ok(())
}