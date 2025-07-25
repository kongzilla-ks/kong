use candid::{decode_one, CandidType, Nat};
use ic_cdk::{init, post_upgrade, pre_upgrade, query, update};
use ic_cdk_macros::inspect_message;
use ic_cdk_timers::set_timer_interval;
use icrc_ledger_types::icrc21::errors::ErrorInfo;
use icrc_ledger_types::icrc21::requests::{ConsentMessageMetadata, ConsentMessageRequest};
use icrc_ledger_types::icrc21::responses::{ConsentInfo, ConsentMessage};
use serde::Deserialize;
use std::time::Duration;

use crate::add_liquidity::add_liquidity_args::AddLiquidityArgs;
use crate::add_liquidity::add_liquidity_reply::AddLiquidityReply;
use crate::add_liquidity_amounts::add_liquidity_amounts_reply::AddLiquidityAmountsReply;
use crate::add_pool::add_pool_args::AddPoolArgs;
use crate::add_pool::add_pool_reply::AddPoolReply;
use crate::add_token::add_token_args::AddTokenArgs;
use crate::add_token::add_token_reply::AddTokenReply;
use crate::add_token::update_token_args::UpdateTokenArgs;
use crate::add_token::update_token_reply::UpdateTokenReply;
use crate::chains::chains::SOL_CHAIN;
use crate::claims::claims_timer::process_claims_timer;
use crate::helpers::nat_helpers::{nat_is_zero, nat_to_decimals_f64, nat_to_f64};
use crate::ic::network::ICNetwork;
use crate::remove_liquidity::remove_liquidity_args::RemoveLiquidityArgs;
use crate::solana::utils::validation;
use crate::stable_kong_settings::kong_settings_map;
use crate::stable_memory::cleanup_old_notifications;
use crate::stable_request::request_archive::archive_request_map;
use crate::stable_token::token::Token;
use crate::stable_token::token_management::check_disabled_tokens;
use crate::stable_token::token_map;
use crate::stable_transfer::transfer_archive::archive_transfer_map;
use crate::stable_tx::tx_archive::archive_tx_map;
use crate::stable_user::principal_id_map::create_principal_id_map;
use crate::swap::swap_args::SwapArgs;

use super::kong_backend::KongBackend;
use super::stable_memory::{get_cached_solana_address, get_solana_transaction};
use super::stable_transfer::tx_id::TxId;
use super::{APP_NAME, APP_VERSION};

// list of query calls
static QUERY_METHODS: [&str; 12] = [
    "icrc1_name",
    "icrc10_supported_standards",
    "tokens",
    "pools",
    "get_user",
    "user_balances",
    "requests",
    "add_liquidity_amounts",
    "remove_liquidity_amounts",
    "swap_amounts",
    "claims",
    "get_solana_address",
];

#[init]
async fn init() {
    ICNetwork::info_log(&format!("{} canister is being initialized", APP_NAME));

    create_principal_id_map();

    set_timer_processes().await;
}

#[pre_upgrade]
fn pre_upgrade() {
    ICNetwork::info_log(&format!("{} canister is being upgraded", APP_NAME));
}

#[post_upgrade]
async fn post_upgrade() {
    ICNetwork::info_log(&format!("{} canister has been upgraded", APP_NAME));

    create_principal_id_map();

    // Check if Solana address is cached
    // NOTE: We cannot make inter-canister calls in post_upgrade, even with spawn
    // The verification must be done by calling cache_solana_address() after upgrade
    let cached_solana_address = get_cached_solana_address();
    if !cached_solana_address.is_empty() {
        ICNetwork::info_log(&format!("Solana address: {}", cached_solana_address));
    } else {
        ICNetwork::error_log("No cached Solana address found");
        ICNetwork::error_log("REQUIRED: Call cache_solana_address() to initialize it");
    }

    set_timer_processes().await;
}

