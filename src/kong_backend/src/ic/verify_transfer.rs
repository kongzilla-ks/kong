use candid::Principal;
use candid::{CandidType, Deserialize, Nat};
use ic_ledger_types::{query_blocks, AccountIdentifier, Block, GetBlocksArgs, Operation, Subaccount, Tokens};
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

/// Verifies a transfer by checking the ledger.
/// For ICRC3 tokens, it tries ICRC3 methods first, falling back to traditional methods.
/// For non-ICRC3 tokens, it uses the traditional verification methods.
pub async fn verify_transfer(token: &StableToken, block_id: &Nat, amount: &Nat) -> Result<(), String> {
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
                return verify_transfer_with_icrc3_get_blocks(
                    token,
                    block_id,
                    amount,
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
                return verify_transfer_with_query_blocks(token, block_id, amount, canister_id, min_valid_timestamp, kong_backend_account)
                    .await;
            }

            // otherwise, use get_transactions
            verify_transfer_with_get_transactions(
                token,
                block_id,
                amount,
                canister_id,
                min_valid_timestamp,
                kong_backend_account,
                caller_account,
            )
            .await
        }
        _ => Err("Verify transfer not supported for this token")?,
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
async fn verify_transfer_with_icrc3_get_blocks(
    token: &StableToken,
    block_id: &Nat,
    amount: &Nat,
    canister_id: candid::Principal,
    token_address_with_chain: &String,
    min_valid_timestamp: u64,
    kong_backend_account: &icrc_ledger_types::icrc1::account::Account,
    caller_account: icrc_ledger_types::icrc1::account::Account,
) -> Result<(), String> {
    // Prepare request arguments based on token type
    let blocks_result = if *token_address_with_chain == TAGGR_CANISTER_ID {
        // TAGGR uses a different format
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
                match tx_amount {
                    Some(transfer_amount) if transfer_amount == *amount => (),
                    Some(transfer_amount) => Err(format!("Invalid transfer amount: rec {:?} exp {:?}", transfer_amount, amount))?,
                    None => continue, // Missing amount
                }

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

                return Ok(()); // success
            }

            Err(format!("Failed to verify {} transfer block id {}", token.symbol(), block_id))?
        }
        Err(e) => Err(e)?,
    }
}

async fn verify_transfer_with_query_blocks(
    token: &StableToken,
    block_id: &Nat,
    amount: &Nat,
    canister_id: candid::Principal,
    min_valid_timestamp: u64,
    kong_backend_account: &icrc_ledger_types::icrc1::account::Account,
) -> Result<(), String> {
    // if ICP ledger, use query_blocks
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
            let amount = Tokens::from_e8s(nat_to_u64(amount).ok_or("Invalid ICP amount")?);
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
                            if transfer_amount != amount {
                                Err(format!("Invalid transfer amount: rec {:?} exp {:?}", transfer_amount, amount))?
                            }
                            if block.transaction.created_at_time.timestamp_nanos < min_valid_timestamp {
                                Err("Expired transfer timestamp")?
                            }

                            return Ok(()); // success
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

async fn verify_transfer_with_get_transactions(
    token: &StableToken,
    block_id: &Nat,
    amount: &Nat,
    canister_id: candid::Principal,
    min_valid_timestamp: u64,
    kong_backend_account: &icrc_ledger_types::icrc1::account::Account,
    caller_account: icrc_ledger_types::icrc1::account::Account,
) -> Result<(), String> {
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
            Ok(response) => match response.candid::<(Option<Transaction1>,)>().map_err(|e| format!("{:?}", e))?.0 {
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
                        if transfer_amount != *amount {
                            Err(format!("Invalid transfer amount: rec {:?} exp {:?}", transfer_amount, amount))?
                        }
                        let timestamp = transaction.timestamp;
                        if timestamp < min_valid_timestamp {
                            Err("Expired transfer timestamp")?
                        }

                        Ok(())
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
                let get_transactions_response = response.candid::<GetTransactionsResponse>().map_err(|e| format!("{:?}", e))?;
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
                        if transfer_amount != *amount {
                            Err(format!("Invalid transfer amount: rec {:?} exp {:?}", transfer_amount, amount))?
                        }
                        let timestamp = transaction.timestamp;
                        if timestamp < min_valid_timestamp {
                            Err("Expired transfer timestamp")?
                        }

                        return Ok(()); // success
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
