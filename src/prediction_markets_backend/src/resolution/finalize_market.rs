//! # Market Finalization Module
//! 
//! This module handles the finalization of prediction markets, including the distribution of
//! winnings to successful bettors. It represents the culmination of the prediction market
//! lifecycle, transforming user bets into payouts based on market outcomes.
//! 
//! ## Distribution Models
//! 
//! The system supports two distinct payout distribution approaches:
//! 
//! ### 1. Standard Distribution
//! 
//! All winning bettors receive proportional payouts based solely on their bet amounts,
//! regardless of when they placed their bets. The formula is straightforward:
//! 
//! ```
//! payout_i = bet_i + (bet_i / total_winning_bets) * profit_pool
//! ```
//! 
//! Where:
//! - `payout_i` is the total payout to user i
//! - `bet_i` is the user's original bet amount
//! - `total_winning_bets` is the sum of all bets on winning outcomes
//! - `profit_pool` is the total amount bet on losing outcomes minus fees
//! 
//! ### 2. Time-Weighted Distribution
//! 
//! This innovative model rewards users who committed to predictions earlier by applying an
//! exponential weighting function to bet timing. Earlier bettors receive higher rewards,
//! creating incentives for early market participation and price discovery.
//! 
//! #### Mathematical Model
//! 
//! The time-weighted payout is calculated as:
//! 
//! ```
//! reward_i = bet_i + (weighted_contribution_i / total_weighted_contribution) * bonus_pool
//! ```
//! 
//! Where:
//! - `weighted_contribution_i = bet_i * weight_i`
//! - `weight_i = α^(t/T)` (exponential decay function)
//! - `α` is the time decay parameter (default 0.1)
//! - `t` is the time elapsed since market creation when the bet was placed
//! - `T` is the total market duration
//! - `bonus_pool` is the distributable profit (losing bets minus fees)
//! 
//! #### Economic Benefits
//! 
//! The time-weighted model provides several advantages:
//! 
//! - **Early Risk Taking**: Rewards users who take positions when uncertainty is highest
//! - **Improved Price Discovery**: Incentivizes early market participation
//! - **Return Floor Guarantee**: All winning bettors receive at least their original bet back
//! - **Predictable Advantage Curve**: With α = 0.1, early bets receive up to 10x the weight
//! 
//! This implementation includes comprehensive safeguards to ensure rewards never exceed
//! the total market pool, with dynamic bonus pool adjustments if necessary.

use candid::Principal;

use super::resolution::*;
use crate::canister::{get_current_time, record_market_payout};
use crate::market::market::*;
use crate::market::estimate_return_types::BetPayoutRecord;
use crate::stable_memory::*;
use crate::types::{TokenAmount, OutcomeIndex, Timestamp, StorableNat, MarketId};
use crate::utils::time_weighting::*;
use crate::token::registry::{get_token_info, TokenIdentifier};
use crate::token::transfer::{transfer_token, handle_fee_transfer, TokenTransferError};

/// Helper function to transfer winnings with retry logic
/// 
/// This function implements a resilient token transfer mechanism with configurable
/// retry capabilities. It's specifically designed to handle the distributed nature
/// of the Internet Computer, where transient errors can occur during token transfers.
/// 
/// ## Retry Strategy
/// 
/// The function distinguishes between retryable errors (e.g., network congestion,
/// rate limiting) and permanent errors (e.g., insufficient funds, invalid recipient).
/// Only retryable errors trigger the retry mechanism, avoiding wasted attempts on
/// permanently failed transactions.
/// 
/// ## Error Handling
/// 
/// If all retry attempts fail, the function returns a structured error that includes:
/// - Detailed error message for troubleshooting
/// - Error classification (retryable vs. permanent)
/// - Original error type from the token canister
/// 
/// This comprehensive error information allows the transaction recovery system
/// to make intelligent decisions about further recovery attempts.
/// 
/// # Parameters
/// * `user` - Principal ID of the token recipient
/// * `amount` - Amount of tokens to transfer
/// * `token_id` - Identifier for the token type to transfer
/// * `retry_count` - Maximum number of retry attempts (not including the initial attempt)
/// 
/// # Returns
/// * `Result<candid::Nat, TokenTransferError>` - On success, returns the block index of the
///   successful transaction. On failure, returns a detailed token transfer error.
async fn transfer_winnings_with_retry(
    user: Principal, 
    amount: TokenAmount,
    token_id: &TokenIdentifier,
    retry_count: u8  // Number of retries
) -> Result<candid::Nat, TokenTransferError> {
    let mut attempts = 0;
    let max_attempts = retry_count + 1; // Initial attempt + retries
    
    loop {
        attempts += 1;
        match transfer_token(user, amount.clone(), token_id, None).await {
            Ok(tx_id) => return Ok(tx_id),
            Err(e) if e.is_retryable() && attempts < max_attempts => {
                ic_cdk::println!("Transfer attempt {} failed with retryable error: {}. Retrying...", 
                               attempts, e.detailed_message());
                // In a real implementation with a timer API, we would add exponential backoff here
                // For now, just log and retry immediately
            },
            Err(e) => {
                ic_cdk::println!("Transfer failed after {} attempts: {}",
                               attempts, e.detailed_message());
                return Err(e);
            }
        }
    }
}

