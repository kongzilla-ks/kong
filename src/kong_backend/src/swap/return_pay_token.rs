use candid::Nat;
use icrc_ledger_types::icrc1::account::Account;

use crate::chains::chains::{IC_CHAIN, SOL_CHAIN};
use crate::helpers::nat_helpers::{nat_subtract, nat_zero};
use crate::ic::{address::Address, transfer::icrc1_transfer};
use crate::solana::stable_memory::get_solana_transaction;
use crate::stable_claim::{claim_map, stable_claim::StableClaim};
use crate::stable_request::reply::Reply;
use crate::stable_request::request_map;
use crate::stable_request::status::StatusCode;
use crate::stable_token::{stable_token::StableToken, token::Token};
use crate::stable_transfer::{stable_transfer::StableTransfer, transfer_map, tx_id::TxId};
use crate::solana::create_solana_swap_job::create_solana_swap_job;

use super::swap_reply::SwapReply;

#[allow(clippy::too_many_arguments)]
pub async fn return_pay_token(
    request_id: u64,
    user_id: u32,
    to_principal_id: &Account,
    pay_token: &StableToken,
    pay_amount: &Nat,
    receive_token: Option<&StableToken>,
    transfer_ids: &mut Vec<u64>,
    ts: u64,
) {
    let token_id = pay_token.token_id();
    let fee = pay_token.fee();

    let mut claim_ids = Vec::new();

    request_map::update_status(request_id, StatusCode::ReturnPayToken, None);

    let pay_amount_with_gas = nat_subtract(pay_amount, &fee).unwrap_or(nat_zero());

    if pay_token.chain() == SOL_CHAIN {
        let sender_address = match get_solana_sender_from_transfers(transfer_ids) {
            Ok(addr) => addr,
            Err(e) => {
                // this should however not happen since the tx is already in the DB with metadata
                request_map::update_status(
                    request_id,
                    StatusCode::ReturnPayTokenFailed,
                    Some(&format!(
                        "Cannot return Solana tokens: sender address not found in metadata. Need to implement metadata fetching: {}",
                        e
                    )),
                );
                let reply = SwapReply::failed(request_id, pay_token, pay_amount, receive_token, transfer_ids, &claim_ids, ts);
                request_map::update_reply(request_id, Reply::Swap(reply));
                return;
            }
        };

        let to_address = Address::SolanaAddress(sender_address.clone());
        match create_solana_swap_job(request_id, user_id, pay_token, &pay_amount_with_gas, &to_address, ts).await {
            Ok(job_id) => {
                let transfer_id = transfer_map::insert(&StableTransfer {
                    transfer_id: 0,
                    request_id,
                    is_send: false,
                    amount: pay_amount_with_gas,
                    token_id,
                    tx_id: TxId::TransactionId(format!("job_{}", job_id)),
                    ts,
                    refund_transfer_id: None,
                });
                transfer_ids.push(transfer_id);
                request_map::update_status(
                    request_id,
                    StatusCode::ReturnPayTokenSuccess,
                    Some(&format!("Solana swap job #{} created", job_id)),
                );
            }
            Err(e) => {
                let claim = StableClaim::new(
                    user_id,
                    token_id,
                    pay_amount,
                    Some(request_id),
                    Some(Address::SolanaAddress(sender_address)),
                    ts,
                );
                let claim_id = claim_map::insert(&claim);
                claim_ids.push(claim_id);
                request_map::update_status(
                    request_id,
                    StatusCode::ReturnPayTokenFailed,
                    Some(&format!("Saved as claim #{}. Error creating swap job: {}", claim_id, e)),
                );
            }
        }
    } else if pay_token.chain() == IC_CHAIN {
        match icrc1_transfer(&pay_amount_with_gas, to_principal_id, pay_token, None).await {
            Ok(tx_id) => {
                let transfer_id = transfer_map::insert(&StableTransfer {
                    transfer_id: 0,
                    request_id,
                    is_send: false,
                    amount: pay_amount_with_gas,
                    token_id,
                    tx_id: TxId::BlockIndex(tx_id),
                    ts,
                    refund_transfer_id: None,
                });
                transfer_ids.push(transfer_id);
                request_map::update_status(request_id, StatusCode::ReturnPayTokenSuccess, None);
            }
            Err(e) => {
                let claim = StableClaim::new(
                    user_id,
                    token_id,
                    pay_amount,
                    Some(request_id),
                    Some(Address::PrincipalId(*to_principal_id)),
                    ts,
                );
                let claim_id = claim_map::insert(&claim);
                claim_ids.push(claim_id);
                request_map::update_status(
                    request_id,
                    StatusCode::ReturnPayTokenFailed,
                    Some(&format!("Saved as claim #{}. {}", claim_id, e)),
                );
            }
        }
    } else {
        // Unsupported chain TODO, we should never get here
        let claim = StableClaim::new(
            user_id,
            token_id,
            pay_amount,
            Some(request_id),
            Some(Address::PrincipalId(*to_principal_id)),
            ts,
        );
        let claim_id = claim_map::insert(&claim);
        claim_ids.push(claim_id);
        request_map::update_status(
            request_id,
            StatusCode::ReturnPayTokenFailed,
            Some(&format!("Saved as claim #{}. Unsupported chain: {}", claim_id, pay_token.chain())),
        );
    };

    let reply = SwapReply::failed(request_id, pay_token, pay_amount, receive_token, transfer_ids, &claim_ids, ts);
    request_map::update_reply(request_id, Reply::Swap(reply));
}

fn get_solana_sender_from_transfers(transfer_ids: &[u64]) -> Result<String, String> {
    for transfer_id in transfer_ids {
        if let Some(transfer) = transfer_map::get_by_transfer_id(*transfer_id) {
            if let TxId::TransactionId(tx_signature) = &transfer.tx_id {
                if let Some(notification) = get_solana_transaction(tx_signature.to_string()) {
                    // parse metadata to get sender, can be in different fields due to the different types of transactions
                    if let Some(metadata_json) = notification.metadata {
                        let metadata: serde_json::Value =
                            serde_json::from_str(&metadata_json).map_err(|e| format!("Failed to parse transaction metadata: {}", e))?;

                        if let Some(sender) = metadata
                            .get("sender")
                            .or_else(|| metadata.get("authority"))
                            .or_else(|| metadata.get("sender_wallet"))
                            .and_then(|v| v.as_str())
                        {
                            return Ok(sender.to_string());
                        }
                    }
                }
            }
        }
    }

    Err("No Solana sender address found in transfers".to_string())
}
