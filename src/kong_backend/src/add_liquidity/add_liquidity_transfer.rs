use candid::Nat;
use icrc_ledger_types::icrc1::account::Account;

use super::add_liquidity::TokenIndex;
use super::add_liquidity_args::AddLiquidityArgs;
use super::add_liquidity_reply::AddLiquidityReply;
use super::add_liquidity_transfer_from::archive_to_kong_data;
use super::add_liquidity_transfer_from::{transfer_from_token, update_liquidity_pool};

use crate::helpers::nat_helpers::{nat_subtract, nat_zero};
use crate::ic::{address::Address, network::ICNetwork, transfer::icrc1_transfer, verify_transfer::{verify_and_record_transfer, TokenType, TransferError}};
use crate::stable_claim::{claim_map, stable_claim::StableClaim};
use crate::stable_kong_settings::kong_settings_map;
use crate::stable_pool::pool_map;
use crate::stable_request::{reply::Reply, request::Request, request_map, stable_request::StableRequest, status::StatusCode};
use crate::stable_token::token_map;
use crate::stable_token::{stable_token::StableToken, token::Token};
use crate::stable_transfer::{stable_transfer::StableTransfer, transfer_map, tx_id::TxId};
use crate::stable_tx::{add_liquidity_tx::AddLiquidityTx, stable_tx::StableTx, tx_map};
use crate::stable_user::user_map;

/// Adds liquidity to a pool with automatic amount mismatch handling
/// 
/// # Amount Mismatch Handling
/// 
/// This function handles cases where the actual transfer amounts differ from expected amounts.
/// Since liquidity operations involve two tokens, amount mismatches can occur on either or both tokens.
/// 
/// When an amount mismatch is detected:
/// 1. The transfer is recorded with the actual amount to prevent reuse
/// 2. Both tokens are returned to the user (minus gas fees) if any were transferred
/// 3. An AddLiquidityReply is created with refund details
/// 
/// This ensures users don't lose funds when transfer amounts don't match due to fees or token behavior.
pub async fn add_liquidity_transfer(args: AddLiquidityArgs) -> Result<AddLiquidityReply, String> {
    // user has transferred one of the tokens, we need to log the request immediately and verify the transfer
    // make sure user is registered, if not create a new user with referred_by if specified
    let user_id = user_map::insert(None)?;
    let ts = ICNetwork::get_time();
    let request_id = request_map::insert(&StableRequest::new(user_id, &Request::AddLiquidity(args.clone()), ts));

    let (token_0, tx_id_0, transfer_id_0, token_1, tx_id_1, transfer_id_1) =
        match check_arguments(&args, request_id, ts).await {
            Ok(result) => result,
            Err(TransferError::AmountMismatch { actual, token_id, tx_id, .. }) => {
                // Amount mismatch - we need to record the transfer and return tokens
                let caller_id = ICNetwork::caller_id();
                
                // Record the transfer with actual amount to prevent reuse
                let transfer_id = transfer_map::insert(&StableTransfer {
                    transfer_id: 0,
                    request_id,
                    is_send: true,
                    amount: actual.clone(),
                    token_id,
                    tx_id: TxId::BlockIndex(tx_id),
                    ts,
                });
                
                let mut transfer_ids = vec![transfer_id];
                
                // Get the tokens
                let token_0 = token_map::get_by_token(&args.token_0).ok();
                let token_1 = token_map::get_by_token(&args.token_1).ok();
                
                // Determine which token had the mismatch
                let (return_token_0, return_token_1) = 
                    if token_0.as_ref().map(|t| t.token_id()) == Some(token_id) {
                        // Token 0 had the mismatch
                        (token_0.as_ref(), None)
                    } else {
                        // Token 1 had the mismatch
                        (None, token_1.as_ref())
                    };
                
                // Return tokens and update request with reply
                return_tokens(
                    request_id,
                    user_id,
                    &caller_id,
                    None,
                    return_token_0,
                    &Ok(()),
                    &actual,
                    return_token_1,
                    &Ok(()),
                    &actual,
                    &mut transfer_ids,
                    ts,
                )
                .await;
                
                // Check if return_tokens created an AddLiquidityReply
                match request_map::get_by_request_id(request_id) {
                    Some(request) => match request.reply {
                        Reply::AddLiquidity(reply) => {
                            _ = archive_to_kong_data(request_id);
                            return Ok(reply);
                        },
                        _ => {
                            request_map::update_status(request_id, StatusCode::Failed, None);
                            _ = archive_to_kong_data(request_id);
                            return Err("Amount mismatch: add liquidity cancelled".to_string());
                        }
                    },
                    None => {
                        request_map::update_status(request_id, StatusCode::Failed, None);
                        _ = archive_to_kong_data(request_id);
                        return Err("Amount mismatch: add liquidity cancelled".to_string());
                    }
                }
            }
            Err(e) => {
                request_map::update_status(request_id, StatusCode::Failed, None);
                _ = archive_to_kong_data(request_id);
                return Err(e.to_string());
            }
        };

    let result = match process_add_liquidity(
        request_id,
        user_id,
        token_0.as_ref(),
        tx_id_0.as_ref(),
        transfer_id_0,
        token_1.as_ref(),
        tx_id_1.as_ref(),
        transfer_id_1,
        &args,
        ts,
    )
    .await
    {
        Ok(reply) => {
            request_map::update_status(request_id, StatusCode::Success, None);
            Ok(reply)
        }
        Err(e) => {
            request_map::update_status(request_id, StatusCode::Failed, None);
            Err(e)
        }
    };
    _ = archive_to_kong_data(request_id);

    result
}