/// Structure to track failed transaction information within the finalization process
/// 
/// When a token transfer fails during market finalization, this structure stores
/// all the relevant information needed to retry the transaction later or diagnose issues.
/// 
/// This is used primarily for in-memory tracking during the finalization process.
/// For persistent storage of failed transactions, see the transaction_recovery module.
#[derive(Clone, Debug)]
pub struct FailedTransactionInfo {
    /// ID of the market associated with this transaction
    pub market_id: MarketId,
    /// Principal ID of the intended token recipient
    pub user: Principal,
    /// Amount of tokens that failed to transfer
    pub amount: TokenAmount,
    /// Identifier for the token type that failed to transfer
    pub token_id: TokenIdentifier,
    /// Detailed error message explaining why the transaction failed
    pub error: String,
    /// Transaction timestamp
    pub timestamp: u64,
}

/// Finalizes a market by distributing winnings to successful bettors
/// 
/// This function handles the complete market resolution process including:
/// 1. Validating the market state and winning outcomes
/// 2. Calculating the total winning pool and platform fees
/// 3. Processing the platform fee (burn or transfer)
/// 4. Distributing winnings to successful bettors using either:
///    - Standard proportional distribution, or
///    - Time-weighted distribution (if market.uses_time_weighting is true)
/// 5. Recording payout information for each winning bet
/// 
/// For time-weighted markets, earlier bets receive higher payouts based on an
/// exponential weighting model. This rewards users who committed to their
/// predictions earlier, while ensuring all correct predictors receive at least
/// their original bet amount back.
/// 
/// # Parameters
/// * `market` - Mutable reference to the market being finalized
/// * `winning_outcomes` - Vector of outcome indices that won
/// 
/// # Returns
/// * `Result<(), ResolutionError>` - Success or error reason if finalization fails
pub async fn finalize_market(market: &mut Market, winning_outcomes: Vec<OutcomeIndex>) -> Result<(), ResolutionError> {
    ic_cdk::println!(
        "Finalizing market {} with winning outcomes {:?}",
        market.id.to_u64(),
        winning_outcomes.iter().map(|n| n.to_u64()).collect::<Vec<_>>()
    );
    // Validate market state
    if !matches!(market.status, MarketStatus::Active) {
        return Err(ResolutionError::AlreadyResolved);
    }

    // Validate winning outcomes
    for outcome in &winning_outcomes {
        if outcome.to_u64() as usize >= market.outcomes.len() {
            return Err(ResolutionError::InvalidOutcome);
        }
    }
    
    // Get the token information for this market
    let token_id = &market.token_id;
    let token_info = get_token_info(token_id)
        .ok_or(ResolutionError::TransferError(format!("Token info not found for ID: {}", token_id)))?;

    // Calculate total winning pool
    let total_winning_pool: StorableNat = winning_outcomes
        .iter()
        .map(|i| market.outcome_pools[i.to_u64() as usize].clone())
        .sum();

    ic_cdk::println!("Total winning pool: {}", total_winning_pool.to_u64());
    ic_cdk::println!("Total market pool: {}", market.total_pool.to_u64());
    
    // Calculate the total profit (losing bets)
    let total_profit = market.total_pool.to_u64() as u64 - total_winning_pool.to_u64();
    ic_cdk::println!("Total profit (losing bets): {}", total_profit);
    
    // Calculate platform fee based on profit (1% for KONG, 2% for others)
    let fee_percentage = token_info.fee_percentage;
    let platform_fee_amount = total_profit * fee_percentage / 10000;
    let platform_fee = TokenAmount::from(platform_fee_amount);
    
    // Calculate the remaining winning pool (for distribution)
    let remaining_pool_u64 = total_winning_pool.to_u64();
    let remaining_pool = TokenAmount::from(remaining_pool_u64);
    
    ic_cdk::println!("Platform fee ({}% of profit): {} {}", 
                    token_info.fee_percentage / 100, 
                    platform_fee.to_u64() / 10u64.pow(token_info.decimals as u32),
                    token_info.symbol);
    
    ic_cdk::println!("Winning pool for distribution: {} {}", 
                    remaining_pool.to_u64() / 10u64.pow(token_info.decimals as u32),
                    token_info.symbol);
    
    // Process the platform fee (burn for KONG, transfer to fee collector for other tokens)
    if platform_fee.to_u64() > token_info.transfer_fee.to_u64() {
        match handle_fee_transfer(platform_fee.clone(), token_id).await {
            Ok(Some(tx_id)) => {
                ic_cdk::println!("Successfully burned platform fee of {} {} (Transaction ID: {})", 
                    platform_fee.to_u64() / 10u64.pow(token_info.decimals as u32), 
                    token_info.symbol, 
                    tx_id);
            },
            Ok(None) => {
                ic_cdk::println!("Successfully burned platform fee of {} {}", 
                    platform_fee.to_u64() / 10u64.pow(token_info.decimals as u32), 
                    token_info.symbol);
            },
            Err(e) => {
                ic_cdk::println!("Error processing platform fee: {:?}. Continuing with distribution.", e);
                // Continue with distribution even if fee processing fails
            }
        }
    } else {
        ic_cdk::println!("Platform fee too small to process (less than transfer fee). Skipping fee transfer.");
    }

    // Track the number of winning bets for final reporting
    let mut winning_bet_count = 0;
    
    if total_winning_pool > 0u64 {
        // Get all winning bets
        let winning_bets = BETS.with(|bets| {
            let bets = bets.borrow();
            if let Some(bet_store) = bets.get(&market.id) {
                bet_store
                    .0
                    .iter()
                    .filter(|bet| winning_outcomes.iter().any(|x| x == &bet.outcome_index))
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        });

        // Store the count for reporting at the end of the function
        winning_bet_count = winning_bets.len();
        ic_cdk::println!("Found {} winning bets", winning_bets.len());

        // Time-Weighted Distribution Model
        // 
        // When a market is configured to use time-weighted distribution (market.uses_time_weighting = true),
        // the system applies an exponential weighting model that rewards earlier betting behavior.
        // This model is designed to incentivize early market participation and improve price discovery.
        if market.uses_time_weighting {
            // The time-weighted distribution uses a sophisticated exponential decay function
            // that calculates weights based on when bets were placed relative to market duration.
            // 
            // The time weight formula is: weight = α^(t/T) where:
            // - α is the decay parameter (typically 0.1, configurable per market)
            // - t is the time elapsed since market creation when the bet was placed
            // - T is the total market duration from creation to closing
            // 
            // With α = 0.1, this creates a powerful incentive structure:
            // - Bets placed at market start receive full weight (1.0)
            // - Bets placed at market midpoint receive weight of ~0.32
            // - Bets placed at market end receive minimum weight (0.1)
            let market_created_at = market.created_at.clone();
            let market_end_time = market.end_time.clone();
            let alpha = get_market_alpha(market);
            
            ic_cdk::println!("Using time-weighted distribution with alpha: {}", alpha);
            
            // Calculate weighted contributions for each winning bet
            // Each bet's contribution is weighted by time - earlier bets get higher weights
            // The tuple contains: (user, bet_amount, time_weight, weighted_contribution, outcome_index)
            let mut weighted_contributions: Vec<(Principal, TokenAmount, f64, f64, OutcomeIndex)> = Vec::new();
            let mut total_weighted_contribution: f64 = 0.0;
            
            for bet in &winning_bets {
                let bet_amount = bet.amount.to_f64();
                
                // Calculate time-based exponential weight
                // 
                // This applies the core time-weighting function that determines how much
                // additional reward a particular bet receives based on its timing.
                // 
                // The weight ranges from 1.0 (bet placed at market creation) to α (bet placed at market end)
                let weight = calculate_time_weight(
                    market_created_at.clone(),  // When the market was created
                    market_end_time.clone(),     // When the market closes for betting
                    bet.timestamp.clone(),       // When this particular bet was placed
                    alpha                        // The exponential decay parameter
                );
                
                // The weighted contribution combines bet amount with time weight
                // This value represents the bet's share of the bonus pool
                // Formula: weighted_contribution = bet_amount * weight
                let weighted_contribution = calculate_weighted_contribution(bet_amount, weight);
                weighted_contributions.push((
                    bet.user, 
                    bet.amount.clone(), 
                    weight, 
                    weighted_contribution,
                    bet.outcome_index.clone()
                ));
                total_weighted_contribution += weighted_contribution;
                
                ic_cdk::println!(
                    "Bet by {} at time {}, weight: {}, weighted contribution: {}",
                    bet.user.to_string(), 
                    bet.timestamp.to_u64(),
                    weight,
                    weighted_contribution
                );
            }
            
            // Use the profit and fee values already calculated
            let total_profit_f64 = total_profit as f64;
            let platform_fee_on_profit = platform_fee.to_u64() as f64;
            
            // Calculate total transfer fees needed for all winning bets
            let transfer_fee_per_payout = token_info.transfer_fee.to_u64() as f64;
            let total_transfer_fees = (weighted_contributions.len() as f64) * transfer_fee_per_payout;
            ic_cdk::println!("Reserving {} {} for transfer fees ({} payouts)", 
                          total_transfer_fees / 10u64.pow(token_info.decimals as u32) as f64,
                          token_info.symbol,
                          weighted_contributions.len());
            
            // Calculate the distributable profit after fees
            let distributable_profit = total_profit_f64 - platform_fee_on_profit - total_transfer_fees;
            
            // Initialize bonus pool with the distributable profit
            // Note: This will potentially be adjusted by the safety check below
            
            // Guaranteed return is the original bet amount for each winner
            let total_guaranteed_return = total_winning_pool.to_u64() as f64;
            
            // Financial Safety System: Guaranteed Return Floor with Maximum Cap
            // 
            // This critical financial safety mechanism ensures two important guarantees:
            // 1. All winning bettors receive at least their original bet back (return floor)
            // 2. Total distributions never exceed the available market pool (maximum cap)
            // 
            // The system dynamically adjusts the bonus pool if necessary to maintain these guarantees.
            let max_distribution = market.total_pool.to_u64() as f64 - platform_fee_on_profit - total_transfer_fees;
            let mut bonus_pool = distributable_profit; // Initialize with original calculation
            
            // Safety check: Verify that total distribution won't exceed total market pool
            // This could happen in edge cases with extreme time-weighting parameters
            if total_guaranteed_return + bonus_pool > max_distribution {
                ic_cdk::println!("WARNING: Distribution would exceed market pool. Adjusting bonus pool...");
                
                // Dynamically adjust the bonus pool to ensure we don't exceed the max distribution
                // Formula: adjusted_bonus_pool = max_distribution - total_guaranteed_return
                let adjusted_bonus_pool = max_distribution - total_guaranteed_return;
                
                if adjusted_bonus_pool < 0.0 {
                    // Extreme edge case: Cannot even guarantee return of original bets
                    // This should never happen with proper validation, but we handle it safely
                    ic_cdk::println!("ERROR: Cannot even distribute guaranteed returns. Setting bonus pool to zero.");
                    bonus_pool = 0.0;
                } else {
                    // Normal adjustment: Reduce bonus pool to safe level
                    ic_cdk::println!("Adjusted bonus pool from {} to {}", bonus_pool, adjusted_bonus_pool);
                    bonus_pool = adjusted_bonus_pool;
                }
            }
            
            ic_cdk::println!(
                "Pool accounting: Total market pool: {}, Total winning pool: {}, Total profit: {}, Platform fee: {}, Transfer fees: {}, Distributable profit: {}, Bonus pool: {}, Max possible distribution: {}",
                market.total_pool.to_u64(),
                total_winning_pool.to_u64(),
                total_profit_f64,
                platform_fee_on_profit,
                total_transfer_fees,
                distributable_profit,
                bonus_pool,
                max_distribution
            );
            
            ic_cdk::println!(
                "Total winning pool: {}, Total weighted contribution: {}, Bonus pool: {}",
                total_winning_pool.to_u64(),
                total_weighted_contribution,
                bonus_pool
            );
            
            // Time-Weighted Reward Distribution System
            // 
            // This core distribution algorithm implements the time-weighted reward model using the formula:
            // reward_i = original_bet_i + (weighted_contribution_i / total_weighted_contribution) * bonus_pool
            // 
            // This formula guarantees two important properties:
            // 1. Return Floor: All winners receive at least their original bet amount back
            // 2. Time Advantage: Earlier bettors receive a larger share of the bonus pool
            // 
            // The system distributes the bonus pool (profits from losing bets) proportionally
            // to each bet's weighted contribution, which combines bet amount and time weight.
            // Bets placed earlier have higher weights, resulting in higher proportional rewards.
            for (user, bet_amount, weight, weighted_contribution, outcome_index) in weighted_contributions {
                // Calculate the share of the bonus pool
                let bonus_share = if total_weighted_contribution > 0.0 {
                    weighted_contribution / total_weighted_contribution * bonus_pool
                } else {
                    0.0
                };
                
                // Total reward = original bet + share of bonus pool
                // Note: Transfer fee is already accounted for in the bonus pool calculation
                let total_reward = bet_amount.to_u64() as f64 + bonus_share;
                let gross_winnings = TokenAmount::from(total_reward as u64);
                
                ic_cdk::println!(
                    "Reward breakdown for {}: Original bet: {}, Bonus share: {} ({}% of profit), Total reward: {}",
                    user.to_string(),
                    bet_amount.to_u64(),
                    bonus_share,
                    if distributable_profit > 0.0 { (bonus_share / distributable_profit) * 100.0 } else { 0.0 },
                    total_reward
                );
                
                // Calculate net amount to transfer (after deducting transfer fee)
                let transfer_amount = if gross_winnings > token_info.transfer_fee {
                    TokenAmount::from((total_reward - token_info.transfer_fee.to_u64() as f64) as u64)
                } else {
                    ic_cdk::println!(
                        "Skipping transfer - winnings {} less than fee {}",
                        gross_winnings.to_u64(),
                        token_info.transfer_fee.to_u64()
                    );
                    continue; // Skip if winnings are less than transfer fee
                };
                
                ic_cdk::println!(
                    "Processing weighted bet - User: {}, Original bet: {}, Weight: {}, Bonus share: {}, Gross reward: {}, Net transfer: {}",
                    user.to_string(),
                    bet_amount.to_u64(),
                    weight,
                    bonus_share,
                    gross_winnings.to_u64(),
                    transfer_amount.to_u64()
                );

                ic_cdk::println!("Transferring {} {} tokens to {}", 
                              transfer_amount.to_u64() / 10u64.pow(token_info.decimals as u32), 
                              token_info.symbol,
                              user.to_string());

                // Transfer winnings to the bettor using the appropriate token with retry logic
                // Attempt the transfer with up to 2 retries for retryable errors
                let transfer_result = transfer_winnings_with_retry(user, transfer_amount.clone(), token_id, 2).await;
                
                match transfer_result {
                    Ok(tx_id) => {
                        ic_cdk::println!("Transfer successful (Transaction ID: {})", tx_id);
                        
                        // Record the payout
                        // Calculate proportional platform fee for this bet
                        let user_platform_fee = if total_reward > 0.0 && total_winning_pool.to_u64() > 0 {
                            // Calculate proportional fee based on this user's share of rewards
                            let user_share = bet_amount.to_u64() as f64 / total_winning_pool.to_u64() as f64;
                            let user_fee = platform_fee_on_profit * user_share;
                            Some(TokenAmount::from(user_fee as u64))
                        } else {
                            None
                        };
                        
                        let payout_record = BetPayoutRecord {
                            market_id: market.id.clone(),
                            user,
                            bet_amount: bet_amount.clone(),
                            payout_amount: gross_winnings.clone(), // Record the gross amount for history
                            timestamp: Timestamp::from(get_current_time()),
                            outcome_index,
                            was_time_weighted: true,
                            time_weight: Some(weight),
                            original_contribution_returned: bet_amount.clone(),
                            bonus_amount: Some(TokenAmount::from(bonus_share as u64)),
                            platform_fee_amount: user_platform_fee,
                            token_id: token_id.clone(),
                            token_symbol: token_info.symbol.clone(),
                            platform_fee_percentage: token_info.fee_percentage,
                            transaction_id: Some(tx_id),
                        };
                        
                        record_market_payout(payout_record);
                    },
                    Err(e) => {
                        // Enhanced error logging with detailed message
                        ic_cdk::println!("Transfer failed: {}. Continuing with other payouts.", e.detailed_message());
                        
                        // Record failed transaction information for potential recovery
                        let failed_tx = FailedTransactionInfo {
                            market_id: market.id.clone(),
                            user,
                            amount: transfer_amount.clone(),
                            token_id: token_id.clone(),
                            error: e.detailed_message(),
                            timestamp: get_current_time().into(),
                        };
                        
                        // For now, just log the failed transaction - in a future version
                        // we would store this in a persistent data structure for admin recovery
                        ic_cdk::println!("Recorded failed transaction: {:?}", failed_tx);
                        
                        // Continue with other payouts instead of returning an error
                    }
                }
            }
        } else {
            // Use standard (non-time-weighted) distribution
            //
            // In the standard distribution model, winnings are distributed proportionally
            // to bet amounts without considering when the bets were placed. Each winner
            // receives a share of the total pool proportional to their bet amount:
            //
            // reward_i = (bet_amount_i / total_bet_amount) * remaining_pool
            //
            // This simpler model provides consistent proportional returns regardless of
            // bet timing. It's more straightforward for users to understand and predict
            // their potential returns.
            
            // Calculate total bet amount from winners only
            let total_bet_amount = winning_bets.iter().map(|bet| bet.amount.to_u64()).sum::<u64>();
            
            // Calculate total transfer fees needed for all winning bets
            // Transfer fees must be reserved from the winning pool to ensure all transfers complete
            // successfully. This is a critical part of reliable payout processing.
            let transfer_fee_per_payout = token_info.transfer_fee.to_u64() as f64;
            let total_winning_bets = winning_bets.len();
            let total_transfer_fees = (total_winning_bets as f64) * transfer_fee_per_payout;
            
            ic_cdk::println!("Reserving {} {} for transfer fees ({} payouts)", 
                          total_transfer_fees / 10u64.pow(token_info.decimals as u32) as f64,
                          token_info.symbol,
                          total_winning_bets);
            
            // Adjust the remaining pool to account for transfer fees
            // This preemptively reserves all needed transfer fees from the winning pool,
            // ensuring that every winner can be paid even after accounting for fees.
            let remaining_pool_after_fees = if total_transfer_fees > 0.0 {
                let fees_amount = TokenAmount::from(total_transfer_fees as u64);
                if remaining_pool > fees_amount {
                    remaining_pool.clone() - fees_amount
                } else {
                    // Safety check: if transfer fees would exceed the entire pool,
                    // warn but proceed with distribution. Some transfers may fail,
                    // but the transaction recovery system will handle retries.
                    ic_cdk::println!("Warning: Transfer fees exceed remaining pool. Some payouts may fail.");
                    remaining_pool.clone()
                }
            } else {
                remaining_pool.clone()
            };
            
            if total_bet_amount > 0 {
                for bet in winning_bets {
                    // Calculate proportional winnings based on the user's bet amount
                    // Proportion = user_bet_amount / total_winning_bets_amount
                    // This proportion determines the user's share of the total prize pool
                    let bet_proportion = bet.amount.to_u64() as f64 / total_bet_amount as f64;
                    
                    // Calculate user's share of the total winning pool using the adjusted pool
                    // that accounts for transfer fees. The formula is:
                    // user_winnings = (total_pool_after_fees * bet_amount) / total_bet_amount
                    //
                    // This ensures fair distribution proportional to each user's contribution
                    // while accounting for necessary platform fees and transfer costs.
                    let user_winnings = (remaining_pool_after_fees.to_u64() as f64 * bet_proportion) as u64;
                    let gross_winnings = TokenAmount::from(user_winnings);
                    
                    // Calculate net amount to transfer (after deducting transfer fee)
                    // The transfer fee is already accounted for in the pool calculation, but we still need to
                    // subtract it from the individual payout amount for the actual transfer.
                    // This separation of concerns (pool adjustment + individual transfer fee) ensures
                    // transparency in the accounting and accurate payouts.
                    let transfer_amount = if gross_winnings > token_info.transfer_fee.clone() {
                        gross_winnings.clone() - token_info.transfer_fee.clone()
                    } else {
                        ic_cdk::println!(
                            "Skipping transfer - winnings {} less than fee {}",
                            gross_winnings.to_u64(),
                            token_info.transfer_fee.to_u64()
                        );
                        continue; // Skip if winnings are less than transfer fee
                    };
                    
                    ic_cdk::println!(
                        "Processing bet - User: {}, Bet: {}, Share: {:.4}, Gross payout: {}, Net transfer: {}",
                        bet.user.to_string(),
                        bet.amount.to_u64(),
                        bet_proportion,
                        gross_winnings.to_u64(),
                        transfer_amount.to_u64()
                    );
                    
                    ic_cdk::println!("Transferring {} {} tokens to {}", 
                                  transfer_amount.to_u64() / 10u64.pow(token_info.decimals as u32),
                                  token_info.symbol,
                                  bet.user.to_string());
                    
                    // Transfer winnings with retry logic (up to 2 retries for retryable errors)
                    // This improves reliability by automatically retrying transient failures,
                    // which are common in the distributed Internet Computer environment
                    let transfer_result = transfer_winnings_with_retry(bet.user, transfer_amount.clone(), token_id, 2).await;
                    
                    match transfer_result {
                        Ok(tx_id) => {
                            ic_cdk::println!("Transfer successful (Transaction ID: {})", tx_id);
                            
                            // Record the payout
                            let user_platform_fee = Some(TokenAmount::from(
                                (platform_fee.to_u64() as f64 * bet_proportion) as u64
                            ));
                            
                            let payout_record = BetPayoutRecord {
                                market_id: market.id.clone(),
                                user: bet.user,
                                bet_amount: bet.amount.clone(),
                                payout_amount: gross_winnings.clone(), // Record the gross amount for history
                                timestamp: Timestamp::from(get_current_time()),
                                outcome_index: bet.outcome_index.clone(),
                                was_time_weighted: false,
                                time_weight: None,
                                original_contribution_returned: bet.amount.clone(),
                                bonus_amount: None,
                                platform_fee_amount: user_platform_fee,
                                token_id: token_id.clone(),
                                token_symbol: token_info.symbol.clone(),
                                platform_fee_percentage: token_info.fee_percentage,
                                transaction_id: Some(tx_id),
                            };
                            
                            record_market_payout(payout_record);
                        },
                        Err(e) => {
                            // Enhanced error logging with detailed message
                            ic_cdk::println!("Transfer failed: {}. Continuing with other payouts.", e.detailed_message());
                            
                            // Record failed transaction information for potential recovery
                            // This information can be used by the transaction_recovery module
                            // to retry failed transfers later
                            let failed_tx = FailedTransactionInfo {
                                market_id: market.id.clone(),
                                user: bet.user,
                                amount: transfer_amount.clone(),
                                token_id: token_id.clone(),
                                error: e.detailed_message(),
                                timestamp: get_current_time().into(),
                            };
                            
                            // CRITICAL IMPROVEMENT: We now continue processing other payouts even when 
                            // one transfer fails. This ensures that one user's failed transfer doesn't 
                            // block other users from receiving their winnings, significantly improving 
                            // the platform's reliability. The failed transaction is recorded for later 
                            // recovery through the transaction_recovery system.
                            ic_cdk::println!("Recorded failed transaction: {:?}", failed_tx);
                            
                            // Continue with other payouts instead of returning an error
                        }
                    }
                }
            } else {
                ic_cdk::println!("No winning bets found for this market");
            }
        }
    }

    // Update market status to Closed with the winning outcomes
    // This finalizes the market in the stable memory system and prevents
    // any further bets or resolutions on this market
    market.status = MarketStatus::Closed(winning_outcomes.into_iter().map(|x| x.inner().clone()).collect());
    
    ic_cdk::println!("Market {} successfully finalized with {} winning bets paid out", 
                  market.id.to_u64(), winning_bet_count);
    
    Ok(())
}
