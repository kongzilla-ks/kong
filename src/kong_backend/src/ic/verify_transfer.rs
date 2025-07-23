//! Transfer verification utilities
//! 
//! This module provides comprehensive functionality for verifying transfers and handling amount mismatches
//! across different operations (swap, add_liquidity, add_pool). It ensures consistent behavior
//! when the actual transfer amount on the blockchain differs from the expected amount.
//! 
//! # Amount Mismatch Handling
//! 
//! When a user initiates a transfer, they specify an amount. However, the actual amount recorded
//! on the blockchain may differ due to:
//! - Transfer fees not accounted for by the user
//! - Token contract behavior
//! - Rounding differences
//! 
//! This module handles these mismatches by:
//! 1. Recording the transfer with the actual amount to prevent reuse
//! 2. Returning a clear error message with both expected and actual amounts
//! 3. Enabling the calling code to return tokens to the user

use candid::Principal;
use candid::{CandidType, Deserialize, Nat};
use ic_ledger_types::{query_blocks, AccountIdentifier, Block, GetBlocksArgs, Operation, Subaccount};
use icrc_ledger_types::icrc::generic_value::ICRC3Value;
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc3::blocks::{GetBlocksRequest as ICRC3GetBlocksRequest, GetBlocksResult as ICRC3GetBlocksResult};
use icrc_ledger_types::icrc3::transactions::{GetTransactionsRequest, GetTransactionsResponse};
use num_traits::cast::ToPrimitive;
use serde::Serialize;
use std::collections::BTreeMap;

use crate::helpers::nat_helpers::nat_to_u64;
use crate::ic::network::ICNetwork;
use crate::stable_kong_settings::kong_settings_map;
use crate::stable_token::stable_token::StableToken;
use crate::stable_token::token::Token;
use crate::stable_transfer::{stable_transfer::StableTransfer, transfer_map, tx_id::TxId};
use crate::stable_request::{request_map, status::StatusCode};

use super::wumbo::Transaction1;

const SUBACCOUNT_LENGTH: usize = 32;

const ICP_CANISTER_ID: &str = "IC.ryjl3-tyaaa-aaaaa-aaaba-cai"; // Mainnet ICP Ledger
const WUMBO_CANISTER_ID: &str = "IC.wkv3f-iiaaa-aaaap-ag73a-cai";
const DAMONIC_CANISTER_ID: &str = "IC.zzsnb-aaaaa-aaaap-ag66q-cai";
const CLOWN_CANISTER_ID: &str = "IC.iwv6l-6iaaa-aaaal-ajjjq-cai";
const TAGGR_CANISTER_ID: &str = "IC.6qfxa-ryaaa-aaaai-qbhsq-cai";

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TaggrGetBlocksArgs {
    pub start: u64,
    pub length: u64,
}


#[derive(Debug, Clone)]
pub enum TransferError {
    DuplicateTransfer { tx_id: Nat },
    TransferNotFound { error: String },
    AmountMismatch { 
        expected: Nat, 
        actual: Nat,
        transfer_id: u64,
    },
}

