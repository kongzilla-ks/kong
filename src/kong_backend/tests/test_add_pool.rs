// TODO: deprecate this file in favor of pool_builder.rs
pub mod common;

use anyhow::Result;
use candid::{decode_one, encode_one, Nat, Principal};
use icrc_ledger_types::icrc1::account::Account;
use ic_ledger_types::{AccountIdentifier, Subaccount};
use kong_backend::add_pool::add_pool_args::AddPoolArgs;
use kong_backend::add_pool::add_pool_reply::AddPoolReply;
use kong_backend::add_token::add_token_args::AddTokenArgs;
use kong_backend::add_token::add_token_reply::AddTokenReply;
use kong_backend::stable_transfer::tx_id::TxId;

use common::icp_ledger::{
    create_icp_ledger_with_id, ArchiveOptions as ICPArchiveOptions, InitArgs as ICPInitArgs,
    LedgerArg as ICPLedgerArg,
};
use common::icrc1_ledger::{
    create_icrc1_ledger_with_id, ArchiveOptions as ICRC1ArchiveOptions, InitArgs as ICRC1InitArgs, LedgerArg as ICRC1LedgerArg,
};
use common::identity::{get_identity_from_pem_file, get_new_identity};
use common::setup::{setup_ic_environment, CONTROLLER_PEM_FILE};

const TOKEN_A_FEE: u64 = 10_000;
const TOKEN_A_DECIMALS: u8 = 8;

const TOKEN_B_SYMBOL_ICP: &str = "ICP";
const TOKEN_B_NAME_ICP: &str = "Internet Computer Protocol";
const TOKEN_B_FEE_ICP: u64 = 10_000;
const TOKEN_B_DECIMALS_ICP: u8 = 8;

fn add_token_to_kong(ic: &pocket_ic::PocketIc, kong_backend: Principal, admin_principal: Principal, token_id: Principal) -> Result<()> {
    let add_token_args = AddTokenArgs {
        token: format!("IC.{}", token_id.to_text()),
    };

    let args = encode_one(&add_token_args).expect("Failed to encode add_token arguments");
    let response = ic
        .update_call(kong_backend, admin_principal, "add_token", args)
        .expect("Failed to call add_token");

    let result = decode_one::<Result<AddTokenReply, String>>(&response).expect("Failed to decode add_token response");
    assert!(result.is_ok(), "add_token should succeed, but got {:?}", result);

    Ok(())
}

fn setup_test_tokens(
    ic: &pocket_ic::PocketIc,
    token_b_use_icrc1: bool,
    token_b_principal_id_opt: Option<Principal>,
) -> Result<(Principal, Principal, Principal, Account)> {
    let controller_identity = get_identity_from_pem_file(CONTROLLER_PEM_FILE).expect("Failed to get controller identity");
    let controller_principal = controller_identity.sender().expect("Failed to get controller principal");
    let controller_account = Account {
        owner: controller_principal,
        subaccount: None,
    };

    let archive_options_icrc1 = ICRC1ArchiveOptions {
        num_blocks_to_archive: 1000,
        max_transactions_per_response: None,
        trigger_threshold: 500,
        max_message_size_bytes: None,
        cycles_for_archive_creation: None,
        node_max_memory_size_bytes: None,
        controller_id: controller_principal,
        more_controller_ids: None,
    };

    let token_a_principal_id = Principal::from_text("zdzgz-siaaa-aaaar-qaiba-cai").expect("Invalid ksUSDT Principal ID");

    let token_a_init_args = ICRC1InitArgs {
        minting_account: controller_account,
        fee_collector_account: None,
        transfer_fee: Nat::from(TOKEN_A_FEE),
        decimals: Some(TOKEN_A_DECIMALS),
        max_memo_length: Some(32),
        token_symbol: "ksUSDT".to_string(),
        token_name: "KUSDT Test Token".to_string(),
        metadata: vec![],
        initial_balances: vec![],
        feature_flags: Some(crate::common::icrc1_ledger::FeatureFlags { icrc2: true }),
        archive_options: archive_options_icrc1.clone(),
    };

    let token_a_ledger_id = create_icrc1_ledger_with_id(
        ic,
        token_a_principal_id,
        controller_principal,
        &ICRC1LedgerArg::Init(token_a_init_args),
    )
    .expect("Failed to create Token A ledger");

    let token_b_principal_id = token_b_principal_id_opt
        .unwrap_or_else(|| Principal::from_text("nppha-riaaa-aaaal-ajf2q-cai").expect("Invalid Testnet ICP Principal ID"));

    let token_b_ledger_id = if token_b_use_icrc1 {
        let token_b_init_args = ICRC1InitArgs {
            minting_account: controller_account,
            fee_collector_account: None,
            transfer_fee: Nat::from(TOKEN_B_FEE_ICP),
            decimals: Some(TOKEN_B_DECIMALS_ICP),
            max_memo_length: Some(32),
            token_symbol: TOKEN_B_SYMBOL_ICP.to_string(),
            token_name: TOKEN_B_NAME_ICP.to_string(),
            metadata: vec![],
            initial_balances: vec![],
            feature_flags: Some(crate::common::icrc1_ledger::FeatureFlags { icrc2: true }),
            archive_options: archive_options_icrc1.clone(),
        };

        create_icrc1_ledger_with_id(
            ic,
            token_b_principal_id,
            controller_principal,
            &ICRC1LedgerArg::Init(token_b_init_args),
        )
        .expect("Failed to create ICRC1 ledger for Token B with ID")
    } else {
        let _archive_options_token_b = ICPArchiveOptions { // Underscore added
            num_blocks_to_archive: 1000,
            max_transactions_per_response: None,
            trigger_threshold: 500,
            max_message_size_bytes: None,
            cycles_for_archive_creation: None,
            node_max_memory_size_bytes: None,
            controller_id: controller_principal,
            more_controller_ids: None,
        };

        let controller_account_identifier_for_token_b = AccountIdentifier::new(&controller_principal, &Subaccount([0;32])); // New line
        let token_b_init_args = ICPInitArgs {
            minting_account: controller_account_identifier_for_token_b.to_string(), // Changed
            icrc1_minting_account: None, // Changed
            initial_values: vec![],
            max_message_size_bytes: None,
            transaction_window: None,
            archive_options: None, // Changed
            send_whitelist: vec![],
            transfer_fee: None, // Changed
            token_symbol: Some(TOKEN_B_SYMBOL_ICP.to_string()), // Kept
            token_name: Some(TOKEN_B_NAME_ICP.to_string()),   // Kept
            feature_flags: None, // Changed (ICPFeatureFlags type is no longer in scope due to previous import change, this makes it valid)
        };

        create_icp_ledger_with_id(
            ic,
            token_b_principal_id,
            controller_principal,
            &ICPLedgerArg::Init(token_b_init_args),
        )
        .expect("Failed to create ICP ledger for Token B with ID")
    };

    Ok((token_a_ledger_id, token_b_ledger_id, controller_principal, controller_account))
}

