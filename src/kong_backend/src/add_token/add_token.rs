use candid::{Nat, Principal};
use ic_cdk::update;

use crate::chains::chains::{IC_CHAIN, SOL_CHAIN};
use crate::ic::guards::{caller_is_proxy, not_in_maintenance_mode};
use crate::stable_token::ic_token::ICToken;
use crate::stable_token::lp_token::LPToken;
use crate::stable_token::solana_token::SolanaToken;
use crate::stable_token::stable_token::StableToken;
use crate::stable_token::token_map;

use super::add_token_args::AddTokenArgs;
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
    if token_map::get_by_address(&args.token).is_ok() {
        Err(format!("Token {} already exists", args.token))?
    }

    // Route based on chain type
    match token_map::get_chain(&args.token) {
        Some(chain) if chain == IC_CHAIN => {
            // IC tokens can be added by anyone (controllers)
            to_add_token_reply(&add_ic_token(&args.token).await?)
        }
        Some(chain) if chain == SOL_CHAIN => {
            // Solana tokens can only be added by the proxy
            caller_is_proxy()?;
            to_add_token_reply(&add_solana_token_from_proxy(&args).await?)
        }
        Some(_) | None => Err("Chain not supported")?,
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

/// Adds a Solana token to the system (deprecated - use add_solana_token_from_proxy)
///
/// This function is kept for backwards compatibility but should not be used.
/// All Solana tokens should be added through the proxy.
pub async fn add_solana_token(_args: &AddTokenArgs) -> Result<StableToken, String> {
    Err("Direct Solana token addition is deprecated. Solana tokens should be added through the proxy.".to_string())
}

/// Adds a Solana token from the proxy with dynamic metadata.
///
/// # Arguments
///
/// * `args` - The arguments containing Solana token information from the proxy.
///
/// # Returns
///
/// * `Ok(StableToken)` - The newly added token.
/// * `Err(String)` - An error message if the operation fails.
///
/// # Security
///
/// This function should only be called by the proxy (verified by the caller_is_proxy guard).
pub async fn add_solana_token_from_proxy(args: &AddTokenArgs) -> Result<StableToken, String> {
    // Extract mint address from token string (format: SOL.MintAddress)
    let mint_address = token_map::get_address(&args.token).ok_or_else(|| format!("Invalid address {}", args.token))?;

    // Validate required fields from proxy
    let name = args.name.clone().ok_or("Missing token name")?;
    let symbol = args.symbol.clone().ok_or("Missing token symbol")?;
    let decimals = args.decimals.ok_or("Missing token decimals")?;
    let program_id = args.solana_program_id.clone().ok_or("Missing program ID")?;
    
    // Use provided fee or default to 5000 (0.005 SOL)
    let fee = args.fee.clone().unwrap_or_else(|| Nat::from(5000u64));

    let solana_token = StableToken::Solana(SolanaToken {
        token_id: 0, // Will be set by insert
        name,
        symbol,
        decimals,
        fee,
        mint_address: mint_address.to_string(),
        program_id,
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