pub async fn add_liquidity_transfer_async(args: AddLiquidityArgs) -> Result<u64, String> {
    let user_id = user_map::insert(None)?;
    let ts = ICNetwork::get_time();
    let request_id = request_map::insert(&StableRequest::new(user_id, &Request::AddLiquidity(args.clone()), ts));

    let (token_0, tx_id_0, transfer_id_0, token_1, tx_id_1, transfer_id_1) =
        check_arguments(&args, request_id, ts).await.map_err(|e| {
            request_map::update_status(request_id, StatusCode::Failed, None);
            _ = archive_to_kong_data(request_id);
            e.to_string()
        })?;

    ic_cdk::futures::spawn(async move {
        match process_add_liquidity(
            request_id,
            user_id,
            token_0.as_ref(),
            tx_id_0.as_ref(),
            transfer_id_0,
            token_1.as_ref(),
            tx_id_1.as_ref(),
            transfer_id_1,
            &args,
            ts,
        )
        .await
        {
            Ok(_) => request_map::update_status(request_id, StatusCode::Success, None),
            Err(_) => request_map::update_status(request_id, StatusCode::Failed, None),
        };
        _ = archive_to_kong_data(request_id);
    });

    Ok(request_id)
}

/// if any of the transfer is valid, then must proceed as we may need to return the token back to the user
async fn check_arguments(
    args: &AddLiquidityArgs,
    request_id: u64,
    ts: u64,
) -> Result<
    (
        Option<StableToken>,
        Option<Nat>,
        Result<u64, String>,
        Option<StableToken>,
        Option<Nat>,
        Result<u64, String>,
    ),
    TransferError,