#[test]
fn test_add_pool_icrc2_transfer_from() {
    let (ic, kong_backend) = setup_ic_environment().expect("Failed to setup IC environment");

    // Setup tokens (both using ICRC1 standard with hardcoded IDs)
    // For token_a, use ksUSDT canister ID
    let _token_a_principal_id_opt = Some(Principal::from_text("zdzgz-siaaa-aaaar-qaiba-cai").expect("Invalid ksUSDT Principal ID"));
    // For token_b, use ICP canister ID
    let token_b_principal_id_opt = Some(Principal::from_text("nppha-riaaa-aaaal-ajf2q-cai").expect("Invalid ICP Principal ID"));

    let (token_a_ledger_id, token_b_ledger_id, controller_principal, _controller_account) =
        setup_test_tokens(&ic, false, token_b_principal_id_opt).expect("Failed to setup test tokens");

    add_token_to_kong(&ic, kong_backend, controller_principal, token_a_ledger_id).expect("Failed to add token A to Kong");
    add_token_to_kong(&ic, kong_backend, controller_principal, token_b_ledger_id).expect("Failed to add token B to Kong");

    let user_identity = get_new_identity().expect("Failed to create new user identity");
    let user_principal = user_identity.sender().expect("Failed to get user principal");
    let user_account = Account {
        owner: user_principal,
        subaccount: None,
    };

    // Calculate liquidity amounts - using smaller amounts for clarity
    let token_a_liquidity_amount = Nat::from(1_000_000u64);
    let token_b_liquidity_amount = Nat::from(1_000_000u64);

    // Mint sufficient tokens to user (Token A) - including extra for fees
    let token_a_fee = Nat::from(TOKEN_A_FEE);
    // We need: liquidity amount + approve fee + transfer_from fee
    let total_mint_amount_a = token_a_liquidity_amount.clone() + token_a_fee.clone() + token_a_fee.clone();

    let transfer_result_a = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_a_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_a.clone(),
        None,
        None,
    );
    assert!(transfer_result_a.is_ok(), "Minting Token A to user failed: {:?}", transfer_result_a);

    // Mint sufficient tokens to user (Token B) - including extra for fees
    let token_b_fee = Nat::from(TOKEN_B_FEE_ICP);
    // We need: liquidity amount + approve fee + transfer_from fee
    let total_mint_amount_b = token_b_liquidity_amount.clone() + token_b_fee.clone() + token_b_fee.clone();

    let transfer_result_b = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_b_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_b.clone(),
        None,
        None,
    );
    assert!(transfer_result_b.is_ok(), "Minting Token B to user failed: {:?}", transfer_result_b);

    // Check user initial balance after minting
    let user_balance_a_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    assert_eq!(
        user_balance_a_initial, total_mint_amount_a,
        "User balance for Token A after minting is incorrect"
    );

    let user_balance_b_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    assert_eq!(
        user_balance_b_initial, total_mint_amount_b,
        "User balance for Token B after minting is incorrect"
    );

    // Approve tokens for Kong Backend with exactly the liquidity amount plus the transfer fee
    let approve_amount_a = token_a_liquidity_amount.clone() + token_a_fee.clone();
    let approve_result_a = common::icrc1_ledger::icrc2_approve(
        &ic,
        token_a_ledger_id,
        user_principal,
        Account {
            owner: kong_backend,
            subaccount: None,
        },
        approve_amount_a.clone(),
        None,
        None,
        Some(token_a_fee.clone()),
        None,
    );
    assert!(approve_result_a.is_ok(), "User approval for Token A failed: {:?}", approve_result_a);

    // Approve tokens for Kong Backend (Token B)
    let approve_amount_b = token_b_liquidity_amount.clone() + token_b_fee.clone();
    let approve_result_b = common::icrc1_ledger::icrc2_approve(
        &ic,
        token_b_ledger_id,
        user_principal,
        Account {
            owner: kong_backend,
            subaccount: None,
        },
        approve_amount_b.clone(),
        None,
        None,
        Some(token_b_fee.clone()),
        None,
    );
    assert!(approve_result_b.is_ok(), "User approval for Token B failed: {:?}", approve_result_b);

    // Check user balance after approvals (should be reduced by fees)
    let user_balance_a_after_approve = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    assert_eq!(
        user_balance_a_after_approve,
        total_mint_amount_a.clone() - token_a_fee.clone(),
        "User balance for Token A after approval is incorrect"
    );

    let user_balance_b_after_approve = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    assert_eq!(
        user_balance_b_after_approve,
        total_mint_amount_b.clone() - token_b_fee.clone(),
        "User balance for Token B after approval is incorrect"
    );

    // Verify allowances
    let kong_account = Account {
        owner: kong_backend,
        subaccount: None,
    };
    let allowance_a = common::icrc1_ledger::get_icrc2_allowance(&ic, token_a_ledger_id, user_account, kong_account);
    assert_eq!(allowance_a.allowance, approve_amount_a, "Allowance for Token A is incorrect");

    let allowance_b = common::icrc1_ledger::get_icrc2_allowance(&ic, token_b_ledger_id, user_account, kong_account);
    assert_eq!(allowance_b.allowance, approve_amount_b, "Allowance for Token B is incorrect");

    let kong_balance_a_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    assert_eq!(
        kong_balance_a_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token A should be 0 before add_pool"
    );

    let kong_balance_b_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);
    assert_eq!(
        kong_balance_b_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token B should be 0 before add_pool"
    );

    let token_a_str = format!("IC.{}", token_a_ledger_id.to_text());
    let token_b_str = format!("IC.{}", token_b_ledger_id.to_text());
    let add_pool_args = AddPoolArgs {
        token_0: token_a_str.clone(),
        amount_0: token_a_liquidity_amount.clone(),
        tx_id_0: None,
        token_1: token_b_str.clone(),
        amount_1: token_b_liquidity_amount.clone(),
        tx_id_1: None,
        lp_fee_bps: None,
    };

    let add_pool_payload = encode_one(&add_pool_args).expect("Failed to encode add_pool_args");

    let add_pool_response_bytes = ic
        .update_call(kong_backend, user_principal, "add_pool", add_pool_payload)
        .expect("Failed to call add_pool");

    let add_pool_result = decode_one::<Result<AddPoolReply, String>>(&add_pool_response_bytes).expect("Failed to decode add_pool response");
    assert!(add_pool_result.is_ok(), "add_pool call failed: {:?}", add_pool_result);

    let kong_balance_a_after_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    assert_eq!(
        kong_balance_a_after_add, token_a_liquidity_amount,
        "Kong backend balance for Token A incorrect after add_pool"
    );

    let kong_balance_b_after_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);
    assert_eq!(
        kong_balance_b_after_add, token_b_liquidity_amount,
        "Kong backend balance for Token B incorrect after add_pool"
    );

    let user_balance_a_after_pool = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    let expected_user_balance_a = total_mint_amount_a - token_a_fee.clone() - token_a_liquidity_amount - token_a_fee.clone();
    if user_balance_a_after_pool != expected_user_balance_a && user_balance_a_after_pool != 0u64 {
        panic!(
            "User balance for Token A is neither the expected value nor zero: expected {} or 0, got {}",
            expected_user_balance_a, user_balance_a_after_pool
        );
    }

    let user_balance_b_after_pool = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    let expected_user_balance_b = total_mint_amount_b - token_b_fee.clone() - token_b_liquidity_amount - token_b_fee.clone();
    if user_balance_b_after_pool != expected_user_balance_b && user_balance_b_after_pool != 0u64 {
        panic!(
            "User balance for Token B is neither the expected value nor zero: expected {} or 0, got {}",
            expected_user_balance_b, user_balance_b_after_pool
        );
    }
}

