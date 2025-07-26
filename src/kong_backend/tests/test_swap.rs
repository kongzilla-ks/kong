// src/kong_backend/tests/test_swap.rs
pub mod common;

// --- Imports ---
use anyhow::Result;
use candid::{decode_one, encode_one, Nat, Principal};
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc1::transfer::{TransferArg, TransferError};
use icrc_ledger_types::icrc2::approve::{ApproveArgs, ApproveError};

// Use the common setup function that includes pool creation
use common::setup_with_pool::{
    setup_swap_test_environment,
    TOKEN_A_FEE,
    TOKEN_B_FEE_ICP,
};
use common::identity::get_identity_from_pem_file;
use common::icrc1_ledger::{create_icrc1_ledger_simple, SimpleLedgerConfig};

// Import AddPoolArgs and AddPoolReply
use kong_backend::add_pool::add_pool_args::AddPoolArgs;
use kong_backend::add_pool::add_pool_reply::AddPoolReply;

// Import kong_backend types needed for tests
use kong_backend::stable_transfer::tx_id::TxId;
use kong_backend::swap::swap_args::SwapArgs;
use kong_backend::swap::swap_reply::SwapReply;
use kong_backend::add_token::add_token_args::AddTokenArgs;
use kong_backend::add_token::add_token_reply::AddTokenReply;

const CONTROLLER_PEM_FILE: &str = "tests/common/identity.pem";

// --- Balance Check Helper (used by multiple tests) ---
// This function works for both ICRC1 tokens and ICP ledger (which implements ICRC1 interface)
fn get_icrc1_balance(ic: &pocket_ic::PocketIc, ledger_id: Principal, account: Account) -> Nat {
    let payload = encode_one(account).expect("Failed to encode account for balance_of");
    let response = ic
        .query_call(ledger_id, Principal::anonymous(), "icrc1_balance_of", payload)
        .expect("Failed to call icrc1_balance_of");
    decode_one::<Nat>(&response).expect("Failed to decode icrc1_balance_of response")
}

// --- Test Functions ---

