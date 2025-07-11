use candid::Nat;

use crate::helpers::nat_helpers::nat_is_zero;
use crate::ic::address::Address;
use crate::ic::address_helpers::get_address;
use crate::ic::network::ICNetwork;
use crate::ic::transfer_verification::{verify_and_record_transfer, TokenType, TransferError};
use crate::stable_kong_settings::kong_settings_map;
use crate::stable_request::{request::Request, request_map, stable_request::StableRequest, status::StatusCode, reply::Reply};
use crate::stable_token::token::Token;
use crate::stable_token::{stable_token::StableToken, token_map};
use crate::stable_transfer::{stable_transfer::StableTransfer, transfer_map, tx_id::TxId};
use crate::stable_user::user_map;

use super::archive_to_kong_data::archive_to_kong_data;
use super::payment_verifier::{PaymentVerification, PaymentVerifier};
use super::return_pay_token::return_pay_token;
use super::send_receive_token::send_receive_token;
use super::swap_args::SwapArgs;
use super::swap_calc::SwapCalc;
use super::swap_reply::SwapReply;
use super::update_liquidity_pool::update_liquidity_pool;

use crate::chains::chains::SOL_CHAIN;

/// Processes a token swap with automatic amount mismatch handling
/// 
/// # Amount Mismatch Handling
/// 
/// When a user initiates a swap, they specify a pay_amount. However, the actual amount recorded
/// on the blockchain may differ due to transfer fees or token contract behavior. This function
/// handles such mismatches by:
/// 
/// 1. **Detection**: Using `verify_and_record_transfer` to compare actual vs expected amounts
/// 2. **Recording**: Always recording the transfer to prevent reuse, even on mismatch
/// 3. **Recovery**: Automatically returning tokens to the user (minus gas fees) when mismatch occurs
/// 4. **Reporting**: Providing clear error messages and creating a SwapReply with refund details
/// 
/// # Flow
/// 
/// 1. Validate arguments and verify the pay token transfer
/// 2. If amount mismatch is detected:
///    - Extract the actual amount from the recorded transfer
///    - Call `return_pay_token` to refund the user
///    - Return a SwapReply indicating the refund status
/// 3. Otherwise, proceed with the swap normally
pub async fn swap_transfer(args: SwapArgs) -> Result<SwapReply, String> {
    
    // as user has transferred the pay token, we need to log the request immediately and verify the transfer
    // make sure user is registered, if not create a new user with referred_by if specified
    
    // Check if this is a Solana swap to allow anonymous users
    let allow_anonymous = match token_map::get_by_token(&args.pay_token) {
        Ok(token) => token.chain() == SOL_CHAIN,
        Err(_) => false,
    };
    
    let user_id = user_map::insert_with_anonymous_option(args.referred_by.as_deref(), allow_anonymous)?;
    let ts = ICNetwork::get_time();
    // insert request into request_map so we have immediate record of this
    let request_id = request_map::insert(&StableRequest::new(user_id, &Request::Swap(args.clone()), ts));
    
    // check arguments and verify the pay token transfer
    let check_result = check_arguments(&args, request_id, ts).await;
    
    let (pay_token, pay_amount, pay_transfer_id) = match check_result {
        Ok(result) => result,
        Err(TransferError::AmountMismatch { actual, transfer_id, .. }) => {
                // Amount Mismatch Recovery Process:
                // 1. The transfer verification detected that actual_amount != expected_amount
                // 2. The transfer was already recorded in transfer_map to prevent reuse
                // 3. We now need to return the actual amount (minus gas) to the user
                // 4. Create a SwapReply to inform the user about the refund
                let pay_token = match token_map::get_by_token(&args.pay_token) {
                    Ok(token) => token,
                    Err(_) => {
                        request_map::update_status(request_id, StatusCode::Failed, None);
                        let _ = archive_to_kong_data(request_id);
                        return Err("Failed to get pay token".to_string());
                    }
                };
                
                // Return the tokens to the user with the actual amount
                let caller_id = ICNetwork::caller_id();
                let mut transfer_ids = vec![transfer_id];
                return_pay_token(
                    request_id,
                    user_id,
                    &caller_id,
                    &pay_token,
                    &actual,
                    None,
                    &mut transfer_ids,
                    ts,
                )
                .await;
                
                // Check if return_pay_token created a SwapReply
                match request_map::get_by_request_id(request_id) {
                    Some(request) => match request.reply {
                        Reply::Swap(reply) => {
                            let _ = archive_to_kong_data(request_id);
                            return Ok(reply);
                        },
                        _ => {
                            request_map::update_status(request_id, StatusCode::Failed, None);
                            let _ = archive_to_kong_data(request_id);
                            return Err("Amount mismatch: swap cancelled".to_string());
                        }
                    },
                    None => {
                        request_map::update_status(request_id, StatusCode::Failed, None);
                        let _ = archive_to_kong_data(request_id);
                        return Err("Amount mismatch: swap cancelled".to_string());
                    }
                }
        }
        Err(e) => {
            // Other errors don't require token return
            request_map::update_status(request_id, StatusCode::Failed, None);
            let _ = archive_to_kong_data(request_id);
            return Err(e.to_string());
        }
    };

    // initialize transfer_ids to keep track of transfers
    let mut transfer_ids = Vec::new();
    let process_swap_result = process_swap(
        request_id,
        user_id,
        &pay_token,
        &pay_amount,
        pay_transfer_id,
        &args,
        &mut transfer_ids,
        ts,
    )
    .await;
    
    let (receive_token, receive_amount_with_fees_and_gas, to_address, mid_price, price, slippage, swaps) = match process_swap_result {
        Ok(result) => result,
        Err(_) => {
            // Check if return_pay_token was called and created a SwapReply
            match request_map::get_by_request_id(request_id) {
                Some(request) => match request.reply {
                    Reply::Swap(reply) => {
                        let _ = archive_to_kong_data(request_id);
                        return Ok(reply);
                    },
                    _ => {
                        request_map::update_status(request_id, StatusCode::Failed, None);
                        let _ = archive_to_kong_data(request_id);
                        return Err("Swap failed and no reply was created".to_string());
                    }
                },
                None => {
                    request_map::update_status(request_id, StatusCode::Failed, None);
                    let _ = archive_to_kong_data(request_id);
                    return Err("Swap failed and request not found".to_string());
                }
            }
        }
    };

    let result = send_receive_token(
        request_id,
        user_id,
        &pay_token,
        &pay_amount,
        &receive_token,
        &receive_amount_with_fees_and_gas,
        &to_address,
        &mut transfer_ids,
        mid_price,
        price,
        slippage,
        &swaps,
        ts,
    )
    .await;

    request_map::update_status(request_id, StatusCode::Success, None);
    let _ = archive_to_kong_data(request_id);

    Ok(result)
}