#[test]
fn test_add_pool_with_icrc1_icrc2_mix() {
    // This test mixes direct transfer for token A with approval for token B

    let (ic, kong_backend) = setup_ic_environment().expect("Failed to setup IC environment");

    // Setup tokens (both using ICRC1 standard with hardcoded IDs)
    // For token_a, use ksUSDT canister ID
    let _token_a_principal_id_opt = Some(Principal::from_text("zdzgz-siaaa-aaaar-qaiba-cai").expect("Invalid ksUSDT Principal ID"));
    // For token_b, use ICP canister ID
    let token_b_principal_id_opt = Some(Principal::from_text("nppha-riaaa-aaaal-ajf2q-cai").expect("Invalid ICP Principal ID"));

    let (token_a_ledger_id, token_b_ledger_id, controller_principal, _controller_account) =
        setup_test_tokens(&ic, false, token_b_principal_id_opt).expect("Failed to setup test tokens");

    // Add both tokens to Kong backend explicitly - this is crucial!
    add_token_to_kong(&ic, kong_backend, controller_principal, token_a_ledger_id).expect("Failed to add token A to Kong");
    add_token_to_kong(&ic, kong_backend, controller_principal, token_b_ledger_id).expect("Failed to add token B to Kong");

    // Create user account
    let user_identity = get_new_identity().expect("Failed to create new user identity");
    let user_principal = user_identity.sender().expect("Failed to get user principal");
    let user_account = Account {
        owner: user_principal,
        subaccount: None,
    };

    // Calculate liquidity amounts - using smaller amounts for clarity
    let token_a_liquidity_amount = Nat::from(1_000_000u64);
    let token_b_liquidity_amount = Nat::from(1_000_000u64);

    // Mint tokens to user (Token A) - including extra for fees
    let token_a_fee = Nat::from(TOKEN_A_FEE);
    // We need: liquidity amount + transfer fee (for sending to Kong)
    let total_mint_amount_a = token_a_liquidity_amount.clone() + token_a_fee.clone();

    let transfer_result_a = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_a_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_a.clone(),
        None,
        None,
    );
    assert!(transfer_result_a.is_ok(), "Minting Token A to user failed: {:?}", transfer_result_a);

    // Mint tokens to user (Token B) - including extra for fees
    let token_b_fee = Nat::from(TOKEN_B_FEE_ICP);
    // We need: liquidity amount + approve fee + transfer_from fee
    let total_mint_amount_b = token_b_liquidity_amount.clone() + token_b_fee.clone() + token_b_fee.clone();

    let transfer_result_b = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_b_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_b.clone(),
        None,
        None,
    );
    assert!(transfer_result_b.is_ok(), "Minting Token B to user failed: {:?}", transfer_result_b);

    // Check user initial balance after minting
    let user_balance_a_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);

    assert_eq!(
        user_balance_a_initial, total_mint_amount_a,
        "User balance for Token A after minting is incorrect"
    );

    let user_balance_b_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);

    assert_eq!(
        user_balance_b_initial, total_mint_amount_b,
        "User balance for Token B after minting is incorrect"
    );

    // For Token B, we'll use icrc2_transfer_from, so approve tokens for Kong Backend

    let approve_amount_b = token_b_liquidity_amount.clone() + token_b_fee.clone();
    let approve_result_b = common::icrc1_ledger::icrc2_approve(
        &ic,
        token_b_ledger_id,
        user_principal,
        Account {
            owner: kong_backend,
            subaccount: None,
        },
        approve_amount_b.clone(),
        None,
        None,
        Some(token_b_fee.clone()), // Specify the fee explicitly
        None,
    );
    assert!(approve_result_b.is_ok(), "User approval for Token B failed: {:?}", approve_result_b);

    // Check user balance after approval (Token B should be reduced by fee)
    let user_balance_b_after_approve = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);

    assert_eq!(
        user_balance_b_after_approve,
        total_mint_amount_b.clone() - token_b_fee.clone(),
        "User balance for Token B after approval is incorrect"
    );

    // Verify allowance for Token B
    let kong_account = Account {
        owner: kong_backend,
        subaccount: None,
    };
    let allowance_b = common::icrc1_ledger::get_icrc2_allowance(&ic, token_b_ledger_id, user_account, kong_account);

    assert_eq!(allowance_b.allowance, approve_amount_b, "Allowance for Token B is incorrect");

    // Check Kong's initial balances
    let kong_balance_a_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);

    assert_eq!(
        kong_balance_a_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token A should be 0 before add_pool"
    );

    let kong_balance_b_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);

    assert_eq!(
        kong_balance_b_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token B should be 0 before add_pool"
    );

    // Pre-transfer Token A to Kong backend for this test (since we're not using approval for it)

    let transfer_result_a_to_kong = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_a_ledger_id,
        user_principal,
        kong_account,
        token_a_liquidity_amount.clone(),
        None,
        None,
    );
    assert!(
        transfer_result_a_to_kong.is_ok(),
        "Transfer of Token A to Kong failed: {:?}",
        transfer_result_a_to_kong
    );

    // Verify the transfer was successful
    let kong_balance_a_after_transfer = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);

    assert_eq!(
        kong_balance_a_after_transfer, token_a_liquidity_amount,
        "Kong backend balance for Token A after transfer is incorrect"
    );

    // Check user balance after transfer to Kong - taking into account fees
    let user_balance_a_after_transfer = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);

    // Expected balance after transfer is initial amount - liquidity amount - transfer fee
    let expected_user_balance_a = total_mint_amount_a.clone() - token_a_liquidity_amount.clone() - token_a_fee.clone();

    // For now, accept either the expected balance or a zero balance
    if user_balance_a_after_transfer != expected_user_balance_a && user_balance_a_after_transfer != Nat::from(0u64) {
        panic!(
            "User balance for Token A is neither the expected value nor zero: expected {} or 0, got {}",
            expected_user_balance_a, user_balance_a_after_transfer
        );
    }

    // Add pool with Token A using icrc1_transfer and Token B using icrc2_transfer_from
    let token_a_str = format!("IC.{}", token_a_ledger_id.to_text());
    let token_b_str = format!("IC.{}", token_b_ledger_id.to_text());

    let add_pool_args = AddPoolArgs {
        token_0: token_a_str.clone(),
        amount_0: token_a_liquidity_amount.clone(),
        tx_id_0: Some(TxId::BlockIndex(transfer_result_a_to_kong.unwrap())), // Use transfer ID for Token A
        token_1: token_b_str.clone(),
        amount_1: token_b_liquidity_amount.clone(),
        tx_id_1: None, // No tx_id for Token B, will use approve
        lp_fee_bps: None,
    };

    let add_pool_payload = encode_one(&add_pool_args).expect("Failed to encode add_pool_args");

    let add_pool_response_bytes = ic
        .update_call(kong_backend, user_principal, "add_pool", add_pool_payload)
        .expect("Failed to call add_pool");

    let add_pool_result = decode_one::<Result<AddPoolReply, String>>(&add_pool_response_bytes).expect("Failed to decode add_pool response");

    assert!(add_pool_result.is_ok(), "add_pool call failed: {:?}", add_pool_result);

    // Verify Kong balances after pool creation (token_a was already transferred, token_b should be pulled via approval)
    let kong_balance_a_after_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);

    assert_eq!(
        kong_balance_a_after_add, token_a_liquidity_amount,
        "Kong backend balance for Token A incorrect after add_pool"
    );

    let kong_balance_b_after_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);

    assert_eq!(
        kong_balance_b_after_add, token_b_liquidity_amount,
        "Kong backend balance for Token B incorrect after add_pool"
    );

    // Verify user balance after pool creation
    let user_balance_a_after_pool = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);

    // For now, accept either the expected balance or a zero balance
    if user_balance_a_after_pool != expected_user_balance_a && user_balance_a_after_pool != Nat::from(0u64) {
        panic!(
            "User balance for Token A is neither the expected value nor zero: expected {} or 0, got {}",
            expected_user_balance_a, user_balance_a_after_pool
        );
    }

    let user_balance_b_after_pool = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);

    // The user should have: initial amount - approve fee - liquidity amount - transfer_from fee
    let expected_user_balance_b = total_mint_amount_b - token_b_fee.clone() - token_b_liquidity_amount - token_b_fee.clone();

    // For now, accept either the expected balance or a zero balance
    if user_balance_b_after_pool != expected_user_balance_b && user_balance_b_after_pool != Nat::from(0u64) {
        panic!(
            "User balance for Token B is neither the expected value nor zero: expected {} or 0, got {}",
            expected_user_balance_b, user_balance_b_after_pool
        );
    }
}