#[test]
fn test_swap_approve_transfer_from_a_to_b() {
    // --- Arrange ---
    // Use the common setup function
    let setup = setup_swap_test_environment().expect("Failed to setup swap test environment");
    let ic = setup.ic;
    let kong_backend = setup.kong_backend;
    let user_principal = setup.user_principal;
    let user_account = setup.user_account;
    let kong_account = setup.kong_account;
    let token_a_ledger_id = setup.token_a_ledger_id;
    let token_b_ledger_id = setup.token_b_ledger_id;
    let token_a_str = setup.token_a_str;
    let token_b_str = setup.token_b_str;

    let approve_swap_amount_a = setup.base_approve_swap_a;
    let amount_out_min_b_approve_swap = Nat::from(1u64); // Expect at least 1 tiny unit of B out

    // --- Act ---

    // 1. Approve Token A for Swap
    let approve_total_amount_a = approve_swap_amount_a.clone() + Nat::from(TOKEN_A_FEE); // Amount + fee for subsequent transfer_from
    let approve_args_swap_a = ApproveArgs {
        from_subaccount: None,
        spender: kong_account, // Approve Kong backend to spend
        amount: approve_total_amount_a.clone(), // Approve enough for the swap amount + transfer_from fee
        expected_allowance: None,
        expires_at: None,
        fee: None, // Use default fee (user pays this approve fee now)
        memo: None,
        created_at_time: None,
    };
    let approve_payload_swap_a = encode_one(approve_args_swap_a).expect("Failed to encode approve_args_swap_a");
    let approve_response_swap_a = ic
        .update_call(token_a_ledger_id, user_principal, "icrc2_approve", approve_payload_swap_a) // Called by USER
        .expect("Failed to call icrc2_approve for Token A swap");
    let approve_result_swap_a = decode_one::<Result<Nat, ApproveError>>(&approve_response_swap_a)
        .expect("Failed to decode icrc2_approve response for Token A swap");
    assert!(
        approve_result_swap_a.is_ok(),
        "Approve Token A swap failed: {:?}",
        approve_result_swap_a
    );

    // Get balances *before* the swap call (after approve)
    let user_balance_a_before_swap = get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    let user_balance_b_before_swap = get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    let kong_balance_a_before_swap = get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    let kong_balance_b_before_swap = get_icrc1_balance(&ic, token_b_ledger_id, kong_account);

    // 2. Perform the Swap (Token A -> Token B using transfer_from)
    let swap_args_approve = SwapArgs {
        pay_token: token_a_str.clone(),
        pay_amount: Nat::from(approve_swap_amount_a), // The actual amount to swap
        pay_tx_id: None,                           // Swap uses transfer_from, so no tx_id needed here
        receive_token: token_b_str.clone(),
        receive_amount: Some(amount_out_min_b_approve_swap.clone()), // Minimum expected
        receive_address: Some(user_principal.to_text()),             // Explicitly set receive address
        max_slippage: Some(50.0),                                    // Explicitly allow up to 50% slippage for this test
        referred_by: None,
        ..Default::default()
    };
    let swap_payload_approve = encode_one(&swap_args_approve).expect("Failed to encode swap_args_approve ");

    let swap_response_bytes_approve = ic
        .update_call(kong_backend, user_principal, "swap", swap_payload_approve)
        .expect("Failed to call swap (approve flow)");

    // --- Assert ---
    let swap_result_approve =
        decode_one::<Result<SwapReply, String>>(&swap_response_bytes_approve).expect("Failed to decode swap_transfer response (approve flow)");


    assert!(
        swap_result_approve.is_ok(),
        "swap_transfer call failed (approve flow): {:?}\nArgs: {:?}",
        swap_result_approve, swap_args_approve
    );
    let swap_reply_approve = swap_result_approve.unwrap(); // Unwrap the Ok result
    let amount_out_b_actual_approve = swap_reply_approve.receive_amount; // Get actual amount from reply

    // Check minimum amount
    assert!(
        amount_out_b_actual_approve >= amount_out_min_b_approve_swap,
        "Actual amount out ({}) is less than minimum expected ({}) in approve swap",
        amount_out_b_actual_approve,
        amount_out_min_b_approve_swap
    );

    // Verify Balances After Swap
    let user_balance_a_after_approve_swap = get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    let user_balance_b_after_approve_swap = get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    let kong_balance_a_after_approve_swap = get_icrc1_balance(&ic, token_a_ledger_id, kong_account); // Check Kong
    let kong_balance_b_after_approve_swap = get_icrc1_balance(&ic, token_b_ledger_id, kong_account); // Check Kong

    // Expected User A: BalanceBeforeSwap - SwapAmountIn - SwapTransferFromFee (paid from approved amount)
    // Note: User already paid approve fee before 'BalanceBeforeSwap' snapshot
    let expected_user_a_after_approve_swap = user_balance_a_before_swap.clone() - approve_swap_amount_a.clone() - Nat::from(TOKEN_A_FEE);
    assert_eq!(
        user_balance_a_after_approve_swap, expected_user_a_after_approve_swap,
        "User balance A after approve/transfer_from swap. Expected {}, got {}",
        expected_user_a_after_approve_swap, user_balance_a_after_approve_swap
    );

    // Expected User B: BalanceBeforeSwap + AmountReceivedB
    let expected_user_b_after_approve_swap = user_balance_b_before_swap.clone() + amount_out_b_actual_approve.clone();
    assert_eq!(
        user_balance_b_after_approve_swap, expected_user_b_after_approve_swap,
        "User balance B after approve/transfer_from swap. Expected {}, got {}",
        expected_user_b_after_approve_swap, user_balance_b_after_approve_swap
    );

    // Expected Kong A: BalanceBeforeSwap + SwapAmountIn
    let expected_kong_a_after_approve_swap = kong_balance_a_before_swap.clone() + approve_swap_amount_a.clone();
    assert_eq!(
        kong_balance_a_after_approve_swap, expected_kong_a_after_approve_swap,
        "Kong balance A after approve/transfer_from swap. Expected {}, got {}",
        expected_kong_a_after_approve_swap, kong_balance_a_after_approve_swap
    );
    // Expected Kong B: BalanceBeforeSwap - AmountReceivedB - TransferFeeB (Kong pays this)
    let expected_kong_b_after_approve_swap =
        kong_balance_b_before_swap.clone() - amount_out_b_actual_approve.clone() - Nat::from(TOKEN_B_FEE_ICP);
    assert_eq!(
        kong_balance_b_after_approve_swap, expected_kong_b_after_approve_swap,
        "Kong balance B after approve/transfer_from swap. Expected {}, got {}",
        expected_kong_b_after_approve_swap, kong_balance_b_after_approve_swap
    );

}