async fn set_timer_processes() {
    // start the background timer to process claims
    let _ = set_timer_interval(Duration::from_secs(kong_settings_map::get().claims_interval_secs), || {
        ic_cdk::futures::spawn(async {
            process_claims_timer().await;
        });
    });

    // start the background timer to archive request map
    let _ = set_timer_interval(Duration::from_secs(kong_settings_map::get().requests_archive_interval_secs), || {
        ic_cdk::futures::spawn(async {
            archive_request_map();
        });
    });

    // start the background timer to archive transfer map
    let _ = set_timer_interval(
        Duration::from_secs(kong_settings_map::get().transfers_archive_interval_secs),
        || {
            ic_cdk::futures::spawn(async {
                archive_transfer_map().await;
            });
        },
    );

    // start the background timer to archive tx map
    let _ = set_timer_interval(Duration::from_secs(kong_settings_map::get().txs_archive_interval_secs), || {
        ic_cdk::futures::spawn(async {
            archive_tx_map();
        });
    });

    // start the background timer to check for disabled tokens
    let _ = set_timer_interval(
        Duration::from_secs(kong_settings_map::get().check_disabled_token_interval_secs),
        || {
            ic_cdk::futures::spawn(async {
                check_disabled_tokens().await;
            });
        },
    );

    // start the background timer to cleanup old Solana notifications
    let _ = set_timer_interval(Duration::from_secs(3600), || {
        // Clean up every hour
        ic_cdk::futures::spawn(async {
            cleanup_old_notifications();
        });
    });
}

/// inspect all ingress messages to the canister that are called as updates
/// calling accept_message() will allow the message to be processed
#[inspect_message]
fn inspect_message() {
    let method_name = ic_cdk::api::msg_method_name();
    if QUERY_METHODS.contains(&method_name.as_str()) {
        ICNetwork::info_log(&format!("{} called as update from {}", method_name, ICNetwork::caller().to_text()));
        ic_cdk::trap(format!("{} must be called as query", method_name));
    }

    // Add anti-spam filtering for swap operations
    if method_name == "swap" || method_name == "swap_async" {
        if let Err(e) = validate_swap_request() {
            ic_cdk::trap(&e);
        }
    }

    // Add validation for remove liquidity operations
    if method_name == "remove_liquidity" || method_name == "remove_liquidity_async" {
        if let Err(e) = validate_remove_liquidity_request() {
            ic_cdk::trap(&e);
        }
    }

    ic_cdk::api::accept_message();
}

/// Basic validation for swap requests to prevent spam before heavy processing
fn validate_swap_request() -> Result<(), String> {
    // Get the raw argument bytes for basic validation
    let args_bytes = ic_cdk::api::msg_arg_data();

    // Basic size check - prevent extremely large payloads
    if args_bytes.len() > 10_000 {
        return Err("Request payload too large".to_string());
    }

    // Try to decode swap args for basic validation
    match decode_one::<SwapArgs>(&args_bytes) {
        Ok(args) => {
            // Basic parameter validation
            if args.pay_token.is_empty() || args.receive_token.is_empty() {
                return Err("Invalid token parameters".to_string());
            }

            // Amount validation - prevent zero or extremely large amounts
            if nat_is_zero(&args.pay_amount) {
                return Err("Pay amount cannot be zero".to_string());
            }

            // Check if Solana transaction is ready (zero-cost early rejection)
            if let Some(TxId::TransactionId(signature)) = args.pay_tx_id {
                // This is a Solana transaction - check if it exists in canister memory
                if get_solana_transaction(signature).is_none() {
                    return Err("TRANSACTION_NOT_READY".to_string());
                }
            }

            Ok(())
        }
        Err(_) => Err("Invalid swap arguments format".to_string()),
    }
}