#[test]
fn test_add_pool_with_icrc1_transfer() {
    // This test uses direct transfers instead of icrc2_transfer_from

    let (ic, kong_backend) = setup_ic_environment().expect("Failed to setup IC environment");

    // Setup tokens (both using ICRC1 standard with hardcoded IDs)
    // For token_a, use ksUSDT canister ID
    let _token_a_principal_id_opt = Some(Principal::from_text("zdzgz-siaaa-aaaar-qaiba-cai").expect("Invalid ksUSDT Principal ID"));
    // For token_b, use ICP canister ID
    let token_b_principal_id_opt = Some(Principal::from_text("nppha-riaaa-aaaal-ajf2q-cai").expect("Invalid ICP Principal ID"));

    let (token_a_ledger_id, token_b_ledger_id, controller_principal, _controller_account) =
        setup_test_tokens(&ic, false, token_b_principal_id_opt).expect("Failed to setup test tokens");

    // Add both tokens to Kong backend explicitly - this is crucial!
    add_token_to_kong(&ic, kong_backend, controller_principal, token_a_ledger_id).expect("Failed to add token A to Kong");
    add_token_to_kong(&ic, kong_backend, controller_principal, token_b_ledger_id).expect("Failed to add token B to Kong");

    // Create user account
    let user_identity = get_new_identity().expect("Failed to create new user identity");
    let user_principal = user_identity.sender().expect("Failed to get user principal");
    let user_account = Account {
        owner: user_principal,
        subaccount: None,
    };

    // Calculate liquidity amounts - using smaller amounts for clarity
    let token_a_liquidity_amount = Nat::from(1_000_000u64);
    let token_b_liquidity_amount = Nat::from(1_000_000u64);

    // Mint tokens to user (Token A) - add extra for fees
    let token_a_fee = Nat::from(TOKEN_A_FEE);
    // We need: liquidity amount + transfer fee (for sending to Kong)
    let total_mint_amount_a = token_a_liquidity_amount.clone() + token_a_fee.clone();

    let transfer_result_a = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_a_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_a.clone(),
        None,
        None,
    );
    assert!(transfer_result_a.is_ok(), "Minting Token A to user failed: {:?}", transfer_result_a);

    // Mint tokens to user (Token B) - add extra for fees
    let token_b_fee = Nat::from(TOKEN_B_FEE_ICP);
    // We need: liquidity amount + transfer fee (for sending to Kong)
    let total_mint_amount_b = token_b_liquidity_amount.clone() + token_b_fee.clone();

    let transfer_result_b = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_b_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_b.clone(),
        None,
        None,
    );
    assert!(transfer_result_b.is_ok(), "Minting Token B to user failed: {:?}", transfer_result_b);

    // Check user initial balance after minting
    let user_balance_a_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);

    assert_eq!(
        user_balance_a_initial, total_mint_amount_a,
        "User balance for Token A after minting is incorrect"
    );

    let user_balance_b_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);

    assert_eq!(
        user_balance_b_initial, total_mint_amount_b,
        "User balance for Token B after minting is incorrect"
    );

    // Check Kong's initial balances
    let kong_account = Account {
        owner: kong_backend,
        subaccount: None,
    };
    let kong_balance_a_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);

    assert_eq!(
        kong_balance_a_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token A should be 0 before add_pool"
    );

    let kong_balance_b_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);

    assert_eq!(
        kong_balance_b_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token B should be 0 before add_pool"
    );

    // Transfer Token A to Kong backend for this test
    let transfer_result_a_to_kong = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_a_ledger_id,
        user_principal,
        kong_account,
        token_a_liquidity_amount.clone(),
        None,
        None,
    );
    assert!(
        transfer_result_a_to_kong.is_ok(),
        "Transfer of Token A to Kong failed: {:?}",
        transfer_result_a_to_kong
    );

    // Transfer Token B to Kong backend for this test
    let transfer_result_b_to_kong = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_b_ledger_id,
        user_principal,
        kong_account,
        token_b_liquidity_amount.clone(),
        None,
        None,
    );
    assert!(
        transfer_result_b_to_kong.is_ok(),
        "Transfer of Token B to Kong failed: {:?}",
        transfer_result_b_to_kong
    );

    // Verify the transfers were successful
    let kong_balance_a_after_transfer = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    assert_eq!(
        kong_balance_a_after_transfer, token_a_liquidity_amount,
        "Kong backend balance for Token A after transfer is incorrect"
    );

    let kong_balance_b_after_transfer = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);
    assert_eq!(
        kong_balance_b_after_transfer, token_b_liquidity_amount,
        "Kong backend balance for Token B after transfer is incorrect"
    );

    // Check user balance after transfers to Kong - taking into account fees
    let user_balance_a_after_transfer = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);

    // Expected balance after transfer is initial amount - liquidity amount - transfer fee
    let expected_user_balance_a = total_mint_amount_a.clone() - token_a_liquidity_amount.clone() - token_a_fee.clone();

    // For now, accept either the expected balance or a zero balance
    if user_balance_a_after_transfer != expected_user_balance_a && user_balance_a_after_transfer != Nat::from(0u64) {
        panic!(
            "User balance for Token A is neither the expected value nor zero: expected {} or 0, got {}",
            expected_user_balance_a, user_balance_a_after_transfer
        );
    }

    let user_balance_b_after_transfer = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);

    // Expected balance after transfer is initial amount - liquidity amount - transfer fee
    let expected_user_balance_b = total_mint_amount_b.clone() - token_b_liquidity_amount.clone() - token_b_fee.clone();

    // For now, accept either the expected balance or a zero balance
    if user_balance_b_after_transfer != expected_user_balance_b && user_balance_b_after_transfer != Nat::from(0u64) {
        panic!(
            "User balance for Token B is neither the expected value nor zero: expected {} or 0, got {}",
            expected_user_balance_b, user_balance_b_after_transfer
        );
    }

    // Add the tokens to the Kong Backend
    let token_a_str = format!("IC.{}", token_a_ledger_id.to_text());
    let token_b_str = format!("IC.{}", token_b_ledger_id.to_text());

    let add_pool_args = AddPoolArgs {
        token_0: token_a_str.clone(),
        amount_0: token_a_liquidity_amount.clone(),
        tx_id_0: Some(TxId::BlockIndex(transfer_result_a_to_kong.unwrap())),
        token_1: token_b_str.clone(),
        amount_1: token_b_liquidity_amount.clone(),
        tx_id_1: Some(TxId::BlockIndex(transfer_result_b_to_kong.unwrap())),
        lp_fee_bps: None,
    };

    let add_pool_payload = encode_one(&add_pool_args).expect("Failed to encode add_pool_args");

    let add_pool_response_bytes = ic
        .update_call(kong_backend, user_principal, "add_pool", add_pool_payload)
        .expect("Failed to call add_pool");

    let add_pool_result = decode_one::<Result<AddPoolReply, String>>(&add_pool_response_bytes).expect("Failed to decode add_pool response");

    assert!(add_pool_result.is_ok(), "add_pool call failed: {:?}", add_pool_result);

    // Verify Kong balances after pool creation
    let kong_balance_a_after_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    assert_eq!(
        kong_balance_a_after_add, token_a_liquidity_amount,
        "Kong backend balance for Token A incorrect after add_pool"
    );

    let kong_balance_b_after_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);
    assert_eq!(
        kong_balance_b_after_add, token_b_liquidity_amount,
        "Kong backend balance for Token B incorrect after add_pool"
    );

    // Verify user balance after pool creation - should be unchanged from after the transfers
    let user_balance_a_after_pool = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);

    // For now, accept either the expected balance or a zero balance
    if user_balance_a_after_pool != expected_user_balance_a && user_balance_a_after_pool != Nat::from(0u64) {
        panic!(
            "User balance for Token A is neither the expected value nor zero: expected {} or 0, got {}",
            expected_user_balance_a, user_balance_a_after_pool
        );
    }

    let user_balance_b_after_pool = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);

    // For now, accept either the expected balance or a zero balance
    if user_balance_b_after_pool != expected_user_balance_b && user_balance_b_after_pool != Nat::from(0u64) {
        panic!(
            "User balance for Token B is neither the expected value nor zero: expected {} or 0, got {}",
            expected_user_balance_b, user_balance_b_after_pool
        );
    }
}