#[test]
fn test_swap_direct_transfer_a_to_b() {
    // --- Arrange ---
    // Use the common setup function
    let setup = setup_swap_test_environment().expect("Failed to setup swap test environment");
    let ic = setup.ic;
    let kong_backend = setup.kong_backend;
    let user_principal = setup.user_principal;
    let user_account = setup.user_account;
    let kong_account = setup.kong_account;
    let token_a_ledger_id = setup.token_a_ledger_id;
    let token_b_ledger_id = setup.token_b_ledger_id;
    let token_a_str = setup.token_a_str;
    let token_b_str = setup.token_b_str;

    let direct_swap_amount_a = setup.base_transfer_swap_a;
    let amount_out_min_b_direct_swap = Nat::from(1u64);

    // Get balances before direct transfer swap
    let user_balance_a_before_direct_swap = get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    let user_balance_b_before_direct_swap = get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    let kong_balance_a_before_direct_swap = get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    let kong_balance_b_before_direct_swap = get_icrc1_balance(&ic, token_b_ledger_id, kong_account);


    // --- Act ---
    // 1. User transfers Token A directly to Kong for the swap
    let transfer_direct_swap_a_args = TransferArg {
        from_subaccount: None,
        to: kong_account, // Send TO Kong
        amount: Nat::from(direct_swap_amount_a),
        fee: None, // Use default fee (User pays this)
        memo: None,
        created_at_time: None,
    };
    let transfer_direct_swap_a_payload =
        encode_one(transfer_direct_swap_a_args).expect("Failed to encode transfer_direct_swap_a_args");
    let transfer_direct_swap_a_response = ic
        .update_call(
            token_a_ledger_id,
            user_principal,
            "icrc1_transfer",
            transfer_direct_swap_a_payload,
        ) // Called by USER
        .expect("Failed to call icrc1_transfer for Token A direct swap");
    let transfer_direct_swap_a_result = decode_one::<Result<Nat, TransferError>>(&transfer_direct_swap_a_response)
        .expect("Failed to decode icrc1_transfer response for Token A direct swap");
    assert!(
        transfer_direct_swap_a_result.is_ok(),
        "User transfer Token A for direct swap failed: {:?}",
        transfer_direct_swap_a_result
    );
    let tx_id_direct_swap_a = transfer_direct_swap_a_result.unwrap(); // Capture the block index (tx_id)

    // Check user balance A immediately after transfer (before swap call)
    let user_balance_a_after_direct_transfer = get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    let expected_user_a_after_direct_transfer =
        user_balance_a_before_direct_swap.clone() - direct_swap_amount_a.clone() - Nat::from(TOKEN_A_FEE);
    assert_eq!(
        user_balance_a_after_direct_transfer, expected_user_a_after_direct_transfer,
        "User balance A after direct transfer, before swap call. Expected {}, got {}",
        expected_user_a_after_direct_transfer, user_balance_a_after_direct_transfer
    );

     // 2. Perform the Swap (Token A -> Token B using direct transfer tx_id)
    let swap_args_direct_a = SwapArgs {
        pay_token: token_a_str.clone(),
        pay_amount: Nat::from(direct_swap_amount_a),
        pay_tx_id: Some(TxId::BlockIndex(tx_id_direct_swap_a)), // Provide the tx_id
        receive_token: token_b_str.clone(),
        receive_amount: Some(amount_out_min_b_direct_swap.clone()), // Minimum expected
        receive_address: Some(user_principal.to_text()),           // Explicitly set receive address
        max_slippage: Some(50.0),                                  // Explicitly allow up to 50% slippage
        referred_by: None,
        ..Default::default()
    };
    let swap_payload_direct_a = encode_one(&swap_args_direct_a).expect("Failed to encode swap_args_direct_a ");

    let swap_response_bytes_direct_a = ic
        .update_call(kong_backend, user_principal, "swap", swap_payload_direct_a)
        .expect("Failed to call swap (direct flow A->B)");

    // --- Assert ---
    let swap_result_direct_a = decode_one::<Result<SwapReply, String>>(&swap_response_bytes_direct_a)
        .expect("Failed to decode swap_transfer response (direct flow A->B)");

    assert!(
        swap_result_direct_a.is_ok(),
        "swap_transfer call failed (direct flow A->B): {:?}\nArgs: {:?}",
        swap_result_direct_a, swap_args_direct_a
    );
    let swap_reply_direct_a = swap_result_direct_a.unwrap();
    let amount_out_b_actual_direct = swap_reply_direct_a.receive_amount;

    // Check minimum amount requirement
    assert!(
        amount_out_b_actual_direct >= amount_out_min_b_direct_swap,
        "Actual amount out ({}) is less than minimum expected ({}) in direct swap A->B",
        amount_out_b_actual_direct,
        amount_out_min_b_direct_swap
    );

    // Verify Balances After Direct Transfer Swap (A -> B)
    let user_balance_a_after_direct_swap_a = get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    let user_balance_b_after_direct_swap_a = get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    let kong_balance_a_after_direct_swap_a = get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    let kong_balance_b_after_direct_swap_a = get_icrc1_balance(&ic, token_b_ledger_id, kong_account);

    // Expected User A: Unchanged from after the direct transfer
    assert_eq!(
        user_balance_a_after_direct_swap_a, user_balance_a_after_direct_transfer,
        "User balance A after direct swap A->B (should be same as after transfer). Expected {}, got {}",
        user_balance_a_after_direct_transfer, user_balance_a_after_direct_swap_a
    );

    // Expected User B: BalanceBeforeDirectSwap + AmountReceivedB
    let expected_user_b_after_direct_swap_a = user_balance_b_before_direct_swap.clone() + amount_out_b_actual_direct.clone();
    assert_eq!(
        user_balance_b_after_direct_swap_a, expected_user_b_after_direct_swap_a,
        "User balance B after direct swap A->B. Expected {}, got {}",
        expected_user_b_after_direct_swap_a, user_balance_b_after_direct_swap_a
    );

    // Expected Kong A: BalanceBeforeDirectSwap + PayAmount (from user's direct transfer)
    let expected_kong_a_after_direct_swap_a = kong_balance_a_before_direct_swap.clone() + direct_swap_amount_a.clone();
    assert_eq!(
        kong_balance_a_after_direct_swap_a, expected_kong_a_after_direct_swap_a,
        "Kong balance A after direct swap A->B. Expected {}, got {}",
        expected_kong_a_after_direct_swap_a, kong_balance_a_after_direct_swap_a
    );
    // Expected Kong B: BalanceBeforeDirectSwap - AmountReceivedB - TransferFeeB (Kong pays this)
    let expected_kong_b_after_direct_swap_a =
        kong_balance_b_before_direct_swap.clone() - amount_out_b_actual_direct.clone() - Nat::from(TOKEN_B_FEE_ICP);
    assert_eq!(
        kong_balance_b_after_direct_swap_a, expected_kong_b_after_direct_swap_a,
        "Kong balance B after direct swap A->B. Expected {}, got {}",
        expected_kong_b_after_direct_swap_a, kong_balance_b_after_direct_swap_a
    );
}