> {
    // update the request status
    request_map::update_status(request_id, StatusCode::Start, None);

    // check token_0. If token_0 is not valid, we still need to continue as we may need to return token_1 to the user
    let token_0 = match token_map::get_by_token(&args.token_0) {
        Ok(token) => Some(token),
        Err(e) => {
            request_map::update_status(request_id, StatusCode::Token0NotFound, Some(&e));
            None
        }
    };

    // check token_1
    let token_1 = match token_map::get_by_token(&args.token_1) {
        Ok(token) => Some(token),
        Err(e) => {
            request_map::update_status(request_id, StatusCode::Token1NotFound, Some(&e));
            None
        }
    };

    // either token_0 and token_1 must be valid token
    if token_0.is_none() && token_1.is_none() {
        return Err(TransferError::TransferNotFound { error: "Token_0 or Token_1 is required".to_string() });
    }

    // check tx_id_0 is valid block index Nat
    let tx_id_0 = match &args.tx_id_0 {
        Some(TxId::BlockIndex(tx_id)) => Some(tx_id.clone()),
        _ => None,
    };

    let tx_id_1 = match &args.tx_id_1 {
        Some(TxId::BlockIndex(tx_id)) => Some(tx_id.clone()),
        _ => None,
    };

    // either tx_id_0 or tx_id_1 must be valid
    if tx_id_0.is_none() && tx_id_1.is_none() {
        return Err(TransferError::TransferNotFound { error: "Tx_id_0 or Tx_id_1 is required".to_string() });
    }

    // transfer_id_0 is used to store if the transfer was successful
    let transfer_id_0 = match &tx_id_0 {
        Some(tx_id) if token_0.is_some() => {
            let token = token_0.as_ref().unwrap();
            if token.is_removed() {
                return Err(TransferError::TransferNotFound { error: "Token_0 is suspended or removed".to_string() });
            }
            let amount = &args.amount_0;
            let transfer_id = verify_and_record_transfer(request_id, TokenType::Token0, token, tx_id, amount, ts).await?;
            Ok(transfer_id)
        }
        _ => Err("Tx_id_0 not specified".to_string()),
    };

    let transfer_id_1 = match &tx_id_1 {
        Some(tx_id) if token_1.is_some() => {
            let token = token_1.as_ref().unwrap();
            if token.is_removed() {
                return Err(TransferError::TransferNotFound { error: "Token_1 is suspended or removed".to_string() });
            }
            let amount = &args.amount_1;
            let transfer_id = verify_and_record_transfer(request_id, TokenType::Token1, token, tx_id, amount, ts).await?;
            Ok(transfer_id)
        }
        _ => Err("Tx_id_1 not specified".to_string()),
    };

    // one of the transfers must be successful
    if transfer_id_0.is_err() && transfer_id_1.is_err() {
        return Err(TransferError::TransferNotFound { error: "Failed to verify transfers".to_string() });
    }

    Ok((token_0, tx_id_0, transfer_id_0, token_1, tx_id_1, transfer_id_1))
}