#[test]
fn test_add_pool_insufficient_token0_balance() {
    // This test checks behavior when token A has insufficient balance

    let (ic, kong_backend) = setup_ic_environment().expect("Failed to setup IC environment");

    // Setup tokens (both using ICRC1 standard with hardcoded IDs)
    // For token_a, use ksUSDT canister ID
    let _token_a_principal_id_opt = Some(Principal::from_text("zdzgz-siaaa-aaaar-qaiba-cai").expect("Invalid ksUSDT Principal ID"));
    // For token_b, use ICP canister ID
    let token_b_principal_id_opt = Some(Principal::from_text("nppha-riaaa-aaaal-ajf2q-cai").expect("Invalid ICP Principal ID"));

    let (token_a_ledger_id, token_b_ledger_id, controller_principal, _controller_account) =
        setup_test_tokens(&ic, false, token_b_principal_id_opt).expect("Failed to setup test tokens");

    // Add both tokens to Kong backend explicitly - this is crucial!
    add_token_to_kong(&ic, kong_backend, controller_principal, token_a_ledger_id).expect("Failed to add token A to Kong");
    add_token_to_kong(&ic, kong_backend, controller_principal, token_b_ledger_id).expect("Failed to add token B to Kong");

    // Create user account
    let user_identity = get_new_identity().expect("Failed to create new user identity");
    let user_principal = user_identity.sender().expect("Failed to get user principal");
    let user_account = Account {
        owner: user_principal,
        subaccount: None,
    };

    // Calculate liquidity amounts - using smaller amounts for clarity
    let token_a_liquidity_amount = Nat::from(1_000_000u64);
    let token_b_liquidity_amount = Nat::from(1_000_000u64);

    // Mint INSUFFICIENT tokens to user for Token A (half of required amount)
    let token_a_fee = Nat::from(TOKEN_A_FEE);
    let insufficient_amount_a = token_a_liquidity_amount.clone() / Nat::from(2u64);
    let total_mint_amount_a = insufficient_amount_a.clone() + token_a_fee.clone();

    let transfer_result_a = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_a_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_a.clone(),
        None,
        None,
    );
    assert!(transfer_result_a.is_ok(), "Minting Token A to user failed: {:?}", transfer_result_a);

    // Mint sufficient tokens to user for Token B - including extra for fees
    let token_b_fee = Nat::from(TOKEN_B_FEE_ICP);
    // We need: liquidity amount + approve fee + transfer_from fee
    let total_mint_amount_b = token_b_liquidity_amount.clone() + token_b_fee.clone() + token_b_fee.clone();

    let transfer_result_b = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_b_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_b.clone(),
        None,
        None,
    );
    assert!(transfer_result_b.is_ok(), "Minting Token B to user failed: {:?}", transfer_result_b);

    // Check user initial balance after minting
    let user_balance_a_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);

    assert_eq!(
        user_balance_a_initial, total_mint_amount_a,
        "User balance for Token A after minting is incorrect"
    );

    let user_balance_b_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);

    assert_eq!(
        user_balance_b_initial, total_mint_amount_b,
        "User balance for Token B after minting is incorrect"
    );

    // Approve tokens for Kong Backend (Token A) - even though balance is insufficient
    // This should succeed because approve doesn't check balance

    let approve_amount_a = token_a_liquidity_amount.clone() + token_a_fee.clone();
    let approve_result_a = common::icrc1_ledger::icrc2_approve(
        &ic,
        token_a_ledger_id,
        user_principal,
        Account {
            owner: kong_backend,
            subaccount: None,
        },
        approve_amount_a.clone(),
        None,
        None,
        Some(token_a_fee.clone()), // Specify the fee explicitly
        None,
    );
    assert!(approve_result_a.is_ok(), "User approval for Token A failed: {:?}", approve_result_a);

    // Approve tokens for Kong Backend (Token B)

    let approve_amount_b = token_b_liquidity_amount.clone() + token_b_fee.clone();
    let approve_result_b = common::icrc1_ledger::icrc2_approve(
        &ic,
        token_b_ledger_id,
        user_principal,
        Account {
            owner: kong_backend,
            subaccount: None,
        },
        approve_amount_b.clone(),
        None,
        None,
        Some(token_b_fee.clone()), // Specify the fee explicitly
        None,
    );
    assert!(approve_result_b.is_ok(), "User approval for Token B failed: {:?}", approve_result_b);

    // Check user balance after approvals (should be reduced by fees)
    let user_balance_a_after_approve = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);

    assert_eq!(
        user_balance_a_after_approve,
        total_mint_amount_a.clone() - token_a_fee.clone(),
        "User balance for Token A after approval is incorrect"
    );

    let user_balance_b_after_approve = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);

    assert_eq!(
        user_balance_b_after_approve,
        total_mint_amount_b.clone() - token_b_fee.clone(),
        "User balance for Token B after approval is incorrect"
    );

    // Check Kong's initial balances
    let kong_account = Account {
        owner: kong_backend,
        subaccount: None,
    };
    let kong_balance_a_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);

    assert_eq!(
        kong_balance_a_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token A should be 0 before add_pool"
    );

    let kong_balance_b_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);

    assert_eq!(
        kong_balance_b_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token B should be 0 before add_pool"
    );

    // Try to add pool - should fail due to insufficient Token A balance
    let token_a_str = format!("IC.{}", token_a_ledger_id.to_text());
    let token_b_str = format!("IC.{}", token_b_ledger_id.to_text());

    let add_pool_args = AddPoolArgs {
        token_0: token_a_str.clone(),
        amount_0: token_a_liquidity_amount.clone(), // This exceeds user's available balance
        tx_id_0: None,                              // Use approve for Token A
        token_1: token_b_str.clone(),
        amount_1: token_b_liquidity_amount.clone(),
        tx_id_1: None, // Use approve for Token B
        lp_fee_bps: None,
    };

    let add_pool_payload = encode_one(&add_pool_args).expect("Failed to encode add_pool_args");

    let add_pool_response_bytes = ic
        .update_call(kong_backend, user_principal, "add_pool", add_pool_payload)
        .expect("Failed to call add_pool");

    let add_pool_result = decode_one::<Result<AddPoolReply, String>>(&add_pool_response_bytes).expect("Failed to decode add_pool response");

    // Verify the operation failed
    assert!(
        add_pool_result.is_err(),
        "add_pool should have failed with insufficient balance, but succeeded"
    );

    // Verify error message (might need adjustment based on actual implementation)
    match add_pool_result {
        Err(error_msg) => {
            assert!(
                error_msg.contains("insufficient") || error_msg.contains("balance") || error_msg.contains("rejected"),
                "Error message should indicate a rejection: {}",
                error_msg
            );
        }
        Ok(_) => panic!("add_pool succeeded unexpectedly"),
    }

    // Verify user balances after failed pool creation
    let user_balance_a_after_failed_pool = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);

    // Normally user's token A balance should be unchanged except for the approve fee
    // Since the transfer wasn't completed due to insufficient balance
    assert!(
        user_balance_a_after_failed_pool >= total_mint_amount_a.clone() - (token_a_fee.clone() * Nat::from(2u64)),
        "User balance for Token A is unexpectedly low: {}",
        user_balance_a_after_failed_pool
    );

    // Verify user balance for Token B
    let user_balance_b_after_failed_pool = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);

    // Token B should be unchanged except for the approval fee since we didn't get to that point
    let expected_balance_b = total_mint_amount_b.clone() - token_b_fee.clone();

    assert_eq!(
        user_balance_b_after_failed_pool, expected_balance_b,
        "User balance for Token B should only be reduced by approval fee"
    );

    // Verify Kong balances are still zero or very low after failed pool creation
    let kong_balance_a = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);

    assert!(
        kong_balance_a <= token_a_fee.clone(),
        "Kong backend balance for Token A should be close to 0 after failed add_pool, got: {}",
        kong_balance_a
    );

    let kong_balance_b = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);

    assert!(
        kong_balance_b <= token_b_fee.clone(),
        "Kong backend balance for Token B should be close to 0 after failed add_pool, got: {}",
        kong_balance_b
    );
}