/// Validation for remove liquidity requests to prevent invalid cross-chain operations
fn validate_remove_liquidity_request() -> Result<(), String> {
    let args_bytes = ic_cdk::api::msg_arg_data();

    // Basic size check
    if args_bytes.len() > 10_000 {
        return Err("Request payload too large".to_string());
    }

    // Try to decode remove liquidity args
    match decode_one::<RemoveLiquidityArgs>(&args_bytes) {
        Ok(args) => {
            // Basic parameter validation
            if args.token_0.is_empty() || args.token_1.is_empty() {
                return Err("Invalid token parameters".to_string());
            }

            // Amount validation
            if nat_is_zero(&args.remove_lp_token_amount) {
                return Err("Remove amount cannot be zero".to_string());
            }

            // Parse tokens to check if they're Solana tokens
            let (chain_0, _) = args.token_0.split_once('.').unwrap_or(("", ""));
            let (chain_1, _) = args.token_1.split_once('.').unwrap_or(("", ""));

            // If token_0 is Solana and no payout_address_0, reject early
            if chain_0 == SOL_CHAIN && args.payout_address_0.is_none() {
                return Err("Solana token payouts require payout_address_0".to_string());
            }

            // If token_1 is Solana and no payout_address_1, reject early
            if chain_1 == SOL_CHAIN && args.payout_address_1.is_none() {
                return Err("Solana token payouts require payout_address_1".to_string());
            }

            // Validate Solana addresses if provided
            if let Some(ref addr) = args.payout_address_0 {
                if chain_0 == SOL_CHAIN {
                    validation::validate_address(addr).map_err(|_| format!("Invalid Solana address for payout_address_0: {}", addr))?;
                }
            }

            if let Some(ref addr) = args.payout_address_1 {
                if chain_1 == SOL_CHAIN {
                    validation::validate_address(addr).map_err(|_| format!("Invalid Solana address for payout_address_1: {}", addr))?;
                }
            }

            Ok(())
        }
        Err(_) => Err("Invalid remove liquidity arguments format".to_string()),
    }
}

#[query]
fn icrc1_name() -> String {
    format!("{} {}", APP_NAME, APP_VERSION)
}

#[derive(CandidType, Deserialize, Eq, PartialEq, Debug)]
pub struct SupportedStandard {
    pub url: String,
    pub name: String,
}

#[query]
fn icrc10_supported_standards() -> Vec<SupportedStandard> {
    vec![
        SupportedStandard {
            url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-10/ICRC-10.md".to_string(),
            name: "ICRC-10".to_string(),
        },
        SupportedStandard {
            url: "https://github.com/dfinity/wg-identity-authentication/blob/main/topics/ICRC-21/icrc_21_consent_msg.md".to_string(),
            name: "ICRC-21".to_string(),
        },
        SupportedStandard {
            url: "https://github.com/dfinity/wg-identity-authentication/blob/main/topics/icrc_28_trusted_origins.md".to_string(),
            name: "ICRC-28".to_string(),
        },
    ]
}