impl TransferError {
    pub fn to_string(&self) -> String {
        match self {
            Self::DuplicateTransfer { tx_id } => 
                format!("Duplicate block id #{}", tx_id),
            Self::TransferNotFound { error } => error.clone(),
            Self::AmountMismatch { expected, actual, .. } => 
                format!("Transfer amount mismatch: expected {} but got {}", expected, actual),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TokenType {
    PayToken,
    Token0,
    Token1,
}

impl TokenType {
    fn verify_status(&self) -> StatusCode {
        match self {
            TokenType::PayToken => StatusCode::VerifyPayToken,
            TokenType::Token0 => StatusCode::VerifyToken0,
            TokenType::Token1 => StatusCode::VerifyToken1,
        }
    }
    
    fn verify_failed_status(&self) -> StatusCode {
        match self {
            TokenType::PayToken => StatusCode::VerifyPayTokenFailed,
            TokenType::Token0 => StatusCode::VerifyToken0Failed,
            TokenType::Token1 => StatusCode::VerifyToken1Failed,
        }
    }
    
    fn verify_success_status(&self) -> StatusCode {
        match self {
            TokenType::PayToken => StatusCode::VerifyPayTokenSuccess,
            TokenType::Token0 => StatusCode::VerifyToken0Success,
            TokenType::Token1 => StatusCode::VerifyToken1Success,
        }
    }
}

/// Verifies a transfer and records it in the transfer map
/// 
/// This function:
/// 1. Verifies the transfer exists on the blockchain
/// 2. Checks for duplicate transfers
/// 3. Compares the actual amount with the expected amount
/// 4. Records the transfer to prevent reuse
/// 
/// # Arguments
/// 
/// * `request_id` - The unique identifier for this request
/// * `token_type` - The type of token being verified (PayToken, Token0, or Token1)
/// * `token` - The token being transferred
/// * `tx_id` - The transaction/block ID on the blockchain
/// * `expected_amount` - The amount the user specified
/// * `ts` - The timestamp of the operation
/// 
/// # Returns
/// 
/// * `Ok(transfer_id)` - The ID of the recorded transfer
/// * `Err(TransferError)` - Error if verification fails or amount mismatches
/// 
/// # Amount Mismatch Behavior
/// 
/// When the actual amount differs from the expected amount:
/// 1. The transfer is still recorded with the actual amount to prevent reuse
/// 2. An error is returned with both amounts for clarity
/// 3. The calling code can then initiate a token return
pub async fn verify_and_record_transfer(
    request_id: u64,
    token_type: TokenType,
    token: &StableToken,
    tx_id: &Nat,
    expected_amount: &Nat,
    ts: u64,
) -> Result<u64, TransferError> {
    let token_id = token.token_id();
    
    request_map::update_status(request_id, token_type.verify_status(), None);
    
    // Get the actual amount from the ledger first
    let actual_amount = match get_transfer_amount(token, tx_id).await {
        Ok(amount) => amount,
        Err(e) => {
            request_map::update_status(request_id, token_type.verify_failed_status(), Some(&e));
            return Err(TransferError::TransferNotFound { error: e });
        }
    };
    
    // Check for duplicates and insert atomically
    if transfer_map::contain(token_id, tx_id) {
        let error = TransferError::DuplicateTransfer { tx_id: tx_id.clone() };
        request_map::update_status(request_id, token_type.verify_failed_status(), Some(&error.to_string()));
        return Err(error);
    }
    
    // Check if amounts match
    if actual_amount != *expected_amount {
        // IMPORTANT: Record the transfer with the actual amount to prevent reuse
        let transfer_id = transfer_map::insert(&StableTransfer {
            transfer_id: 0,
            request_id,
            is_send: true,
            amount: actual_amount.clone(),
            token_id,
            tx_id: TxId::BlockIndex(tx_id.clone()),
            ts,
        });
        
        let error = TransferError::AmountMismatch {
            expected: expected_amount.clone(),
            actual: actual_amount,
            transfer_id,
        };
        request_map::update_status(request_id, token_type.verify_failed_status(), Some(&error.to_string()));
        return Err(error);
    }
    
    // Amounts match - record the transfer and return success
    let transfer_id = transfer_map::insert(&StableTransfer {
        transfer_id: 0,
        request_id,
        is_send: true,
        amount: expected_amount.clone(),
        token_id,
        tx_id: TxId::BlockIndex(tx_id.clone()),
        ts,
    });
    
    request_map::update_status(request_id, token_type.verify_success_status(), None);
    Ok(transfer_id)
}

/// Verifies a transfer by checking the ledger and validating the amount matches.
/// For ICRC3 tokens, it tries ICRC3 methods first, falling back to traditional methods.
/// For non-ICRC3 tokens, it uses the traditional verification methods.
/// Returns Ok(()) if verification succeeds, otherwise returns an error.
pub async fn verify_transfer(token: &StableToken, block_id: &Nat, amount: &Nat) -> Result<(), String> {
    let actual_amount = get_transfer_amount(token, block_id).await?;
    
    if actual_amount != *amount {
        return Err(format!("Transfer amount mismatch: expected {} but got {}", amount, actual_amount));
    }
    
    Ok(())
}

/// Retrieves the actual transfer amount from the ledger without validation.
/// For ICRC3 tokens, it tries ICRC3 methods first, falling back to traditional methods.
/// For non-ICRC3 tokens, it uses the traditional verification methods.
/// Returns the actual transfer amount found on the ledger.
pub async fn get_transfer_amount(token: &StableToken, block_id: &Nat) -> Result<Nat, String> {
    match token {
        StableToken::IC(ic_token) => {
            let canister_id = *token.canister_id().ok_or("Invalid canister id")?;
            let token_address_with_chain = token.address_with_chain();
            let kong_settings = kong_settings_map::get();
            let min_valid_timestamp = ICNetwork::get_time() - kong_settings.transfer_expiry_nanosecs;
            let kong_backend_account = &kong_settings.kong_backend;
            let caller_account = ICNetwork::caller_id();

            // try icrc3_get_blocks first
            if ic_token.icrc3 {
                return get_transfer_amount_with_icrc3_get_blocks(
                    token,
                    block_id,
                    canister_id,
                    &token_address_with_chain,
                    min_valid_timestamp,
                    kong_backend_account,
                    caller_account,
                )
                .await;
            }

            // if ICP ledger, use query_blocks
            if token_address_with_chain == ICP_CANISTER_ID {
                return get_transfer_amount_with_query_blocks(token, block_id, canister_id, min_valid_timestamp, kong_backend_account)
                    .await;
            }

            // otherwise, use get_transactions
            get_transfer_amount_with_get_transactions(
                token,
                block_id,
                canister_id,
                min_valid_timestamp,
                kong_backend_account,
                caller_account,
            )
            .await
        }
        _ => Err("Get transfer amount not supported for this token")?,
    }
}

fn try_decode_icrc3_account_value(icrc3_value_arr: &[ICRC3Value]) -> Option<Account> {
    if let Some(ICRC3Value::Blob(blob)) = icrc3_value_arr.first() {
        if let Ok(owner) = Principal::try_from_slice(blob) {
            let subaccount = if icrc3_value_arr.len() >= 2 {
                icrc3_value_arr.get(1).and_then(|val| match val {
                    ICRC3Value::Blob(blob2) if blob2.len() == SUBACCOUNT_LENGTH => blob2.as_slice().try_into().ok(),
                    _ => None,
                })
            } else {
                None
            };
            return Some(Account { owner, subaccount });
        }
    }

    if icrc3_value_arr.len() == 1 {
        if let Some(ICRC3Value::Blob(blob)) = icrc3_value_arr.first() {
            if let Ok(map) = ciborium::de::from_reader::<BTreeMap<String, ciborium::value::Value>, _>(blob.as_slice()) {
                let owner_bytes = match map.get("owner") {
                    Some(ciborium::value::Value::Bytes(bytes)) => bytes.as_slice(),
                    _ => return None,
                };
                let owner = Principal::try_from_slice(owner_bytes).ok()?;
                let subaccount = map.get("subaccount").and_then(|val| match val {
                    ciborium::value::Value::Bytes(bytes) if !bytes.is_empty() && bytes.len() == SUBACCOUNT_LENGTH => {
                        let mut sa = [0u8; SUBACCOUNT_LENGTH];
                        sa.copy_from_slice(bytes);
                        Some(sa)
                    }
                    _ => None,
                });
                return Some(Account { owner, subaccount });
            }
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
async fn get_transfer_amount_with_icrc3_get_blocks(
    token: &StableToken,
    block_id: &Nat,
    canister_id: candid::Principal,
    token_address_with_chain: &String,
    min_valid_timestamp: u64,
    kong_backend_account: &icrc_ledger_types::icrc1::account::Account,
    caller_account: icrc_ledger_types::icrc1::account::Account,
) -> Result<Nat, String> {
    // Prepare request arguments based on token type
    let blocks_result = if *token_address_with_chain == TAGGR_CANISTER_ID {
        // TAGGR uses a different format
        // API boundary: TAGGR's icrc3_get_blocks requires u64
        let start_u64 = nat_to_u64(block_id).ok_or("Invalid block_id format for TAGGR")?;
        let block_args = vec![TaggrGetBlocksArgs {
            start: start_u64,
            length: 1u64,
        }];
        ic_cdk::call::Call::unbounded_wait(canister_id, "icrc3_get_blocks")
            .with_arg((block_args,))
            .await
            .map_err(|e| format!("{:?}", e))?
            .candid::<(ICRC3GetBlocksResult,)>()
            .map(|(result,)| result)
            .map_err(|e| format!("{:?}", e))
    } else {
        // Standard ICRC3 format
        let single_request_arg = ICRC3GetBlocksRequest {
            start: block_id.clone(),
            length: Nat::from(1u32),
        };
        // ICRC-3 icrc3_get_blocks expects `vec record { start: nat; length: nat; }`
        // So we wrap the single request argument in a vector.
        let block_args_vec = vec![single_request_arg];
        ic_cdk::call::Call::unbounded_wait(canister_id, "icrc3_get_blocks")
            .with_arg(block_args_vec)
            .await
            .map_err(|e| format!("{:?}", e))?
            .candid::<ICRC3GetBlocksResult>()
            .map_err(|e| format!("{:?}", e))
    };
    match blocks_result {
        Ok(response) => {
            let blocks_data = response;

            for block_envelope in blocks_data.blocks.iter() {
                // Skip blocks that don't match our ID
                if block_envelope.id != *block_id {
                    continue;
                }

                // Only process map-type blocks
                let fields = match &block_envelope.block {
                    ICRC3Value::Map(fields) => fields,
                    _ => continue,
                };

                let tx_map = fields.get("tx");

                match fields.get("ts").and_then(|v| match v {
                    ICRC3Value::Nat(ts_nat) => ts_nat.0.to_u64(),
                    _ => None,
                }) {
                    Some(ts) if ts < min_valid_timestamp => Err("Expired transfer timestamp")?,
                    None => Err("Missing transfer timestamp".to_string())?,
                    Some(_) => (),
                };

                // Extract fields from the transaction
                let mut op = None;
                // Check for block type
                if let Some(ICRC3Value::Text(btype)) = fields.get("btype") {
                    op = Some(btype.clone());
                }
                if let Some(ICRC3Value::Map(tx)) = tx_map {
                    // tx.op overrides btype
                    if let Some(ICRC3Value::Text(tx_op)) = tx.get("op") {
                        op = Some(tx_op.clone());
                    }
                }
                match op {
                    Some(op_str) if matches!(op_str.as_str(), "icrc1_transfer" | "1xfer" | "transfer" | "xfer") => (),
                    Some(op_str) => Err(format!("Invalid transfer operation: {}", op_str))?,
                    None => Err("Missing operation")?,
                }

                // Extract transaction details from tx map or top level
                let mut tx_amount = None;
                if let Some(ICRC3Value::Map(tx)) = tx_map {
                    // Amount
                    if let Some(ICRC3Value::Nat(amt)) = tx.get("amt") {
                        tx_amount = Some(amt.clone());
                    }
                } else {
                    // Top level fields (TAGGR style)
                    if let Some(ICRC3Value::Nat(amt)) = fields.get("amt") {
                        tx_amount = Some(amt.clone());
                    }
                }
                let transfer_amount = match tx_amount {
                    Some(amt) => amt,
                    None => continue, // Missing amount
                };

                let tx_from = if let Some(ICRC3Value::Map(tx)) = tx_map {
                    tx.get("from").and_then(|v| {
                        if let ICRC3Value::Array(arr) = v {
                            try_decode_icrc3_account_value(arr)
                        } else {
                            None
                        }
                    })
                } else {
                    fields.get("from").and_then(|v| {
                        if let ICRC3Value::Array(arr) = v {
                            try_decode_icrc3_account_value(arr)
                        } else {
                            None
                        }
                    })
                };
                match tx_from {
                    Some(from) if from == caller_account => (),
                    Some(_) => Err("Transfer from does not match caller")?,
                    None => Err("Missing from account")?,
                }

                let tx_to = if let Some(ICRC3Value::Map(tx)) = tx_map {
                    tx.get("to").and_then(|v| {
                        if let ICRC3Value::Array(arr) = v {
                            try_decode_icrc3_account_value(arr)
                        } else {
                            None
                        }
                    })
                } else {
                    fields.get("to").and_then(|v| {
                        if let ICRC3Value::Array(arr) = v {
                            try_decode_icrc3_account_value(arr)
                        } else {
                            None
                        }
                    })
                };
                match tx_to {
                    Some(to) if to == *kong_backend_account => (),
                    Some(_) => Err("Transfer to does not match Kong backend")?,
                    None => Err("Missing to account")?,
                }

                let tx_spender = if let Some(ICRC3Value::Map(tx)) = tx_map {
                    tx.get("spender").and_then(|v| {
                        if let ICRC3Value::Array(arr) = v {
                            try_decode_icrc3_account_value(arr)
                        } else {
                            None
                        }
                    })
                } else {
                    fields.get("spender").and_then(|v| {
                        if let ICRC3Value::Array(arr) = v {
                            try_decode_icrc3_account_value(arr)
                        } else {
                            None
                        }
                    })
                };
                match tx_spender {
                    None => (), // icrc1_transfer should have no spender
                    Some(_) => Err("Invalid transfer spender")?,
                }

                return Ok(transfer_amount); // success
            }

            Err(format!("Failed to verify {} transfer block id {}", token.symbol(), block_id))?
        }
        Err(e) => Err(e)?,
    }
}

async fn get_transfer_amount_with_query_blocks(
    token: &StableToken,
    block_id: &Nat,
    canister_id: candid::Principal,
    min_valid_timestamp: u64,
    kong_backend_account: &icrc_ledger_types::icrc1::account::Account,
) -> Result<Nat, String> {
    // if ICP ledger, use query_blocks
    // API boundary: ICP ledger's query_blocks requires u64 block index
    let block_args = GetBlocksArgs {
        start: nat_to_u64(block_id).ok_or_else(|| format!("ICP ledger block id {:?} not found", block_id))?,
        length: 1,
    };
    match query_blocks(canister_id, &block_args).await {
        Ok(query_response) => {
            let blocks: Vec<Block> = query_response.blocks;
            let backend_account_id = AccountIdentifier::new(
                &kong_backend_account.owner,
                &Subaccount(kong_backend_account.subaccount.unwrap_or([0; 32])),
            );
            for block in blocks.into_iter() {
                match block.transaction.operation {
                    Some(operation) => match operation {
                        Operation::Transfer {
                            from,
                            to,
                            amount: transfer_amount,
                            ..
                        } => {
                            // ICP ledger seems to combine transfer and transfer_from
                            // use account id for ICP
                            if from != ICNetwork::caller_account_id() {
                                Err("Transfer from does not match caller")?
                            }
                            if to != backend_account_id {
                                Err("Transfer to does not match Kong backend")?
                            }
                            let transfer_amount_nat = Nat::from(transfer_amount.e8s());
                            if block.transaction.created_at_time.timestamp_nanos < min_valid_timestamp {
                                Err("Expired transfer timestamp")?
                            }

                            return Ok(transfer_amount_nat); // success
                        }
                        Operation::Mint { .. } => (),
                        Operation::Burn { .. } => (),
                        Operation::Approve { .. } => (),
                        Operation::TransferFrom { .. } => (), // not supported by ICP ledger
                    },
                    None => Err("No transactions in block")?,
                }
            }

            Err(format!("Failed to verify {} transfer block id {}", token.symbol(), block_id))?
        }
        Err(e) => Err(e.to_string())?,
    }
}

async fn get_transfer_amount_with_get_transactions(
    token: &StableToken,
    block_id: &Nat,
    canister_id: candid::Principal,
    min_valid_timestamp: u64,
    kong_backend_account: &icrc_ledger_types::icrc1::account::Account,
    caller_account: icrc_ledger_types::icrc1::account::Account,
) -> Result<Nat, String> {
    let token_address_with_chain = token.address_with_chain();

    // Handle special tokens (WUMBO, DAMONIC, CLOWN) that use get_transaction (no 's')
    if token_address_with_chain == WUMBO_CANISTER_ID
        || token_address_with_chain == DAMONIC_CANISTER_ID
        || token_address_with_chain == CLOWN_CANISTER_ID
    {
        match ic_cdk::call::Call::unbounded_wait(canister_id, "get_transaction")
            .with_arg((block_id.clone(),))
            .await
        {
            Ok(response) => match response.candid::<(Option<Transaction1>,)>()
                .map_err(|e| format!("{:?}", e))?
                .0 {
                Some(transaction) => {
                    if let Some(transfer) = transaction.transfer {
                        let from = transfer.from;
                        if from != caller_account {
                            Err("Transfer from does not match caller")?
                        }
                        let to = transfer.to;
                        if to != *kong_backend_account {
                            Err("Transfer to does not match Kong backend")?
                        }
                        let transfer_amount = transfer.amount;
                        let timestamp = transaction.timestamp;
                        if timestamp < min_valid_timestamp {
                            Err("Expired transfer timestamp")?
                        }

                        Ok(transfer_amount)
                    } else if let Some(_burn) = transaction.burn {
                        Err("Invalid burn transaction")?
                    } else if let Some(_mint) = transaction.mint {
                        Err("Invalid mint transaction")?
                    } else {
                        Err(format!("Invalid transaction kind: {}", transaction.kind))?
                    }
                }
                None => Err("No transaction found")?,
            },
            Err(e) => Err(format!("{:?}", e))?,
        }
    } else {
        // Standard tokens use get_transactions (with 's')
        let block_args = GetTransactionsRequest {
            start: block_id.clone(),
            length: Nat::from(1_u32),
        };
        match ic_cdk::call::Call::unbounded_wait(canister_id, "get_transactions")
            .with_arg(block_args)
            .await
        {
            Ok(response) => {
                let get_transactions_response = response.candid::<GetTransactionsResponse>()
                    .map_err(|e| format!("{:?}", e))?;
                let transactions = get_transactions_response.transactions;
                for transaction in transactions.into_iter() {
                    if let Some(transfer) = transaction.transfer {
                        let from = transfer.from;
                        if from != caller_account {
                            Err("Transfer from does not match caller")?
                        }
                        let to = transfer.to;
                        if to != *kong_backend_account {
                            Err("Transfer to does not match Kong backend")?
                        }
                        // make sure spender is None, so not an icrc2_transfer_from transaction
                        let spender = transfer.spender;
                        if spender.is_some() {
                            Err("Invalid transfer spender")?
                        }
                        let transfer_amount = transfer.amount;
                        let timestamp = transaction.timestamp;
                        if timestamp < min_valid_timestamp {
                            Err("Expired transfer timestamp")?
                        }

                        return Ok(transfer_amount); // success
                    } else if let Some(_burn) = transaction.burn {
                        // not used
                    } else if let Some(_mint) = transaction.mint {
                        // not used
                    } else if let Some(_approve) = transaction.approve {
                        // not used
                    } else {
                        Err(format!("Invalid transaction kind: {}", transaction.kind))?
                    }
                }

                Err(format!("Failed to verify {} transfer block id {}", token.symbol(), block_id))?
            }
            Err(e) => Err(format!("{:?}", e))?,
        }
    }
}