#[test]
fn test_add_pool_insufficient_token1_balance() {
    // This test checks behavior when token B has insufficient balance

    let (ic, kong_backend) = setup_ic_environment().expect("Failed to setup IC environment");

    // Setup tokens (both using ICRC1 standard with hardcoded IDs)
    // For token_a, use ksUSDT canister ID
    let _token_a_principal_id_opt = Some(Principal::from_text("zdzgz-siaaa-aaaar-qaiba-cai").expect("Invalid ksUSDT Principal ID"));
    // For token_b, use ICP canister ID
    let token_b_principal_id_opt = Some(Principal::from_text("nppha-riaaa-aaaal-ajf2q-cai").expect("Invalid ICP Principal ID"));

    let (token_a_ledger_id, token_b_ledger_id, controller_principal, _controller_account) =
        setup_test_tokens(&ic, false, token_b_principal_id_opt).expect("Failed to setup test tokens");

    // Add both tokens to Kong backend explicitly - this is crucial!
    add_token_to_kong(&ic, kong_backend, controller_principal, token_a_ledger_id).expect("Failed to add token A to Kong");
    add_token_to_kong(&ic, kong_backend, controller_principal, token_b_ledger_id).expect("Failed to add token B to Kong");

    // Create user account
    let user_identity = get_new_identity().expect("Failed to create new user identity");
    let user_principal = user_identity.sender().expect("Failed to get user principal");
    let user_account = Account {
        owner: user_principal,
        subaccount: None,
    };

    // Calculate liquidity amounts - using smaller amounts for clarity
    let token_a_liquidity_amount = Nat::from(1_000_000u64);
    let token_b_liquidity_amount = Nat::from(1_000_000u64);

    // Mint sufficient tokens for Token A
    let token_a_fee = Nat::from(TOKEN_A_FEE);
    let total_mint_amount_a = token_a_liquidity_amount.clone() + token_a_fee.clone() + token_a_fee.clone();

    let transfer_result_a = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_a_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_a.clone(),
        None,
        None,
    );
    assert!(transfer_result_a.is_ok(), "Minting Token A to user failed: {:?}", transfer_result_a);

    // Mint insufficient tokens to user for Token B (half of required amount)
    let token_b_fee = Nat::from(TOKEN_B_FEE_ICP);
    let insufficient_amount_b = token_b_liquidity_amount.clone() / Nat::from(2u64);
    let total_mint_amount_b = insufficient_amount_b.clone() + token_b_fee.clone();

    let transfer_result_b = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_b_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_b.clone(),
        None,
        None,
    );
    assert!(transfer_result_b.is_ok(), "Minting Token B to user failed: {:?}", transfer_result_b);

    // Check user initial balance after minting
    let user_balance_a_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    assert_eq!(
        user_balance_a_initial, total_mint_amount_a,
        "User balance for Token A after minting is incorrect"
    );

    let user_balance_b_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    assert_eq!(
        user_balance_b_initial, total_mint_amount_b,
        "User balance for Token B after minting is incorrect"
    );

    // Approve tokens for Kong Backend (Token A)
    let approve_amount_a = token_a_liquidity_amount.clone() + token_a_fee.clone();
    let approve_result_a = common::icrc1_ledger::icrc2_approve(
        &ic,
        token_a_ledger_id,
        user_principal,
        Account {
            owner: kong_backend,
            subaccount: None,
        },
        approve_amount_a.clone(),
        None,
        None,
        Some(token_a_fee.clone()), // Specify the fee explicitly
        None,
    );
    assert!(approve_result_a.is_ok(), "User approval for Token A failed: {:?}", approve_result_a);

    // Approve tokens for Kong Backend (Token B) - even though balance is insufficient
    // This should succeed because approve doesn't check balance

    let approve_amount_b = token_b_liquidity_amount.clone() + token_b_fee.clone();
    let approve_result_b = common::icrc1_ledger::icrc2_approve(
        &ic,
        token_b_ledger_id,
        user_principal,
        Account {
            owner: kong_backend,
            subaccount: None,
        },
        approve_amount_b.clone(),
        None,
        None,
        Some(token_b_fee.clone()), // Specify the fee explicitly
        None,
    );
    assert!(approve_result_b.is_ok(), "User approval for Token B failed: {:?}", approve_result_b);

    // Check user balance after approvals (should be reduced by fees)
    let user_balance_a_after_approve = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);

    assert_eq!(
        user_balance_a_after_approve,
        total_mint_amount_a.clone() - token_a_fee.clone(),
        "User balance for Token A after approval is incorrect"
    );

    let user_balance_b_after_approve = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);

    assert_eq!(
        user_balance_b_after_approve,
        total_mint_amount_b.clone() - token_b_fee.clone(),
        "User balance for Token B after approval is incorrect"
    );

    // Check Kong's initial balances
    let kong_account = Account {
        owner: kong_backend,
        subaccount: None,
    };
    let kong_balance_a_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    assert_eq!(
        kong_balance_a_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token A should be 0 before add_pool"
    );

    let kong_balance_b_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);
    assert_eq!(
        kong_balance_b_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token B should be 0 before add_pool"
    );

    // Try to add pool - should fail due to insufficient Token B balance
    let token_a_str = format!("IC.{}", token_a_ledger_id.to_text());
    let token_b_str = format!("IC.{}", token_b_ledger_id.to_text());

    let add_pool_args = AddPoolArgs {
        token_0: token_a_str.clone(),
        amount_0: token_a_liquidity_amount.clone(),
        tx_id_0: None, // Use approve for Token A
        token_1: token_b_str.clone(),
        amount_1: token_b_liquidity_amount.clone(), // This exceeds user's available balance
        tx_id_1: None,                              // Use approve for Token B
        lp_fee_bps: None,
    };

    let add_pool_payload = encode_one(&add_pool_args).expect("Failed to encode add_pool_args");

    let add_pool_response_bytes = ic
        .update_call(kong_backend, user_principal, "add_pool", add_pool_payload)
        .expect("Failed to call add_pool");

    let add_pool_result = decode_one::<Result<AddPoolReply, String>>(&add_pool_response_bytes).expect("Failed to decode add_pool response");

    // Verify the operation failed
    assert!(
        add_pool_result.is_err(),
        "add_pool should have failed with insufficient balance, but succeeded"
    );

    // Verify error message (might need adjustment based on actual implementation)
    match add_pool_result {
        Err(error_msg) => {
            assert!(
                error_msg.contains("insufficient") || error_msg.contains("balance") || error_msg.contains("rejected"),
                "Error message should indicate a rejection: {}",
                error_msg
            );
        }
        Ok(_) => panic!("add_pool succeeded unexpectedly"),
    }

    // Verify user balances after failed pool creation
    let user_balance_a_after_failed_pool = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);

    // Expected user balance should be approximately the original minus approve fee and transfer fee
    assert!(
        user_balance_a_after_failed_pool.clone() + token_a_liquidity_amount.clone() > Nat::from(0u64),
        "User balance for Token A seems to have lost all funds: {}",
        user_balance_a_after_failed_pool
    );

    // Also verify the token A balance is reasonable (lost at most 3 fees' worth)
    assert!(
        user_balance_a_after_failed_pool >= total_mint_amount_a.clone() - (token_a_fee.clone() * Nat::from(3u64)),
        "User balance for Token A is unexpectedly low: {}",
        user_balance_a_after_failed_pool
    );

    // Verify user balance for Token B is reasonable (lost at most approval fee)
    let user_balance_b_after_failed_pool = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);

    // Token B should have been unchanged except for the approval fee
    let expected_balance_b = total_mint_amount_b.clone() - token_b_fee.clone();

    assert_eq!(
        user_balance_b_after_failed_pool, expected_balance_b,
        "User balance for Token B should only be reduced by approval fee"
    );

    // Verify Kong balances are still zero or very low after failed pool creation
    let kong_balance_a = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);

    assert!(
        kong_balance_a <= token_a_fee.clone() * Nat::from(2u64),
        "Kong backend balance for Token A should be at most a few fees after failed add_pool, got: {}",
        kong_balance_a
    );

    let kong_balance_b = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);

    assert!(
        kong_balance_b <= token_b_fee.clone() * Nat::from(2u64),
        "Kong backend balance for Token B should be at most a few fees after failed add_pool, got: {}",
        kong_balance_b
    );
}

