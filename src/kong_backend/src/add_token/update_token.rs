use candid::Principal;
use ic_cdk::update;

use crate::chains::chains::{IC_CHAIN, SOL_CHAIN};
use crate::ic::guards::{caller_is_kingkong, not_in_maintenance_mode};
use crate::stable_token::ic_token::ICToken;
use crate::stable_token::solana_token::SolanaToken;
use crate::stable_token::stable_token::StableToken;
use crate::stable_token::token;
use crate::stable_token::token::Token;
use crate::stable_token::token_map;

use super::update_token_args::UpdateTokenArgs;
use super::update_token_reply::UpdateTokenReply;
use super::update_token_reply_helpers::to_update_token_reply;

/// updates the token
/// also updates
#[update(guard = "not_in_maintenance_mode")]
async fn update_token(args: UpdateTokenArgs) -> Result<UpdateTokenReply, String> {
    match token_map::get_chain(&args.token) {
        Some(chain) if chain == IC_CHAIN => to_update_token_reply(&update_ic_token(&args.token).await?),
        Some(chain) if chain == SOL_CHAIN => {
            // Solana token updates are only allowed from King Kong
            caller_is_kingkong()?;
            to_update_token_reply(&update_solana_token(&args).await?)
        }
        Some(_) | None => Err("Chain not supported")?,
    }
}

pub async fn update_ic_token(token: &str) -> Result<StableToken, String> {
    let stable_token = token_map::get_by_token(token)?;
    let address = stable_token.address();
    let token_id = stable_token.token_id();
    let symbol = stable_token.symbol();

    let canister_id = Principal::from_text(address).map_err(|e| format!("Invalid canister id {}: {}", token, e))?;

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

    token_map::get_by_token_id(token_id).ok_or_else(|| format!("Failed to update token {}", token))
}

/// Updates a Solana token's metadata
/// Only callable by King Kong to ensure metadata changes are authorized
pub async fn update_solana_token(args: &UpdateTokenArgs) -> Result<StableToken, String> {
    let stable_token = token_map::get_by_token(&args.token)?;
    
    match stable_token {
        StableToken::Solana(mut solana_token) => {
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
            
            // Update LP token symbols if they exist
            update_lp_token_symbols(&solana_token)?;
            
            token_map::get_by_token_id(solana_token.token_id)
                .ok_or_else(|| format!("Failed to update Solana token {}", args.token))
        }
        _ => Err("Token is not a Solana token".to_string()),
    }
}

/// Updates LP token symbols that include the updated Solana token
fn update_lp_token_symbols(solana_token: &SolanaToken) -> Result<(), String> {
    let symbol = &solana_token.symbol;
    
    // Update _ckUSDT pool for symbol
    let ckusdt = token_map::get_ckusdt()?;
    if let Ok(StableToken::LP(mut lp_token)) = token_map::get_by_token(&format!("LP.{}_{}", symbol, ckusdt.symbol())) {
        lp_token.symbol = token::symbol(&StableToken::Solana(solana_token.clone()), &ckusdt);
        token_map::update(&StableToken::LP(lp_token));
    }
    
    // Update _ICP pool for symbol
    let icp = token_map::get_icp()?;
    if let Ok(StableToken::LP(mut lp_token)) = token_map::get_by_token(&format!("LP.{}_{}", symbol, icp.symbol())) {
        lp_token.symbol = token::symbol(&StableToken::Solana(solana_token.clone()), &icp);
        token_map::update(&StableToken::LP(lp_token));
    }
    
    Ok(())
}