#[allow(clippy::too_many_arguments)]
async fn process_add_liquidity(
    request_id: u64,
    user_id: u32,
    token_0: Option<&StableToken>,
    tx_id_0: Option<&Nat>,
    transfer_id_0: Result<u64, String>,
    token_1: Option<&StableToken>,
    tx_id_1: Option<&Nat>,
    transfer_id_1: Result<u64, String>,
    args: &AddLiquidityArgs,
    ts: u64,
) -> Result<AddLiquidityReply, String> {
    let add_amount_0 = &args.amount_0;
    let add_amount_1 = &args.amount_1;

    let caller_id = ICNetwork::caller_id();
    let kong_backend = kong_settings_map::get().kong_backend;
    let mut transfer_ids = Vec::new();

    let mut transfer_0 = match transfer_id_0 {
        Ok(transfer_id) => {
            // user issued an icrc1_transfer and has been verified in check_arguments()
            transfer_ids.push(transfer_id);
            Ok(())
        }
        // either icrc1_transfer could not be verified (transfer_id_0.is_err()) or must be an icrc2_transfer_from (tx_id_0.is_none())
        // which is handled later on
        Err(e) => Err(e),
    };

    let mut transfer_1 = match transfer_id_1 {
        Ok(transfer_id) => {
            // user issued an icrc1_transfer and has been verified in check_arguments()
            transfer_ids.push(transfer_id);
            Ok(())
        }
        Err(e) => Err(e),
    };

    let pool = if token_0.is_some() && token_1.is_some() {
        let tok_0 = token_0.unwrap();
        let tok_id_0 = tok_0.token_id();
        let tok_1 = token_1.unwrap();
        let tok_id_1 = tok_1.token_id();
        match pool_map::get_by_token_ids(tok_id_0, tok_id_1) {
            Some(pool) => {
                if transfer_0.is_err() && tx_id_0.is_none() {
                    transfer_0 = transfer_from_token(
                        request_id,
                        &caller_id,
                        &TokenIndex::Token0,
                        tok_0,
                        add_amount_0,
                        &kong_backend,
                        &mut transfer_ids,
                        ts,
                    )
                    .await;
                }
                if transfer_0.is_ok() && transfer_1.is_err() && tx_id_1.is_none() {
                    transfer_1 = transfer_from_token(
                        request_id,
                        &caller_id,
                        &TokenIndex::Token1,
                        tok_1,
                        add_amount_1,
                        &kong_backend,
                        &mut transfer_ids,
                        ts,
                    )
                    .await;
                }
                pool
            }
            None => {
                request_map::update_status(request_id, StatusCode::PoolNotFound, None);
                return_tokens(
                    request_id,
                    user_id,
                    &caller_id,
                    None,
                    token_0,
                    &transfer_0,
                    add_amount_0,
                    token_1,
                    &transfer_1,
                    add_amount_1,
                    &mut transfer_ids,
                    ts,
                )
                .await;
                Err(format!("Req #{} failed. Pool not found", request_id))?
            }
        }
    } else {
        request_map::update_status(request_id, StatusCode::PoolNotFound, None);
        return_tokens(
            request_id,
            user_id,
            &caller_id,
            None,
            token_0,
            &transfer_0,
            add_amount_0,
            token_1,
            &transfer_1,
            add_amount_1,
            &mut transfer_ids,
            ts,
        )
        .await;
        Err(format!("Req #{} failed. Pool not found", request_id))?
    };

    // both transfers must be successful
    if transfer_0.is_err() || transfer_1.is_err() {
        return_tokens(
            request_id,
            user_id,
            &caller_id,
            Some(pool.pool_id),
            token_0,
            &transfer_0,
            add_amount_0,
            token_1,
            &transfer_1,
            add_amount_1,
            &mut transfer_ids,
            ts,
        )
        .await;
        if transfer_0.is_err() {
            return Err(format!("Req #{} failed. {}", request_id, transfer_0.unwrap_err()));
        } else {
            return Err(format!("Req #{} failed. {}", request_id, transfer_1.unwrap_err()));
        };
    }

    // re-calculate with latest pool state and make sure amounts are valid
    let (pool, amount_0, amount_1, add_lp_token_amount) =
        match update_liquidity_pool(request_id, user_id, &pool, add_amount_0, add_amount_1, ts) {
            Ok((pool, amount_0, amount_1, add_lp_token_amount)) => (pool, amount_0, amount_1, add_lp_token_amount),
            Err(e) => {
                // LP amounts are incorrect. return token_0 and token_1 back to user
                return_tokens(
                    request_id,
                    user_id,
                    &caller_id,
                    Some(pool.pool_id),
                    token_0,
                    &transfer_0,
                    add_amount_0,
                    token_1,
                    &transfer_1,
                    add_amount_1,
                    &mut transfer_ids,
                    ts,
                )
                .await;
                Err(format!("Req #{} failed. {}", request_id, e))?
            }
        };

    // successful, add tx and update request with reply
    let add_liquidity_tx = AddLiquidityTx::new_success(
        pool.pool_id,
        user_id,
        request_id,
        &amount_0,
        &amount_1,
        &add_lp_token_amount,
        &transfer_ids,
        &Vec::new(),
        ts,
    );
    let tx_id = tx_map::insert(&StableTx::AddLiquidity(add_liquidity_tx.clone()));
    let reply = match tx_map::get_by_user_and_token_id(Some(tx_id), None, None, None).first() {
        Some(StableTx::AddLiquidity(add_liquidity_tx)) => {
            AddLiquidityReply::try_from(add_liquidity_tx).unwrap_or_else(|_| AddLiquidityReply::failed(pool.pool_id, request_id, &transfer_ids, &Vec::new(), ts))
        },
        _ => AddLiquidityReply::failed(pool.pool_id, request_id, &transfer_ids, &Vec::new(), ts),
    };
    request_map::update_reply(request_id, Reply::AddLiquidity(reply.clone()));

    Ok(reply)
}