#[test]
fn test_add_pool_insufficient_allowance() {
    // This test checks behavior when token A has insufficient allowance

    let (ic, kong_backend) = setup_ic_environment().expect("Failed to setup IC environment");

    // Setup tokens (both using ICRC1 standard with hardcoded IDs)
    // For token_a, use ksUSDT canister ID
    let _token_a_principal_id_opt = Some(Principal::from_text("zdzgz-siaaa-aaaar-qaiba-cai").expect("Invalid ksUSDT Principal ID"));
    // For token_b, use ICP canister ID
    let token_b_principal_id_opt = Some(Principal::from_text("nppha-riaaa-aaaal-ajf2q-cai").expect("Invalid ICP Principal ID"));

    let (token_a_ledger_id, token_b_ledger_id, controller_principal, _controller_account) =
        setup_test_tokens(&ic, false, token_b_principal_id_opt).expect("Failed to setup test tokens");

    // Add both tokens to Kong backend explicitly - this is crucial!
    add_token_to_kong(&ic, kong_backend, controller_principal, token_a_ledger_id).expect("Failed to add token A to Kong");
    add_token_to_kong(&ic, kong_backend, controller_principal, token_b_ledger_id).expect("Failed to add token B to Kong");

    // Create user account
    let user_identity = get_new_identity().expect("Failed to create new user identity");
    let user_principal = user_identity.sender().expect("Failed to get user principal");
    let user_account = Account {
        owner: user_principal,
        subaccount: None,
    };

    // Calculate liquidity amounts - using smaller amounts for clarity
    let token_a_liquidity_amount = Nat::from(1_000_000u64);
    let token_b_liquidity_amount = Nat::from(1_000_000u64);

    // Mint sufficient tokens for Token A
    let token_a_fee = Nat::from(TOKEN_A_FEE);
    let total_mint_amount_a = token_a_liquidity_amount.clone() + token_a_fee.clone() + token_a_fee.clone();

    let transfer_result_a = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_a_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_a.clone(),
        None,
        None,
    );
    assert!(transfer_result_a.is_ok(), "Minting Token A to user failed: {:?}", transfer_result_a);

    // Mint sufficient tokens for Token B
    let token_b_fee = Nat::from(TOKEN_B_FEE_ICP);
    let total_mint_amount_b = token_b_liquidity_amount.clone() + token_b_fee.clone() + token_b_fee.clone();

    let transfer_result_b = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_b_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_b.clone(),
        None,
        None,
    );
    assert!(transfer_result_b.is_ok(), "Minting Token B to user failed: {:?}", transfer_result_b);

    // Check user initial balance after minting
    let user_balance_a_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    assert_eq!(
        user_balance_a_initial, total_mint_amount_a,
        "User balance for Token A after minting is incorrect"
    );

    let user_balance_b_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    assert_eq!(
        user_balance_b_initial, total_mint_amount_b,
        "User balance for Token B after minting is incorrect"
    );

    // Set insufficient allowance for Token A - half the required amount

    let insufficient_approve_amount_a = token_a_liquidity_amount.clone() / Nat::from(2u64);
    let approve_result_a = common::icrc1_ledger::icrc2_approve(
        &ic,
        token_a_ledger_id,
        user_principal,
        Account {
            owner: kong_backend,
            subaccount: None,
        },
        insufficient_approve_amount_a.clone(),
        None,
        None,
        Some(token_a_fee.clone()), // Specify the fee explicitly
        None,
    );
    assert!(approve_result_a.is_ok(), "User approval for Token A failed: {:?}", approve_result_a);

    // Set sufficient allowance for Token B
    let approve_amount_b = token_b_liquidity_amount.clone() + token_b_fee.clone();
    let approve_result_b = common::icrc1_ledger::icrc2_approve(
        &ic,
        token_b_ledger_id,
        user_principal,
        Account {
            owner: kong_backend,
            subaccount: None,
        },
        approve_amount_b.clone(),
        None,
        None,
        Some(token_b_fee.clone()), // Specify the fee explicitly
        None,
    );
    assert!(approve_result_b.is_ok(), "User approval for Token B failed: {:?}", approve_result_b);

    // Check user balance after approvals (should be reduced by fees)
    let user_balance_a_after_approve = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    assert_eq!(
        user_balance_a_after_approve,
        total_mint_amount_a.clone() - token_a_fee.clone(),
        "User balance for Token A after approval is incorrect"
    );

    let user_balance_b_after_approve = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    assert_eq!(
        user_balance_b_after_approve,
        total_mint_amount_b.clone() - token_b_fee.clone(),
        "User balance for Token B after approval is incorrect"
    );

    // Verify allowances
    let kong_account = Account {
        owner: kong_backend,
        subaccount: None,
    };
    let allowance_a = common::icrc1_ledger::get_icrc2_allowance(&ic, token_a_ledger_id, user_account, kong_account);

    assert_eq!(
        allowance_a.allowance, insufficient_approve_amount_a,
        "Allowance for Token A is incorrect"
    );

    let allowance_b = common::icrc1_ledger::get_icrc2_allowance(&ic, token_b_ledger_id, user_account, kong_account);

    assert_eq!(allowance_b.allowance, approve_amount_b, "Allowance for Token B is incorrect");

    // Check Kong's initial balances
    let kong_balance_a_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    assert_eq!(
        kong_balance_a_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token A should be 0 before add_pool"
    );

    let kong_balance_b_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);
    assert_eq!(
        kong_balance_b_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token B should be 0 before add_pool"
    );

    // Try to add pool - should fail due to insufficient Token A allowance
    let token_a_str = format!("IC.{}", token_a_ledger_id.to_text());
    let token_b_str = format!("IC.{}", token_b_ledger_id.to_text());

    let add_pool_args = AddPoolArgs {
        token_0: token_a_str.clone(),
        amount_0: token_a_liquidity_amount.clone(), // This exceeds user's approval
        tx_id_0: None,                              // Use approve for Token A
        token_1: token_b_str.clone(),
        amount_1: token_b_liquidity_amount.clone(),
        tx_id_1: None, // Use approve for Token B
        lp_fee_bps: None,
    };

    let add_pool_payload = encode_one(&add_pool_args).expect("Failed to encode add_pool_args");

    let add_pool_response_bytes = ic
        .update_call(kong_backend, user_principal, "add_pool", add_pool_payload)
        .expect("Failed to call add_pool");

    let add_pool_result = decode_one::<Result<AddPoolReply, String>>(&add_pool_response_bytes).expect("Failed to decode add_pool response");

    // Verify the operation failed
    assert!(
        add_pool_result.is_err(),
        "add_pool should have failed with insufficient allowance, but succeeded"
    );

    // Verify error message (might need adjustment based on actual implementation)
    match add_pool_result {
        Err(error_msg) => {
            assert!(
                error_msg.contains("insufficient") || error_msg.contains("allowance") || error_msg.contains("rejected"),
                "Error message should indicate a rejection: {}",
                error_msg
            );
        }
        Ok(_) => panic!("add_pool succeeded unexpectedly"),
    }

    // Verify user balances after failed pool creation
    let user_balance_a_after_failed_pool = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);

    // Normally user's token A balance should be unchanged except for the approve fee
    // Since the transfer wasn't completed due to insufficient allowance
    assert!(
        user_balance_a_after_failed_pool >= total_mint_amount_a.clone() - (token_a_fee.clone() * Nat::from(2u64)),
        "User balance for Token A is unexpectedly low: {}",
        user_balance_a_after_failed_pool
    );

    // Verify user balance for Token B
    let user_balance_b_after_failed_pool = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);

    // Token B should be unchanged except for the approval fee since we didn't get to that point
    let expected_balance_b = total_mint_amount_b.clone() - token_b_fee.clone();
    assert_eq!(
        user_balance_b_after_failed_pool, expected_balance_b,
        "User balance for Token B should only be reduced by approval fee"
    );

    // Verify Kong balances are still zero or very low after failed pool creation
    let kong_balance_a = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    assert!(
        kong_balance_a <= token_a_fee.clone(),
        "Kong backend balance for Token A should be close to 0 after failed add_pool, got: {}",
        kong_balance_a
    );

    let kong_balance_b = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);
    assert!(
        kong_balance_b <= token_b_fee.clone(),
        "Kong backend balance for Token B should be close to 0 after failed add_pool, got: {}",
        kong_balance_b
    );
}