pub async fn swap_transfer_async(args: SwapArgs) -> Result<u64, String> {
    // Check if this is a Solana swap to allow anonymous users
    let allow_anonymous = match token_map::get_by_token(&args.pay_token) {
        Ok(token) => token.chain() == SOL_CHAIN,
        Err(_) => false,
    };
    
    let user_id = user_map::insert_with_anonymous_option(args.referred_by.as_deref(), allow_anonymous)?;
    let ts = ICNetwork::get_time();
    let request_id = request_map::insert(&StableRequest::new(user_id, &Request::Swap(args.clone()), ts));
    let (pay_token, pay_amount, pay_transfer_id) = check_arguments(&args, request_id, ts).await.map_err(|e| {
        request_map::update_status(request_id, StatusCode::Failed, None);
        let _ = archive_to_kong_data(request_id);
        e.to_string()
    })?;

    ic_cdk::futures::spawn(async move {
        let mut transfer_ids = Vec::new();

        let Ok((receive_token, receive_amount_with_fees_and_gas, to_address, mid_price, price, slippage, swaps)) = process_swap(
            request_id,
            user_id,
            &pay_token,
            &pay_amount,
            pay_transfer_id,
            &args,
            &mut transfer_ids,
            ts,
        )
        .await
        else {
            request_map::update_status(request_id, StatusCode::Failed, None);
            let _ = archive_to_kong_data(request_id);
            return;
        };

        ic_cdk::futures::spawn(async move {
            send_receive_token(
                request_id,
                user_id,
                &pay_token,
                &pay_amount,
                &receive_token,
                &receive_amount_with_fees_and_gas,
                &to_address,
                &mut transfer_ids,
                mid_price,
                price,
                slippage,
                &swaps,
                ts,
            )
            .await;

            let _ = archive_to_kong_data(request_id);
        });

        request_map::update_status(request_id, StatusCode::Success, None);
    });

    Ok(request_id)
}