#[test]
fn test_swap_with_transfer_fee_token() {
    // --- Arrange ---
    // Use the common setup function
    let setup = setup_swap_test_environment().expect("Failed to setup swap test environment");
    let ic = setup.ic;
    let kong_backend = setup.kong_backend;
    let user_principal = setup.user_principal;
    let user_account = setup.user_account;
    let kong_account = setup.kong_account;
    let token_b_ledger_id = setup.token_b_ledger_id;
    let token_b_str = setup.token_b_str;
    
    // Create a token with 1% transfer fee
    let controller = get_identity_from_pem_file(CONTROLLER_PEM_FILE).expect("Failed to get controller identity");
    let controller_principal = controller.sender().expect("Failed to get controller principal");
    
    // Create token with 1% fee 
    let fee_token_config = SimpleLedgerConfig {
        token_symbol: "FEE".to_string(),
        token_name: "Fee Token".to_string(),
        decimals: 8,
        transfer_fee: Nat::from(100_000u64), // 0.001 FEE per transfer (with 8 decimals)
        initial_balances: vec![(user_account, Nat::from(10_000_000_000_000u64))], // 100,000 tokens
        controller: controller_principal,
    };
    
    let fee_token_ledger = create_icrc1_ledger_simple(&ic, fee_token_config)
        .expect("Failed to create fee token");
    
    // Add fee token to Kong backend
    let add_token_args = AddTokenArgs {
        token: format!("IC.{}", fee_token_ledger.to_text()),
    };
    let add_token_response = ic
        .update_call(
            kong_backend,
            controller_principal,
            "add_token",
            encode_one(add_token_args).expect("Failed to encode add_token_args"),
        )
        .expect("Failed to add fee token");
    
    let add_token_result = decode_one::<Result<AddTokenReply, String>>(&add_token_response)
        .expect("Failed to decode add_token_response");
    assert!(add_token_result.is_ok(), "Failed to add token: {:?}", add_token_result);
    let _add_token_reply = add_token_result.unwrap();
    
    let fee_token_str = format!("IC.{}", fee_token_ledger.to_text());
    
    // Mint some Token B to user for liquidity
    let mint_b_for_pool = TransferArg {
        from_subaccount: None,
        to: user_account,
        fee: None,
        created_at_time: None,
        memo: None,
        amount: Nat::from(5_000_000_000_000u64), // 50,000 Token B
    };
    let mint_b_response = ic
        .update_call(token_b_ledger_id, controller_principal, "icrc1_transfer", encode_one(&mint_b_for_pool).expect("Failed to encode"))
        .expect("Failed to mint Token B");
    let mint_b_result = decode_one::<Result<Nat, TransferError>>(&mint_b_response).expect("Failed to decode");
    assert!(mint_b_result.is_ok(), "Failed to mint Token B: {:?}", mint_b_result);
    
    // --- Act ---
    
    // 1. Transfer tokens to Kong for liquidity
    let liquidity_fee_amount = Nat::from(5_000_000_000_000u64); // 50,000 FEE tokens
    let liquidity_b_amount = Nat::from(1_000_000_000_000u64); // 10,000 Token B
    
    // Transfer fee tokens for liquidity
    let transfer_fee_args = TransferArg {
        from_subaccount: None,
        to: kong_account,
        amount: liquidity_fee_amount.clone(),
        fee: None,
        memo: None,
        created_at_time: None,
    };
    let transfer_fee_response = ic
        .update_call(fee_token_ledger, user_principal, "icrc1_transfer", encode_one(transfer_fee_args).expect("Failed to encode"))
        .expect("Failed to transfer fee tokens");
    let fee_tx_id = decode_one::<Result<Nat, TransferError>>(&transfer_fee_response)
        .expect("Failed to decode")
        .expect("Transfer failed");
    
    // Transfer Token B for liquidity
    let transfer_b_args = TransferArg {
        from_subaccount: None,
        to: kong_account,
        amount: liquidity_b_amount.clone(),
        fee: None,
        memo: None,
        created_at_time: None,
    };
    let transfer_b_response = ic
        .update_call(token_b_ledger_id, user_principal, "icrc1_transfer", encode_one(transfer_b_args).expect("Failed to encode"))
        .expect("Failed to transfer Token B");
    let b_tx_id = decode_one::<Result<Nat, TransferError>>(&transfer_b_response)
        .expect("Failed to decode")
        .expect("Transfer failed");
    
    // 2. Create pool with fee token and Token B
    let add_pool_args = AddPoolArgs {
        token_0: fee_token_str.clone(),
        amount_0: liquidity_fee_amount.clone(),
        tx_id_0: Some(TxId::BlockIndex(fee_tx_id)),
        token_1: token_b_str.clone(),
        amount_1: liquidity_b_amount.clone(),
        tx_id_1: Some(TxId::BlockIndex(b_tx_id)),
        lp_fee_bps: Some(30),
        signature_0: None,
        signature_1: None
    };
    let add_pool_response = ic
        .update_call(kong_backend, user_principal, "add_pool", encode_one(&add_pool_args).expect("Failed to encode"))
        .expect("Failed to add pool");
    let add_pool_result = decode_one::<Result<AddPoolReply, String>>(&add_pool_response).expect("Failed to decode");
    assert!(add_pool_result.is_ok(), "Failed to add pool: {:?}", add_pool_result);
    
    // 3. Perform swap with fee token - transfer exact amount user wants to swap
    let swap_amount = Nat::from(1_000_000_000_000u64); // 10,000 FEE tokens
    let fee_token_fee = Nat::from(100_000u64); // 0.001 FEE
    
    // Get balances before transfer
    let user_fee_balance_before = get_icrc1_balance(&ic, fee_token_ledger, user_account);
    let kong_fee_balance_before = get_icrc1_balance(&ic, fee_token_ledger, kong_account);
    
    
    // Transfer fee tokens to Kong (user will pay the transfer fee)
    let transfer_swap_args = TransferArg {
        from_subaccount: None,
        to: kong_account,
        amount: swap_amount.clone(),
        fee: None,
        memo: None,
        created_at_time: None,
    };
    let transfer_swap_response = ic
        .update_call(fee_token_ledger, user_principal, "icrc1_transfer", encode_one(transfer_swap_args).expect("Failed to encode"))
        .expect("Failed to transfer for swap");
    let swap_tx_id = decode_one::<Result<Nat, TransferError>>(&transfer_swap_response)
        .expect("Failed to decode")
        .expect("Transfer failed");
    
    // Check balances after transfer
    let user_fee_balance_after_transfer = get_icrc1_balance(&ic, fee_token_ledger, user_account);
    let kong_fee_balance_after_transfer = get_icrc1_balance(&ic, fee_token_ledger, kong_account);
    
    
    // Verify the transfer amounts
    let user_paid = user_fee_balance_before.clone() - user_fee_balance_after_transfer.clone();
    let kong_received = kong_fee_balance_after_transfer.clone() - kong_fee_balance_before.clone();
    
    
    // User should pay swap_amount + fee
    assert_eq!(user_paid, swap_amount.clone() + fee_token_fee.clone(), "User should pay exact amount + fee");
    // Kong should receive exactly swap_amount (fee goes to ledger)
    assert_eq!(kong_received, swap_amount.clone(), "Kong should receive exact swap amount");
    
    // 4. Call swap with the amount Kong actually received
    let swap_args = SwapArgs {
        pay_token: fee_token_str.clone(),
        pay_amount: swap_amount.clone(), // The amount we intended to swap
        pay_tx_id: Some(TxId::BlockIndex(swap_tx_id)),
        receive_token: token_b_str.clone(),
        receive_amount: Some(Nat::from(1u64)), // Minimum expected
        receive_address: Some(user_principal.to_text()),
        max_slippage: Some(50.0),
        referred_by: None,
        ..Default::default()
    };
    
    let user_b_balance_before_swap = get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    
    let swap_response = ic
        .update_call(kong_backend, user_principal, "swap", encode_one(&swap_args).expect("Failed to encode"))
        .expect("Failed to call swap");
    let swap_result = decode_one::<Result<SwapReply, String>>(&swap_response).expect("Failed to decode");
    
    
    // --- Assert ---
    assert!(swap_result.is_ok(), "Swap should succeed with fee token");
    let swap_reply = swap_result.unwrap();
    
    // Verify user received Token B
    let user_b_balance_after_swap = get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    let b_received = user_b_balance_after_swap - user_b_balance_before_swap;
    
    assert_eq!(b_received, swap_reply.receive_amount, "User should receive the amount specified in reply");
    assert!(b_received > Nat::from(0u64), "User should receive some Token B");
    
}