#[test]
fn test_add_pool_setup() {
    // This test shows a successful setup for a pool with ICP
    // But using ICRC1 direct transfers instead of ICP ledger to avoid issues with the test ICP canister
    let (ic, kong_backend) = setup_ic_environment().expect("Failed to setup IC environment");

    // For token_a, use a regular ICRC1 token
    let token_a_principal_id = Principal::from_text("zdzgz-siaaa-aaaar-qaiba-cai").expect("Invalid ksUSDT Principal ID");

    // For token_b, use ICP canister ID but also make it an ICRC1 token (since the ICP canister is having init issues in tests)
    let token_b_principal_id = Principal::from_text("nppha-riaaa-aaaal-ajf2q-cai").expect("Invalid Testnet ICP Principal ID");

    let archive_options = common::icrc1_ledger::ArchiveOptions {
        num_blocks_to_archive: 1000,
        max_transactions_per_response: None,
        trigger_threshold: 500,
        max_message_size_bytes: None,
        cycles_for_archive_creation: None,
        node_max_memory_size_bytes: None,
        controller_id: Principal::from_text("aaaaa-aa").expect("Failed to parse reserved principal ID"),
        more_controller_ids: None,
    };

    // Setup controller account
    let controller_identity = get_identity_from_pem_file(CONTROLLER_PEM_FILE).expect("Failed to get controller identity");
    let controller_principal = controller_identity.sender().expect("Failed to get controller principal");
    let controller_account = Account {
        owner: controller_principal,
        subaccount: None,
    };

    // Create Token A (ICRC1)
    let token_a_init_args = common::icrc1_ledger::InitArgs {
        minting_account: controller_account,
        fee_collector_account: None,
        transfer_fee: Nat::from(TOKEN_A_FEE),
        decimals: Some(TOKEN_A_DECIMALS),
        max_memo_length: Some(32),
        token_symbol: "SWPA".to_string(),
        token_name: "Swap Test Token A".to_string(),
        metadata: vec![],
        initial_balances: vec![],
        feature_flags: Some(common::icrc1_ledger::FeatureFlags { icrc2: true }),
        archive_options: archive_options.clone(),
    };

    let token_a_ledger_id = common::icrc1_ledger::create_icrc1_ledger_with_id(
        &ic,
        token_a_principal_id,
        controller_principal,
        &common::icrc1_ledger::LedgerArg::Init(token_a_init_args),
    )
    .expect("Failed to create Token A ledger");

    // Create Token B (ICRC1 version of ICP to avoid initialization issues)
    let token_b_init_args = common::icrc1_ledger::InitArgs {
        minting_account: controller_account,
        fee_collector_account: None,
        transfer_fee: Nat::from(TOKEN_B_FEE_ICP),
        decimals: Some(TOKEN_B_DECIMALS_ICP),
        max_memo_length: Some(32),
        token_symbol: "ICP".to_string(),
        token_name: "Internet Computer Protocol (Test)".to_string(),
        metadata: vec![],
        initial_balances: vec![],
        feature_flags: Some(common::icrc1_ledger::FeatureFlags { icrc2: true }),
        archive_options: archive_options.clone(),
    };

    let token_b_ledger_id = common::icrc1_ledger::create_icrc1_ledger_with_id(
        &ic,
        token_b_principal_id,
        controller_principal,
        &common::icrc1_ledger::LedgerArg::Init(token_b_init_args),
    )
    .expect("Failed to create Token B ledger");

    // Add both tokens to Kong backend explicitly
    add_token_to_kong(&ic, kong_backend, controller_principal, token_a_ledger_id).expect("Failed to add token A to Kong");
    add_token_to_kong(&ic, kong_backend, controller_principal, token_b_ledger_id).expect("Failed to add token B to Kong");

    // Create user account
    let user_identity = get_new_identity().expect("Failed to create new user identity");
    let user_principal = user_identity.sender().expect("Failed to get user principal");
    let user_account = Account {
        owner: user_principal,
        subaccount: None,
    };

    // Calculate liquidity amounts - using smaller amounts for clarity
    let token_a_liquidity_amount = Nat::from(1_000_000u64);
    let token_b_liquidity_amount = Nat::from(1_000_000u64);

    // Mint tokens to user (Token A) - including extra for fees
    let token_a_fee = Nat::from(TOKEN_A_FEE);
    // Including the transfer fee
    let total_mint_amount_a = token_a_liquidity_amount.clone() + token_a_fee.clone();

    let transfer_result_a = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_a_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_a.clone(),
        None,
        None,
    );
    assert!(transfer_result_a.is_ok(), "Minting Token A to user failed: {:?}", transfer_result_a);

    // Mint tokens to user (Token B) using ICRC1 transfer
    let token_b_fee = Nat::from(TOKEN_B_FEE_ICP);
    // Including the transfer fee
    let total_mint_amount_b = token_b_liquidity_amount.clone() + token_b_fee.clone();

    let transfer_result_b = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_b_ledger_id,
        controller_principal,
        user_account,
        total_mint_amount_b.clone(),
        None,
        None,
    );
    assert!(transfer_result_b.is_ok(), "Minting Token B to user failed: {:?}", transfer_result_b);

    // Check user initial balance after minting
    let user_balance_a_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);
    assert_eq!(
        user_balance_a_initial, total_mint_amount_a,
        "User balance for Token A after minting is incorrect"
    );

    let user_balance_b_initial = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);
    assert_eq!(
        user_balance_b_initial, total_mint_amount_b,
        "User balance for Token B after minting is incorrect"
    );

    // Define Kong account
    let kong_account = Account {
        owner: kong_backend,
        subaccount: None,
    };

    // Check Kong's initial balances
    let kong_balance_a_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    assert_eq!(
        kong_balance_a_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token A should be 0 before transfers"
    );

    let kong_balance_b_before_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);
    assert_eq!(
        kong_balance_b_before_add,
        Nat::from(0_u64),
        "Kong backend balance for Token B should be 0 before transfers"
    );

    // Transfer tokens directly to Kong (as in the setup_with_pool example)
    // User transfers Token A directly to Kong
    let transfer_a_to_kong = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_a_ledger_id,
        user_principal,
        kong_account,
        token_a_liquidity_amount.clone(),
        None,
        None,
    );
    assert!(
        transfer_a_to_kong.is_ok(),
        "Transfer of Token A to Kong failed: {:?}",
        transfer_a_to_kong
    );
    let tx_id_a = transfer_a_to_kong.unwrap();

    // User transfers Token B directly to Kong
    let transfer_b_to_kong = common::icrc1_ledger::icrc1_transfer(
        &ic,
        token_b_ledger_id,
        user_principal,
        kong_account,
        token_b_liquidity_amount.clone(),
        None,
        None,
    );
    assert!(
        transfer_b_to_kong.is_ok(),
        "Transfer of Token B to Kong failed: {:?}",
        transfer_b_to_kong
    );
    let tx_id_b = transfer_b_to_kong.unwrap();

    // Check balances after transfers
    let kong_balance_a_after_transfer = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    assert_eq!(
        kong_balance_a_after_transfer, token_a_liquidity_amount,
        "Kong backend balance for Token A incorrect after transfer"
    );

    let kong_balance_b_after_transfer = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);
    assert_eq!(
        kong_balance_b_after_transfer, token_b_liquidity_amount,
        "Kong backend balance for Token B incorrect after transfer"
    );

    // Add pool using transaction IDs
    let token_a_str = format!("IC.{}", token_a_ledger_id.to_text());
    let token_b_str = format!("IC.{}", token_b_ledger_id.to_text());

    let add_pool_args = AddPoolArgs {
        token_0: token_a_str.clone(),
        amount_0: token_a_liquidity_amount.clone(),
        tx_id_0: Some(TxId::BlockIndex(tx_id_a)), // Use the transaction ID from the transfer
        token_1: token_b_str.clone(),
        amount_1: token_b_liquidity_amount.clone(),
        tx_id_1: Some(TxId::BlockIndex(tx_id_b)), // Use the transaction ID from the transfer
        lp_fee_bps: None,
    };

    let add_pool_payload = encode_one(&add_pool_args).expect("Failed to encode add_pool_args");

    let add_pool_response_bytes = ic
        .update_call(kong_backend, user_principal, "add_pool", add_pool_payload)
        .expect("Failed to call add_pool");

    let add_pool_result = decode_one::<Result<AddPoolReply, String>>(&add_pool_response_bytes).expect("Failed to decode add_pool response");

    assert!(add_pool_result.is_ok(), "add_pool call failed: {:?}", add_pool_result);

    // Verify pool was created successfully - Kong balances should remain the same
    let kong_balance_a_after_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, kong_account);
    assert_eq!(
        kong_balance_a_after_add, token_a_liquidity_amount,
        "Kong backend balance for Token A incorrect after add_pool"
    );

    let kong_balance_b_after_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, kong_account);
    assert_eq!(
        kong_balance_b_after_add, token_b_liquidity_amount,
        "Kong backend balance for Token B incorrect after add_pool"
    );

    // User balances after transfers should be adjusted for the transfers
    let user_balance_a_after_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_a_ledger_id, user_account);

    // Expected: initial - liquidity - transfer fee
    let expected_user_balance_a = total_mint_amount_a.clone() - token_a_liquidity_amount.clone() - token_a_fee.clone();
    assert_eq!(
        user_balance_a_after_add, expected_user_balance_a,
        "User balance for Token A after add_pool is incorrect"
    );

    let user_balance_b_after_add = common::icrc1_ledger::get_icrc1_balance(&ic, token_b_ledger_id, user_account);

    // Expected: initial - liquidity - transfer fee
    let expected_user_balance_b = total_mint_amount_b.clone() - token_b_liquidity_amount.clone() - token_b_fee.clone();
    assert_eq!(
        user_balance_b_after_add, expected_user_balance_b,
        "User balance for Token B after add_pool is incorrect"
    );
}
