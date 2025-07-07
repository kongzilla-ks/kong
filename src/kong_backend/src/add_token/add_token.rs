use candid::{Nat, Principal};
use ic_cdk::update;

use crate::chains::chains::{IC_CHAIN, SOL_CHAIN};
use crate::ic::guards::{caller_is_proxy, not_in_maintenance_mode};
use crate::stable_token::ic_token::ICToken;
use crate::stable_token::lp_token::LPToken;
use crate::stable_token::solana_token::SolanaToken;
use crate::stable_token::stable_token::StableToken;
use crate::stable_token::token_map;

use super::add_token_args::{AddTokenArgs, AddSplTokenArgs};
use super::add_token_reply::AddTokenReply;
use super::add_token_reply_helpers::to_add_token_reply;

/// Adds a token to Kong
///
/// # Arguments
///
/// * `args` - The arguments for adding a token.
///
/// # Returns
///
/// * `Ok(String)` - A success message if the token is added successfully.
/// * `Err(String)` - An error message if the operation fails.
///
/// # Errors
///
/// This function returns an error if:
/// - The caller is not a controller.
/// - The token already exists.
#[update(guard = "not_in_maintenance_mode")]
async fn add_token(args: AddTokenArgs) -> Result<AddTokenReply, String> {
    // Use get_by_address to check if token exists and get chain info
    match token_map::get_by_address(&args.token) {
        Ok(_existing_token) => {
            // Token already exists
            Err(format!("Token {} already exists", args.token))?
        }
        Err(_) => {
            // Token doesn't exist, determine chain for new token
            // get_by_address handles all formats and defaults to IC for backward compatibility
            let chain = token_map::get_chain(&args.token)
                .unwrap_or_else(|| IC_CHAIN.to_string());

            // Route based on chain type
            match chain.as_str() {
                IC_CHAIN => {
                    // IC tokens can be added by anyone (controllers)
                    to_add_token_reply(&add_ic_token(&args.token).await?)
                }
                SOL_CHAIN => {
                    // Solana tokens are added automatically via ATA discovery
                    Err("Solana tokens are added automatically. Use add_spl_token endpoint for proxy calls.".to_string())?
                }
                _ => Err("Chain not supported")?,
            }
        }
    }
}

/// Adds an Internet Computer (IC) token to the system.
///
/// # Arguments
///
/// * `token` - The address of the token to be added. Must be in the format IC.CanisterId.
///
/// # Returns
///
/// * `Ok(StableToken)` - The newly added token.
/// * `Err(String)` - An error message if the operation fails.
///
/// # Errors
///
/// This function returns an error if:
/// - The address of the token is not found.
/// - The address cannot be converted to a `Principal`.
/// - Creating the `ICToken` fails.
/// - Inserting the token into the token map fails.
/// - Retrieving the inserted token fails.
pub async fn add_ic_token(token: &str) -> Result<StableToken, String> {
    // Retrieves the address of the token
    let address = token_map::get_address(token).ok_or_else(|| format!("Invalid address {}", token))?;

    // Valid and converts the address to a Principal ID
    let canister_id = Principal::from_text(address).map_err(|e| format!("Invalid canister id {}: {}", token, e))?;

    let ic_token = StableToken::IC(ICToken::new(&canister_id).await?);
    let token_id = token_map::insert(&ic_token)?;

    // Retrieves the inserted token by its token_id
    token_map::get_by_token_id(token_id).ok_or_else(|| format!("Failed to add token {}", token))
}

/// Adds an SPL token with metadata (proxy-only endpoint).
///
/// This endpoint is used by the kong_rpc proxy during ATA discovery:
/// 1. Proxy detects new token accounts on Solana  
/// 2. Proxy fetches token metadata from Solana RPC
/// 3. Proxy calls this function to add the token to Kong
///
/// # Arguments
///
/// * `args` - SPL token address and metadata fetched from Solana
///
/// # Returns
///
/// * `Ok(AddTokenReply)` - The newly added token information
/// * `Err(String)` - Error if token creation fails
///
/// # Security
///
/// Only callable by the kong_rpc proxy via caller_is_proxy guard.
#[update(guard = "not_in_maintenance_mode")]
async fn add_spl_token(args: AddSplTokenArgs) -> Result<AddTokenReply, String> {
    // Only proxy can call this endpoint
    caller_is_proxy()?;
    
    // Use get_by_address to check if token exists and get chain info
    match token_map::get_by_address(&args.token) {
        Ok(_existing_token) => {
            // Token already exists
            Err(format!("Token {} already exists", args.token))?
        }
        Err(_) => {
            // Token doesn't exist, determine chain for new token
            let chain = token_map::get_chain(&args.token)
                .unwrap_or_else(|| IC_CHAIN.to_string());

            // Ensure it's a Solana token
            match chain.as_str() {
                SOL_CHAIN => {
                    to_add_token_reply(&add_solana_token_internal(&args).await?)
                }
                _ => Err("This endpoint is only for Solana tokens".to_string())?
            }
        }
    }
}

/// Internal function to create a Solana token from SPL token args
async fn add_solana_token_internal(args: &AddSplTokenArgs) -> Result<StableToken, String> {
    // Extract mint address from token string (format: SOL.MintAddress)
    let mint_address = token_map::get_address(&args.token).ok_or_else(|| format!("Invalid address {}", args.token))?;

    // Use provided fee or default to 5000 (0.005 SOL)
    let fee = args.fee.clone().unwrap_or_else(|| Nat::from(5000u64));

    let solana_token = StableToken::Solana(SolanaToken {
        token_id: 0, // Will be set by insert
        name: args.name.clone(),
        symbol: args.symbol.clone(),
        decimals: args.decimals,
        fee,
        mint_address: mint_address.to_string(),
        program_id: args.program_id.clone(),
        total_supply: None,                                               // We don't track total supply for now
        is_spl_token: mint_address != "11111111111111111111111111111111", // False for native SOL
    });

    let token_id = token_map::insert(&solana_token)?;

    // Retrieves the inserted token by its token_id
    token_map::get_by_token_id(token_id).ok_or_else(|| format!("Failed to add Solana token {}", args.token))
}

pub fn add_lp_token(token_0: &StableToken, token_1: &StableToken) -> Result<StableToken, String> {
    let lp_token = StableToken::LP(LPToken::new(token_0, token_1));
    let token_id = token_map::insert(&lp_token)?;

    // Retrieves the inserted token by its token_id
    token_map::get_by_token_id(token_id).ok_or_else(|| "Failed to add LP token".to_string())
}