#[test]
fn test_swap_direct_transfer_b_to_a() {
    // --- Arrange ---
    // Use the common setup function
    let setup = setup_swap_test_environment().expect("Failed to setup swap test environment");
    let ic = setup.ic;
    let kong_backend = setup.kong_backend;
    let user_principal = setup.user_principal;
    let user_account = setup.user_account;
    let kong_account = setup.kong_account;
    let token_a_ledger_id = setup.token_a_ledger_id;
    let token_b_ledger_id = setup.token_b_ledger_id;
    let token_a_str = setup.token_a_str;
    let token_b_str = setup.token_b_str;

    let direct_swap_amount_b = setup.base_transfer_swap_b;
    let amount_out_min_a_direct_swap = Nat::from(1u64); // Expect at least 1 tiny unit of A out

    // Get balances before direct transfer swap B->A
    let user_balance_a_before_direct_swap_b = get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    let user_balance_b_before_direct_swap_b = get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    let kong_balance_a_before_direct_swap_b = get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    let kong_balance_b_before_direct_swap_b = get_icrc1_balance(&ic, token_b_ledger_id, kong_account);


    // --- Act ---
    // 1. User transfers Token B directly to Kong for the swap
    let transfer_direct_swap_b_args = TransferArg {
        from_subaccount: None,
        to: kong_account, // Send TO Kong
        amount: Nat::from(direct_swap_amount_b),
        fee: None, // Use default fee (User pays this)
        memo: None,
        created_at_time: None,
    };
    let transfer_direct_swap_b_payload =
        encode_one(transfer_direct_swap_b_args).expect("Failed to encode transfer_direct_swap_b_args");
    let transfer_direct_swap_b_response = ic
        .update_call(
            token_b_ledger_id, // Use Token B ledger
            user_principal,
            "icrc1_transfer",
            transfer_direct_swap_b_payload,
        ) // Called by USER
        .expect("Failed to call icrc1_transfer for Token B direct swap");
    let transfer_direct_swap_b_result = decode_one::<Result<Nat, TransferError>>(&transfer_direct_swap_b_response)
        .expect("Failed to decode icrc1_transfer response for Token B direct swap");
    assert!(
        transfer_direct_swap_b_result.is_ok(),
        "User transfer Token B for direct swap failed: {:?}",
        transfer_direct_swap_b_result
    );
    let tx_id_direct_swap_b = transfer_direct_swap_b_result.unwrap(); // Capture the block index (tx_id)

    // Check user balance B immediately after transfer (before swap call)
    let user_balance_b_after_direct_transfer = get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    let expected_user_b_after_direct_transfer =
        user_balance_b_before_direct_swap_b.clone() - direct_swap_amount_b.clone() - Nat::from(TOKEN_B_FEE_ICP);
    assert_eq!(
        user_balance_b_after_direct_transfer, expected_user_b_after_direct_transfer,
        "User balance B after direct transfer, before swap call. Expected {}, got {}",
        expected_user_b_after_direct_transfer, user_balance_b_after_direct_transfer
    );

    // 2. Perform the Swap (Token B -> Token A using direct transfer tx_id)
    let swap_args_direct_b = SwapArgs {
        pay_token: token_b_str.clone(), // Pay with B
        pay_amount: Nat::from(direct_swap_amount_b),
        pay_tx_id: Some(TxId::BlockIndex(tx_id_direct_swap_b)), // Provide the tx_id
        receive_token: token_a_str.clone(),                     // Receive A
        receive_amount: Some(amount_out_min_a_direct_swap.clone()), // Minimum expected A
        receive_address: Some(user_principal.to_text()),        // Explicitly set receive address
        max_slippage: Some(50.0),                               // Explicitly allow up to 50% slippage
        referred_by: None,
        ..Default::default()
    };
    let swap_payload_direct_b = encode_one(&swap_args_direct_b).expect("Failed to encode swap_args_direct_b ");

    let swap_response_bytes_direct_b = ic
        .update_call(kong_backend, user_principal, "swap", swap_payload_direct_b)
        .expect("Failed to call swap (direct flow B->A)");

    // --- Assert ---
    let swap_result_direct_b = decode_one::<Result<SwapReply, String>>(&swap_response_bytes_direct_b)
        .expect("Failed to decode swap_transfer response (direct flow B->A)");

    assert!(
        swap_result_direct_b.is_ok(),
        "swap_transfer call failed (direct flow B->A): {:?}\nArgs: {:?}",
        swap_result_direct_b, swap_args_direct_b
    );
    let swap_reply_direct_b = swap_result_direct_b.unwrap();
    let amount_out_a_actual_direct = swap_reply_direct_b.receive_amount;

    // Check minimum amount requirement
    assert!(
        amount_out_a_actual_direct >= amount_out_min_a_direct_swap,
        "Actual amount out ({}) is less than minimum expected ({}) in direct swap B->A",
        amount_out_a_actual_direct,
        amount_out_min_a_direct_swap
    );

    // Verify Balances After Direct Transfer Swap (B -> A)
    let user_balance_a_after_direct_swap_b = get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    let user_balance_b_after_direct_swap_b = get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    let kong_balance_a_after_direct_swap_b = get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    let kong_balance_b_after_direct_swap_b = get_icrc1_balance(&ic, token_b_ledger_id, kong_account);

    // Expected User B: Unchanged from after the direct transfer
    assert_eq!(
        user_balance_b_after_direct_swap_b, user_balance_b_after_direct_transfer,
        "User balance B after direct swap B->A (should be same as after transfer). Expected {}, got {}",
        user_balance_b_after_direct_transfer, user_balance_b_after_direct_swap_b
    );

    // Expected User A: BalanceBeforeDirectSwapB + AmountReceivedA
    let expected_user_a_after_direct_swap_b = user_balance_a_before_direct_swap_b.clone() + amount_out_a_actual_direct.clone();
    assert_eq!(
        user_balance_a_after_direct_swap_b, expected_user_a_after_direct_swap_b,
        "User balance A after direct swap B->A. Expected {}, got {}",
        expected_user_a_after_direct_swap_b, user_balance_a_after_direct_swap_b
    );

    // Expected Kong B: BalanceBeforeDirectSwapB + PayAmountB (from user's direct transfer)
    let expected_kong_b_after_direct_swap_b = kong_balance_b_before_direct_swap_b.clone() + direct_swap_amount_b.clone();
    assert_eq!(
        kong_balance_b_after_direct_swap_b, expected_kong_b_after_direct_swap_b,
        "Kong balance B after direct swap B->A. Expected {}, got {}",
        expected_kong_b_after_direct_swap_b, kong_balance_b_after_direct_swap_b
    );
    // Expected Kong A: BalanceBeforeDirectSwapB - AmountReceivedA - TransferFeeA (Kong pays this)
    let expected_kong_a_after_direct_swap_b =
        kong_balance_a_before_direct_swap_b.clone() - amount_out_a_actual_direct.clone() - Nat::from(TOKEN_A_FEE);
    assert_eq!(
        kong_balance_a_after_direct_swap_b, expected_kong_a_after_direct_swap_b,
        "Kong balance A after direct swap B->A. Expected {}, got {}",
        expected_kong_a_after_direct_swap_b, kong_balance_a_after_direct_swap_b
    );

}

