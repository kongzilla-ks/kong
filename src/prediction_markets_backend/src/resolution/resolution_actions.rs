//! # Resolution Core Actions
//!
//! This module implements the core resolution actions like market finalization,
//! voiding, and direct resolution.

use candid::{Principal, Nat};
use ic_cdk::update;

use crate::resolution::finalize_market::finalize_market;
use crate::resolution::resolution_auth::*;
use crate::resolution::resolution_refunds::refund_all_bets;
use crate::resolution::resolution::*;
use crate::controllers::admin::*;
use crate::market::market::*;
use crate::stable_memory::*;
use crate::types::{MarketId, OutcomeIndex};
use crate::canister::get_current_time;

/// Resolves a market directly (for admin created markets)
///
/// This function handles the direct resolution path for admin-created markets,
/// bypassing the dual approval process.
///
/// # Parameters
/// * `market_id` - ID of the market to resolve
/// * `market` - The market to be resolved (mutable)
/// * `winning_outcomes` - Vector of indices for winning outcomes
/// * `resolver` - Principal ID of the admin resolving the market
///
/// # Returns
/// * `Result<(), ResolutionError>` - Success or error reason if resolution fails
pub async fn resolve_market_directly(
    market_id: MarketId,
    market: &mut Market,
    winning_outcomes: Vec<OutcomeIndex>,
    resolver: Principal
) -> Result<(), ResolutionError> {
    // First finalize the market (distribute payouts)
    finalize_market(market, winning_outcomes.clone()).await?;
    
    // Update market status to closed with the winning outcomes
    market.status = MarketStatus::Closed(
        winning_outcomes.iter().map(|idx| Nat::from(idx.clone())).collect()
    );
    
    // Record who resolved the market
    market.resolved_by = Some(resolver);
    
    // Clone market_id before using it in closures
    let market_id_clone = market_id.clone();
    
    // Update market in stable storage
    MARKETS.with(|markets| {
        let mut markets_ref = markets.borrow_mut();
        markets_ref.insert(market_id_clone.clone(), market.clone());
    });
    
    // Remove any existing resolution proposals
    RESOLUTION_PROPOSALS.with(|proposals| {
        let mut proposals = proposals.borrow_mut();
        proposals.remove(&market_id_clone);
    });
    
    Ok(())
}

/// Allows an admin to force resolve a market, bypassing the dual-approval process
///
/// This function provides a mechanism for admins to resolve markets in exceptional
/// circumstances, such as when the normal dual approval process is stuck.
///
/// # Parameters
/// * `market_id` - ID of the market to force resolve
/// * `winning_outcomes` - Vector of outcome indices that won
///
/// # Returns
/// * `Result<(), ResolutionError>` - Success or error reason if resolution fails
///
/// # Security
/// Only admins can call this function.
// Note: #[update] attribute removed to avoid conflict with the original function in dual_approval.rs
pub async fn force_resolve_market(
    market_id: MarketId,
    winning_outcomes: Vec<OutcomeIndex>
) -> Result<(), ResolutionError> {
    // Validate outcome indices are not empty
    if winning_outcomes.is_empty() {
        return Err(ResolutionError::InvalidOutcome);
    }
    
    let admin = ic_cdk::caller();
    
    // Verify the caller is an admin
    if !is_admin(admin) {
        return Err(ResolutionError::Unauthorized);
    }
    
    // Get the market
    let mut market = MARKETS.with(|markets| {
        let markets_ref = markets.borrow();
        markets_ref.get(&market_id).ok_or(ResolutionError::MarketNotFound)
    })?;
    
    // Verify market is in an active state
    if !matches!(market.status, MarketStatus::Active) {
        return Err(ResolutionError::InvalidMarketStatus);
    }
    
    // Validate outcome indices
    for outcome_index in &winning_outcomes {
        let idx = outcome_index.to_u64() as usize;
        if idx >= market.outcomes.len() {
            return Err(ResolutionError::InvalidOutcome);
        }
    }
    
    // Log the force resolution action
    ic_cdk::println!("Admin {} is force-resolving market {}", admin, market_id);
    
    // Resolve directly
    resolve_market_directly(market_id, &mut market, winning_outcomes, admin).await
}

/// Voids a market and refunds all bets to users
///
/// This function allows admins to void a market (due to ambiguous resolution,
/// technical issues, or other reasons) and ensure all users receive refunds of
/// their original bets.
///
/// # Parameters
/// * `market_id` - ID of the market to void
///
/// # Returns
/// * `Result<(), ResolutionError>` - Success or error reason if the process fails
///
/// # Security
/// Only admins can call this function.
// Note: #[update] attribute removed to avoid conflict with the original function in dual_approval.rs
pub async fn void_market(
    market_id: MarketId
) -> Result<(), ResolutionError> {
    let caller = ic_cdk::caller();
    
    // Verify caller is an admin
    if !is_admin(caller) {
        return Err(ResolutionError::Unauthorized);
    }
    
    // Get the market
    let mut market = MARKETS.with(|markets| {
        let markets_ref = markets.borrow();
        markets_ref.get(&market_id).ok_or(ResolutionError::MarketNotFound)
    })?;
    
    // Check that the market is in active status
    if !matches!(market.status, MarketStatus::Active) {
        return Err(ResolutionError::InvalidMarketStatus);
    }
    
    // Log the void action
    ic_cdk::println!("Admin {} is voiding market {}", caller, market_id);
    
    // Process refunds for all bets
    refund_all_bets(&market_id, &market).await?;
    
    // Update market status to voided
    market.status = MarketStatus::Voided;
    // Note: Market doesn't have a closed_at field in this implementation
    
    // Clone market_id for insertion to avoid ownership issues
    let market_id_clone = market_id.clone();
    
    // Update market in storage
    MARKETS.with(|markets| {
        let mut markets_ref = markets.borrow_mut();
        markets_ref.insert(market_id_clone, market);
    });
    
    // Remove any resolution proposals for this market
    RESOLUTION_PROPOSALS.with(|proposals| {
        let mut proposals = proposals.borrow_mut();
        proposals.remove(&market_id);
    });
    
    Ok(())
}