#[update]
fn icrc21_canister_call_consent_message(consent_msg_request: ConsentMessageRequest) -> Result<ConsentInfo, ErrorInfo> {
    let consent_message = match consent_msg_request.method.as_str() {
        "swap" | "swap_async" => {
            let Ok(swap_args) = decode_one::<SwapArgs>(&consent_msg_request.arg) else {
                Err(ErrorInfo {
                    description: "Failed to decode SwapArgs".to_string(),
                })?
            };
            let Ok(token) = token_map::get_by_token(swap_args.pay_token.as_str()) else {
                Err(ErrorInfo {
                    description: "Failed to get token".to_string(),
                })?
            };
            let decimals = token.decimals();
            let pay_amount = nat_to_decimals_f64(decimals, &swap_args.pay_amount).ok_or_else(|| ErrorInfo {
                description: "Failed to convert pay amount to f64".to_string(),
            })?;
            let to_address = match swap_args.receive_address {
                Some(address) => address,
                None => ICNetwork::caller().to_text(),
            };
            let receive_token = match swap_args.receive_amount {
                Some(amount) => {
                    let receive_amount = nat_to_f64(&amount).ok_or_else(|| ErrorInfo {
                        description: "Failed to convert receive amount to f64".to_string(),
                    })?;
                    format!("Min. amount {} {}", receive_amount, swap_args.receive_token)
                }
                None => {
                    let max_slippage = swap_args.max_slippage.unwrap_or(kong_settings_map::get().default_max_slippage);
                    format!("{} (max. slippage {}%)", swap_args.receive_token, max_slippage)
                }
            };

            ConsentMessage::GenericDisplayMessage(format!(
                "# Approve KongSwap swap
                
**Pay token:**
{} {}

**Receive token:**
{}

**Receive address:**
{}",
                pay_amount, swap_args.pay_token, receive_token, to_address
            ))
        }
        "add_liquidity" | "add_liquidity_async" => {
            let Ok(add_liquidity_args) = decode_one::<AddLiquidityArgs>(&consent_msg_request.arg) else {
                Err(ErrorInfo {
                    description: "Failed to decode AddLiquidityArgs".to_string(),
                })?
            };
            let Ok(token_0) = token_map::get_by_token(add_liquidity_args.token_0.as_str()) else {
                Err(ErrorInfo {
                    description: "Failed to get token_0".to_string(),
                })?
            };
            let decimals_0 = token_0.decimals();
            let amount_0 = nat_to_decimals_f64(decimals_0, &add_liquidity_args.amount_0).ok_or_else(|| ErrorInfo {
                description: "Failed to convert token_0 amount to f64".to_string(),
            })?;
            let Ok(token_1) = token_map::get_by_token(add_liquidity_args.token_1.as_str()) else {
                Err(ErrorInfo {
                    description: "Failed to get token_1".to_string(),
                })?
            };
            let decimals_1 = token_1.decimals();
            let amount_1 = nat_to_decimals_f64(decimals_1, &add_liquidity_args.amount_1).ok_or_else(|| ErrorInfo {
                description: "Failed to convert token_1 amount to f64".to_string(),
            })?;
            ConsentMessage::GenericDisplayMessage(format!(
                "# Approve KongSwap add liquidity

**Token 0:**
{} {}

**Token 1:**
{} {}",
                amount_0, add_liquidity_args.token_0, amount_1, add_liquidity_args.token_1
            ))
        }
        "add_pool" => {
            let Ok(add_pool_args) = decode_one::<AddPoolArgs>(&consent_msg_request.arg) else {
                Err(ErrorInfo {
                    description: "Failed to decode AddPoolArgs".to_string(),
                })?
            };
            let Ok(token_0) = token_map::get_by_token(add_pool_args.token_0.as_str()) else {
                Err(ErrorInfo {
                    description: "Failed to get token_0".to_string(),
                })?
            };
            let decimals_0 = token_0.decimals();
            let amount_0 = nat_to_decimals_f64(decimals_0, &add_pool_args.amount_0).ok_or_else(|| ErrorInfo {
                description: "Failed to convert token_0 amount to f64".to_string(),
            })?;
            let Ok(token_1) = token_map::get_by_token(add_pool_args.token_1.as_str()) else {
                Err(ErrorInfo {
                    description: "Failed to get token_1".to_string(),
                })?
            };
            let decimals_1 = token_1.decimals();
            let amount_1 = nat_to_decimals_f64(decimals_1, &add_pool_args.amount_1).ok_or_else(|| ErrorInfo {
                description: "Failed to convert token_1 amount to f64".to_string(),
            })?;
            ConsentMessage::GenericDisplayMessage(format!(
                "# Approve KongSwap add pool

**Token 0:**
{} {}

**Token 1:**
{} {}",
                amount_0, add_pool_args.token_0, amount_1, add_pool_args.token_1
            ))
        }
        _ => ConsentMessage::GenericDisplayMessage(format!("Approve KongSwap to execute {}", consent_msg_request.method)),
    };

    let metadata = ConsentMessageMetadata {
        language: "en".to_string(),
        utc_offset_minutes: None,
    };

    Ok(ConsentInfo { metadata, consent_message })
}

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct Icrc28TrustedOriginsResponse {
    pub trusted_origins: Vec<String>,
}

// list every base URL that users will authenticate to your app from
#[update]
fn icrc28_trusted_origins() -> Icrc28TrustedOriginsResponse {
    let canister = KongBackend::canister().to_text();
    let trusted_origins = vec![
        format!("https://{}.icp0.io", canister),
        #[cfg(not(feature = "prod"))]
        format!("http://{}.localhost:4943", canister),
        #[cfg(not(feature = "prod"))]
        format!("https://edoy4-liaaa-aaaar-qakha-cai.localhost:5173"), // svelte FE
        #[cfg(not(feature = "prod"))]
        format!("http://localhost:5173"),
        #[cfg(feature = "prod")]
        String::from("https://kongswap.io"),
        #[cfg(feature = "prod")]
        String::from("https://www.kongswap.io"),
        #[cfg(feature = "prod")]
        String::from("https://edoy4-liaaa-aaaar-qakha-cai.icp0.io"),
        #[cfg(feature = "prod")]
        String::from("https://dev.kongswap.io"),
    ];

    Icrc28TrustedOriginsResponse { trusted_origins }
}

ic_cdk::export_candid!();