#[test]
fn test_swap_duplicate_block_id_protection() {
    // This test verifies that the system protects against using the same block ID twice
    // This is a critical security test to prevent double spending for the same block ID for icrc1_transfer
    
    // --- Arrange ---
    let setup = setup_swap_test_environment().expect("Failed to setup swap test environment");
    let ic = setup.ic;
    let ledger_id_a = setup.token_a_ledger_id;
    let ledger_id_b = setup.token_b_ledger_id;
    let kong_id = setup.kong_backend;
    let controller = setup.controller_principal;
    let user = setup.user_principal;
    let kong_account = setup.kong_account;
    let user_account = setup.user_account;
    
    // Setup: User needs Token A for the swap
    let transfer_amount = Nat::from(10_000_000u64); // 10 tokens
    let transfer_args = TransferArg {
        from_subaccount: None,
        to: user_account,
        amount: transfer_amount.clone(),
        fee: None,
        memo: None,
        created_at_time: None,
    };
    
    // Controller transfers Token A to user
    let payload = encode_one(transfer_args).expect("Failed to encode transfer args");
    ic.update_call(
        ledger_id_a,
        controller,
        "icrc1_transfer",
        payload,
    )
    .expect("Failed to transfer Token A to user");
    
    // --- Act ---
    // User transfers Token A to Kong (this is the single blockchain transaction)
    let transfer_to_kong_args = TransferArg {
        from_subaccount: None,
        to: kong_account,
        amount: transfer_amount.clone(),
        fee: None,
        memo: None,
        created_at_time: None,
    };
    
    let payload = encode_one(transfer_to_kong_args).expect("Failed to encode transfer args");
    let response = ic
        .update_call(
            ledger_id_a,
            user,
            "icrc1_transfer",
            payload,
        )
        .expect("Failed to transfer Token A to Kong");
    
    let block_id = match decode_one::<Result<Nat, TransferError>>(&response)
        .expect("Failed to decode transfer response") {
        Ok(block_id) => block_id,
        Err(e) => panic!("Transfer to Kong failed: {:?}", e),
    };
    
    // Get token strings for swap
    let token_a_str = format!("IC.{}", ledger_id_a.to_string());
    let token_b_str = format!("IC.{}", ledger_id_b.to_string());
    
    // First swap attempt with the block ID
    let swap_args_1 = SwapArgs {
        pay_token: token_a_str.clone(),
        pay_amount: transfer_amount.clone(),
        pay_tx_id: Some(TxId::BlockIndex(block_id.clone())),
        receive_token: token_b_str.clone(),
        receive_amount: Some(Nat::from(1u64)), // Minimum expected
        receive_address: None,
        max_slippage: Some(100.0), // 100% slippage tolerance for test
        referred_by: None,
        pay_signature: None,
    };
    
    let payload_1 = encode_one(swap_args_1).expect("Failed to encode swap args 1");
    let swap_response_1 = ic
        .update_call(
            kong_id,
            user,
            "swap",
            payload_1,
        )
        .expect("Failed to call swap 1");
    
    let swap_reply_1 = decode_one::<Result<SwapReply, String>>(&swap_response_1)
        .expect("Failed to decode swap response 1");
    
    // Second swap attempt with the SAME block ID (simulating race condition)
    let swap_args_2 = SwapArgs {
        pay_token: token_a_str.clone(),
        pay_amount: transfer_amount.clone(),
        pay_tx_id: Some(TxId::BlockIndex(block_id.clone())), // SAME block ID!
        receive_token: token_b_str.clone(),
        receive_amount: Some(Nat::from(1u64)),
        receive_address: None,
        max_slippage: Some(100.0),
        referred_by: None,
        pay_signature: None,
    };
    
    let payload_2 = encode_one(swap_args_2).expect("Failed to encode swap args 2");
    let swap_response_2 = ic
        .update_call(
            kong_id,
            user,
            "swap",
            payload_2,
        )
        .expect("Failed to call swap 2");
    
    let swap_reply_2 = decode_one::<Result<SwapReply, String>>(&swap_response_2)
        .expect("Failed to decode swap response 2");
    
    // --- Assert ---
    // Print the results for debugging
    println!("\n=== SWAP TEST RESULTS ===");
    println!("Swap 1 result: {:?}", swap_reply_1);
    println!("Swap 2 result: {:?}", swap_reply_2);
    
    // Check if the second swap failed due to slippage rather than duplicate block ID
    if let Err(error2) = &swap_reply_2 {
        if error2.contains("Slippage exceeded") {
            println!("\n⚠️  WARNING: Swap 2 failed due to slippage, not duplicate block ID!");
            println!("This suggests the race condition might exist but was masked by slippage.");
        }
    }
    
    // One swap should succeed, the other should fail with duplicate block ID error
    match (&swap_reply_1, &swap_reply_2) {
        (Ok(reply1), Err(error2)) => {
            // First succeeded, second failed - this is the expected behavior
            assert_eq!(reply1.status, "Success", "First swap should succeed");
            assert!(
                error2.contains("Duplicate block id") || error2.contains("duplicate"),
                "Second swap should fail with duplicate block ID error, got: {}",
                error2
            );
        }
        (Err(error1), Ok(reply2)) => {
            // First failed, second succeeded - this would indicate a race condition
            // but is acceptable if the system processes them in reverse order
            assert!(
                error1.contains("Duplicate block id") || error1.contains("duplicate"),
                "First swap should fail with duplicate block ID error if second succeeds, got: {}",
                error1
            );
            assert_eq!(reply2.status, "Success", "Second swap should succeed if first failed");
        }
        (Ok(reply1), Ok(reply2)) => {
            // CRITICAL: Both succeeded - this is the vulnerability!
            panic!(
                "SECURITY VULNERABILITY: Both swaps succeeded with the same block ID!\n\
                First swap: request_id={}, status={}\n\
                Second swap: request_id={}, status={}\n\
                This indicates a race condition that allows double spending!",
                reply1.request_id, reply1.status,
                reply2.request_id, reply2.status
            );
        }
        (Err(error1), Err(error2)) => {
            panic!(
                "Both swaps failed unexpectedly.\n\
                First error: {}\n\
                Second error: {}",
                error1, error2
            );
        }
    }
    
    // Additional check: Verify user's Token B balance only increased once
    let user_balance_b = get_icrc1_balance(&ic, ledger_id_b, user_account);
    
    // The user should have received Token B from exactly one swap
    // (We can't check the exact amount without knowing the pool state, but it should be > 0)
    assert!(
        user_balance_b > Nat::from(0u64),
        "User should have received some Token B from the successful swap"
    );
    
    println!("✅ Duplicate block ID protection test passed!");
}