#[allow(clippy::too_many_arguments)]
async fn return_tokens(
    request_id: u64,
    user_id: u32,
    to_principal_id: &Account,
    pool_id: Option<u32>,
    token_0: Option<&StableToken>,
    transfer_0: &Result<(), String>,
    amount_0: &Nat,
    token_1: Option<&StableToken>,
    transfer_1: &Result<(), String>,
    amount_1: &Nat,
    transfer_ids: &mut Vec<u64>,
    ts: u64,
) {
    let mut claim_ids = Vec::new();

    if token_0.is_some() && transfer_0.is_ok() {
        let token_0 = token_0.unwrap();
        return_token(
            request_id,
            user_id,
            to_principal_id,
            &TokenIndex::Token0,
            token_0,
            amount_0,
            transfer_ids,
            &mut claim_ids,
            ts,
        )
        .await;
    }

    if token_1.is_some() && transfer_1.is_ok() {
        let token_1 = token_1.unwrap();
        return_token(
            request_id,
            user_id,
            to_principal_id,
            &TokenIndex::Token1,
            token_1,
            amount_1,
            transfer_ids,
            &mut claim_ids,
            ts,
        )
        .await;
    }

    let pool_id = pool_id.unwrap_or(0);
    let reply = AddLiquidityReply::failed(pool_id, request_id, transfer_ids, &claim_ids, ts);
    request_map::update_reply(request_id, Reply::AddLiquidity(reply));
}

#[allow(clippy::too_many_arguments)]
async fn return_token(
    request_id: u64,
    user_id: u32,
    to_principal_id: &Account,
    token_index: &TokenIndex,
    token: &StableToken,
    amount: &Nat,
    transfer_ids: &mut Vec<u64>,
    claim_ids: &mut Vec<u64>,
    ts: u64,
) {
    let token_id = token.token_id();
    let fee = token.fee();

    match token_index {
        TokenIndex::Token0 => request_map::update_status(request_id, StatusCode::ReturnToken0, None),
        TokenIndex::Token1 => request_map::update_status(request_id, StatusCode::ReturnToken1, None),
    };

    let amount_with_gas = nat_subtract(amount, &fee).unwrap_or(nat_zero());
    match icrc1_transfer(&amount_with_gas, to_principal_id, token, None).await {
        Ok(block_id) => {
            let transfer_id = transfer_map::insert(&StableTransfer {
                transfer_id: 0,
                request_id,
                is_send: false,
                amount: amount_with_gas,
                token_id,
                tx_id: TxId::BlockIndex(block_id),
                ts,
            });
            transfer_ids.push(transfer_id);
            match token_index {
                TokenIndex::Token0 => request_map::update_status(request_id, StatusCode::ReturnToken0Success, None),
                TokenIndex::Token1 => request_map::update_status(request_id, StatusCode::ReturnToken1Success, None),
            };
        }
        Err(e) => {
            let claim = StableClaim::new(
                user_id,
                token_id,
                amount,
                Some(request_id),
                Some(Address::PrincipalId(*to_principal_id)),
                ts,
            );
            let claim_id = claim_map::insert(&claim);
            claim_ids.push(claim_id);
            let message = format!("Saved as claim #{}. {}", claim_id, e);
            match token_index {
                TokenIndex::Token0 => request_map::update_status(request_id, StatusCode::ReturnToken0Failed, Some(&message)),
                TokenIndex::Token1 => request_map::update_status(request_id, StatusCode::ReturnToken1Failed, Some(&message)),
            };
        }
    }
}