/// check pay token is valid and verify the transfer
async fn check_arguments(args: &SwapArgs, request_id: u64, ts: u64) -> Result<(StableToken, Nat, u64), TransferError> {
    request_map::update_status(request_id, StatusCode::Start, None);

    // check pay_token is a valid token. We need to know the canister id so return here if token is not valid
    let pay_token = token_map::get_by_token(&args.pay_token).map_err(|e| {
        request_map::update_status(request_id, StatusCode::PayTokenNotFound, Some(&e));
        TransferError::TransferNotFound { error: e }
    })?;


    let pay_amount = args.pay_amount.clone();

    
    // Check token type to determine payment verification path
    if pay_token.chain() == SOL_CHAIN {
        // Solana tokens require signature verification
        let verifier = PaymentVerifier::new(ICNetwork::caller());
        let verification = verifier.verify_payment(args, &pay_token, &pay_amount)
            .await
            .map_err(|e| {
                request_map::update_status(request_id, StatusCode::VerifyPayTokenFailed, Some(&e));
                TransferError::TransferNotFound { error: e }
            })?;

        match verification {
            PaymentVerification::SolanaPayment { tx_signature, .. } => {
                // Check if this Solana transaction has already been used
                if transfer_map::contains_tx_signature(pay_token.token_id(), &tx_signature) {
                    return Err(TransferError::TransferNotFound { 
                    error: "Solana transaction signature already used for this token".to_string() 
                });
                }
                
                // For Solana payments, we create a transfer record with the transaction hash
                let transfer_id = transfer_map::insert(&StableTransfer {
                    transfer_id: 0,
                    request_id,
                    is_send: true,
                    amount: pay_amount.clone(),
                    token_id: pay_token.token_id(),
                    tx_id: TxId::TransactionId(tx_signature),
                    ts,
                });
                request_map::update_status(request_id, StatusCode::VerifyPayTokenSuccess, None);
                Ok((pay_token, pay_amount, transfer_id))
            }
            PaymentVerification::IcpPayment { block_index, .. } => {
                // IC payment verified through signature (rare case)
                let transfer_id = transfer_map::insert(&StableTransfer {
                    transfer_id: 0,
                    request_id,
                    is_send: true,
                    amount: pay_amount.clone(),
                    token_id: pay_token.token_id(),
                    tx_id: TxId::BlockIndex(block_index.into()),
                    ts,
                });
                request_map::update_status(request_id, StatusCode::VerifyPayTokenSuccess, None);
                Ok((pay_token, pay_amount, transfer_id))
            }
        }
    } else {
        // IC token path - verify transfer without signature
        // For IC tokens, we need a valid block index
        let transfer_id = match &args.pay_tx_id {
            Some(pay_tx_id) => match pay_tx_id {
                TxId::BlockIndex(pay_tx_id) => verify_transfer_token(request_id, &pay_token, pay_tx_id, &pay_amount, ts).await?,
                TxId::TransactionId(_) => {
                    // TransactionId is only valid for cross-chain swaps with signatures
                    request_map::update_status(request_id, StatusCode::PayTxIdNotSupported, None);
                    return Err(TransferError::TransferNotFound { 
                        error: "TransactionId requires signature for cross-chain swaps. For IC tokens, use BlockIndex.".to_string() 
                    });
                }
            },
            None => {
                request_map::update_status(request_id, StatusCode::PayTxIdNotFound, None);
                return Err(TransferError::TransferNotFound { 
                    error: "Pay tx_id required for IC token swaps".to_string() 
                });
            }
        };
        Ok((pay_token, pay_amount, transfer_id))
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_swap(
    request_id: u64,
    user_id: u32,
    pay_token: &StableToken,
    pay_amount: &Nat,
    pay_transfer_id: u64,
    args: &SwapArgs,
    transfer_ids: &mut Vec<u64>,
    ts: u64,
) -> Result<(StableToken, Nat, Address, f64, f64, f64, Vec<SwapCalc>), String> {
    let caller_id = ICNetwork::caller_id();

    transfer_ids.push(pay_transfer_id);

    let receive_token = token_map::get_by_token(&args.receive_token).inspect_err(|e| {
        request_map::update_status(request_id, StatusCode::ReceiveTokenNotFound, Some(e));
    })?;
    if receive_token.is_removed() {
        request_map::update_status(request_id, StatusCode::ReceiveTokenNotFound, None);
        return_pay_token(
            request_id,
            user_id,
            &caller_id,
            pay_token,
            pay_amount,
            Some(&receive_token),
            transfer_ids,
            ts,
        )
        .await;
        Err(format!("Req #{} failed. Receive token is suspended or removed", request_id))?
    }
    let receive_amount = args.receive_amount.as_ref();

    if pay_token.is_removed() {
        request_map::update_status(request_id, StatusCode::PayTokenNotFound, None);
        return_pay_token(
            request_id,
            user_id,
            &caller_id,
            pay_token,
            pay_amount,
            Some(&receive_token),
            transfer_ids,
            ts,
        )
        .await;
        Err(format!("Req #{} failed. Pay token is suspended or removed", request_id))?
    }
    if nat_is_zero(pay_amount) {
        request_map::update_status(request_id, StatusCode::PayTokenAmountIsZero, None);
        return_pay_token(
            request_id,
            user_id,
            &caller_id,
            pay_token,
            pay_amount,
            Some(&receive_token),
            transfer_ids,
            ts,
        )
        .await;
        Err(format!("Req #{} failed. Pay amount is zero", request_id))?
    }

    // use specified max slippage or use default
    let max_slippage = args.max_slippage.unwrap_or(kong_settings_map::get().default_max_slippage);
    // use specified address or default to caller's principal id
    let to_address = match args.receive_address {
        Some(ref address) => match get_address(&receive_token, address) {
            Ok(address) => address,
            Err(e) => {
                request_map::update_status(request_id, StatusCode::ReceiveAddressNotFound, None);
                return_pay_token(
                    request_id,
                    user_id,
                    &caller_id,
                    pay_token,
                    pay_amount,
                    Some(&receive_token),
                    transfer_ids,
                    ts,
                )
                .await;
                Err(format!("Req #{} failed. {}", request_id, e))?
            }
        },
        None => Address::PrincipalId(caller_id),
    };

    let (receive_amount_with_fees_and_gas, mid_price, price, slippage, swaps) =
        match update_liquidity_pool(request_id, pay_token, pay_amount, &receive_token, receive_amount, max_slippage) {
            Ok((receive_amount, mid_price, price, slippage, swaps)) => (receive_amount, mid_price, price, slippage, swaps),
            Err(e) => {
                return_pay_token(
                    request_id,
                    user_id,
                    &caller_id,
                    pay_token,
                    pay_amount,
                    Some(&receive_token),
                    transfer_ids,
                    ts,
                )
                .await;
                Err(format!("Req #{} failed. {}", request_id, e))?
            }
        };

    request_map::update_status(request_id, StatusCode::SwapSuccess, None);

    Ok((
        receive_token,
        receive_amount_with_fees_and_gas,
        to_address,
        mid_price,
        price,
        slippage,
        swaps,
    ))
}

async fn verify_transfer_token(request_id: u64, token: &StableToken, tx_id: &Nat, amount: &Nat, ts: u64) -> Result<u64, TransferError> {
    // Use the shared utility for consistent transfer verification
    verify_and_record_transfer(request_id, TokenType::PayToken, token, tx_id, amount, ts).await
}
