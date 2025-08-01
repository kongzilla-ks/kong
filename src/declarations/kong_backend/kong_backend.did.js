export const idlFactory = ({ IDL }) => {
  const TxId = IDL.Variant({
    'TransactionId' : IDL.Text,
    'BlockIndex' : IDL.Nat,
  });
  const AddLiquidityArgs = IDL.Record({
    'signature_0' : IDL.Opt(IDL.Text),
    'signature_1' : IDL.Opt(IDL.Text),
    'token_0' : IDL.Text,
    'token_1' : IDL.Text,
    'amount_0' : IDL.Nat,
    'amount_1' : IDL.Nat,
    'tx_id_0' : IDL.Opt(TxId),
    'tx_id_1' : IDL.Opt(TxId),
  });
  const ICTransferReply = IDL.Record({
    'is_send' : IDL.Bool,
    'block_index' : IDL.Nat,
    'chain' : IDL.Text,
    'canister_id' : IDL.Text,
    'amount' : IDL.Nat,
    'symbol' : IDL.Text,
  });
  const SolanaTransferReply = IDL.Record({
    'is_send' : IDL.Bool,
    'signature' : IDL.Text,
    'chain' : IDL.Text,
    'slot' : IDL.Opt(IDL.Nat64),
    'amount' : IDL.Nat,
    'mint_address' : IDL.Text,
    'symbol' : IDL.Text,
  });
  const TransferReply = IDL.Variant({
    'IC' : ICTransferReply,
    'Solana' : SolanaTransferReply,
  });
  const TransferIdReply = IDL.Record({
    'transfer_id' : IDL.Nat64,
    'transfer' : TransferReply,
  });
  const AddLiquidityReply = IDL.Record({
    'ts' : IDL.Nat64,
    'request_id' : IDL.Nat64,
    'status' : IDL.Text,
    'tx_id' : IDL.Nat64,
    'add_lp_token_amount' : IDL.Nat,
    'transfer_ids' : IDL.Vec(TransferIdReply),
    'amount_0' : IDL.Nat,
    'amount_1' : IDL.Nat,
    'claim_ids' : IDL.Vec(IDL.Nat64),
    'address_0' : IDL.Text,
    'address_1' : IDL.Text,
    'symbol_0' : IDL.Text,
    'symbol_1' : IDL.Text,
    'chain_0' : IDL.Text,
    'chain_1' : IDL.Text,
    'symbol' : IDL.Text,
  });
  const AddLiquidityResult = IDL.Variant({
    'Ok' : AddLiquidityReply,
    'Err' : IDL.Text,
  });
  const AddLiquidityAmountsReply = IDL.Record({
    'add_lp_token_amount' : IDL.Nat,
    'amount_0' : IDL.Nat,
    'amount_1' : IDL.Nat,
    'address_0' : IDL.Text,
    'address_1' : IDL.Text,
    'symbol_0' : IDL.Text,
    'symbol_1' : IDL.Text,
    'chain_0' : IDL.Text,
    'chain_1' : IDL.Text,
    'symbol' : IDL.Text,
    'fee_0' : IDL.Nat,
    'fee_1' : IDL.Nat,
  });
  const AddLiquiditAmountsResult = IDL.Variant({
    'Ok' : AddLiquidityAmountsReply,
    'Err' : IDL.Text,
  });
  const AddLiquidityAsyncResult = IDL.Variant({
    'Ok' : IDL.Nat64,
    'Err' : IDL.Text,
  });
  const AddPoolArgs = IDL.Record({
    'signature_0' : IDL.Opt(IDL.Text),
    'signature_1' : IDL.Opt(IDL.Text),
    'token_0' : IDL.Text,
    'token_1' : IDL.Text,
    'amount_0' : IDL.Nat,
    'amount_1' : IDL.Nat,
    'tx_id_0' : IDL.Opt(TxId),
    'tx_id_1' : IDL.Opt(TxId),
    'lp_fee_bps' : IDL.Opt(IDL.Nat8),
  });
  const AddPoolReply = IDL.Record({
    'ts' : IDL.Nat64,
    'request_id' : IDL.Nat64,
    'status' : IDL.Text,
    'tx_id' : IDL.Nat64,
    'lp_token_symbol' : IDL.Text,
    'add_lp_token_amount' : IDL.Nat,
    'transfer_ids' : IDL.Vec(TransferIdReply),
    'name' : IDL.Text,
    'amount_0' : IDL.Nat,
    'amount_1' : IDL.Nat,
    'claim_ids' : IDL.Vec(IDL.Nat64),
    'address_0' : IDL.Text,
    'address_1' : IDL.Text,
    'symbol_0' : IDL.Text,
    'symbol_1' : IDL.Text,
    'pool_id' : IDL.Nat32,
    'chain_0' : IDL.Text,
    'chain_1' : IDL.Text,
    'is_removed' : IDL.Bool,
    'symbol' : IDL.Text,
    'lp_fee_bps' : IDL.Nat8,
  });
  const AddPoolResult = IDL.Variant({ 'Ok' : AddPoolReply, 'Err' : IDL.Text });
  const AddTokenArgs = IDL.Record({ 'token' : IDL.Text });
  const ICTokenReply = IDL.Record({
    'fee' : IDL.Nat,
    'decimals' : IDL.Nat8,
    'token_id' : IDL.Nat32,
    'chain' : IDL.Text,
    'name' : IDL.Text,
    'canister_id' : IDL.Text,
    'icrc1' : IDL.Bool,
    'icrc2' : IDL.Bool,
    'icrc3' : IDL.Bool,
    'is_removed' : IDL.Bool,
    'symbol' : IDL.Text,
  });
  const SolanaTokenReply = IDL.Record({
    'fee' : IDL.Nat,
    'decimals' : IDL.Nat8,
    'token_id' : IDL.Nat32,
    'chain' : IDL.Text,
    'name' : IDL.Text,
    'program_id' : IDL.Text,
    'is_spl_token' : IDL.Bool,
    'mint_address' : IDL.Text,
    'symbol' : IDL.Text,
  });
  const AddTokenReply = IDL.Variant({
    'IC' : ICTokenReply,
    'Solana' : SolanaTokenReply,
  });
  const AddTokenResult = IDL.Variant({
    'Ok' : AddTokenReply,
    'Err' : IDL.Text,
  });
  const PoolExpectedBalance = IDL.Record({
    'balance' : IDL.Nat,
    'kong_fee' : IDL.Nat,
    'pool_symbol' : IDL.Text,
    'lp_fee' : IDL.Nat,
  });
  const ExpectedBalance = IDL.Record({
    'balance' : IDL.Nat,
    'pool_balances' : IDL.Vec(PoolExpectedBalance),
    'unclaimed_claims' : IDL.Nat,
  });
  const CheckPoolsReply = IDL.Record({
    'expected_balance' : ExpectedBalance,
    'diff_balance' : IDL.Int,
    'actual_balance' : IDL.Nat,
    'symbol' : IDL.Text,
  });
  const CheckPoolsResult = IDL.Variant({
    'Ok' : IDL.Vec(CheckPoolsReply),
    'Err' : IDL.Text,
  });
  const ClaimReply = IDL.Record({
    'ts' : IDL.Nat64,
    'fee' : IDL.Nat,
    'status' : IDL.Text,
    'claim_id' : IDL.Nat64,
    'transfer_ids' : IDL.Vec(TransferIdReply),
    'desc' : IDL.Text,
    'chain' : IDL.Text,
    'canister_id' : IDL.Opt(IDL.Text),
    'to_address' : IDL.Text,
    'amount' : IDL.Nat,
    'symbol' : IDL.Text,
  });
  const ClaimResult = IDL.Variant({ 'Ok' : ClaimReply, 'Err' : IDL.Text });
  const ClaimsReply = IDL.Record({
    'ts' : IDL.Nat64,
    'fee' : IDL.Nat,
    'status' : IDL.Text,
    'claim_id' : IDL.Nat64,
    'desc' : IDL.Text,
    'chain' : IDL.Text,
    'canister_id' : IDL.Opt(IDL.Text),
    'to_address' : IDL.Text,
    'amount' : IDL.Nat,
    'symbol' : IDL.Text,
  });
  const ClaimsResult = IDL.Variant({
    'Ok' : IDL.Vec(ClaimsReply),
    'Err' : IDL.Text,
  });
  const UserReply = IDL.Record({
    'account_id' : IDL.Text,
    'fee_level_expires_at' : IDL.Opt(IDL.Nat64),
    'referred_by' : IDL.Opt(IDL.Text),
    'user_id' : IDL.Nat32,
    'fee_level' : IDL.Nat8,
    'principal_id' : IDL.Text,
    'referred_by_expires_at' : IDL.Opt(IDL.Nat64),
    'my_referral_code' : IDL.Text,
  });
  const UserResult = IDL.Variant({ 'Ok' : UserReply, 'Err' : IDL.Text });
  const Icrc10SupportedStandards = IDL.Record({
    'url' : IDL.Text,
    'name' : IDL.Text,
  });
  const icrc21_consent_message_metadata = IDL.Record({
    'utc_offset_minutes' : IDL.Opt(IDL.Int16),
    'language' : IDL.Text,
  });
  const icrc21_consent_message_spec = IDL.Record({
    'metadata' : icrc21_consent_message_metadata,
    'device_spec' : IDL.Opt(
      IDL.Variant({
        'GenericDisplay' : IDL.Null,
        'LineDisplay' : IDL.Record({
          'characters_per_line' : IDL.Nat16,
          'lines_per_page' : IDL.Nat16,
        }),
      })
    ),
  });
  const icrc21_consent_message_request = IDL.Record({
    'arg' : IDL.Vec(IDL.Nat8),
    'method' : IDL.Text,
    'user_preferences' : icrc21_consent_message_spec,
  });
  const icrc21_consent_message = IDL.Variant({
    'LineDisplayMessage' : IDL.Record({
      'pages' : IDL.Vec(IDL.Record({ 'lines' : IDL.Vec(IDL.Text) })),
    }),
    'GenericDisplayMessage' : IDL.Text,
  });
  const icrc21_consent_info = IDL.Record({
    'metadata' : icrc21_consent_message_metadata,
    'consent_message' : icrc21_consent_message,
  });
  const icrc21_error_info = IDL.Record({ 'description' : IDL.Text });
  const icrc21_error = IDL.Variant({
    'GenericError' : IDL.Record({
      'description' : IDL.Text,
      'error_code' : IDL.Nat,
    }),
    'InsufficientPayment' : icrc21_error_info,
    'UnsupportedCanisterCall' : icrc21_error_info,
    'ConsentMessageUnavailable' : icrc21_error_info,
  });
  const icrc21_consent_message_response = IDL.Variant({
    'Ok' : icrc21_consent_info,
    'Err' : icrc21_error,
  });
  const Icrc28TrustedOriginsResponse = IDL.Record({
    'trusted_origins' : IDL.Vec(IDL.Text),
  });
  const PoolReply = IDL.Record({
    'lp_token_symbol' : IDL.Text,
    'name' : IDL.Text,
    'lp_fee_0' : IDL.Nat,
    'lp_fee_1' : IDL.Nat,
    'balance_0' : IDL.Nat,
    'balance_1' : IDL.Nat,
    'address_0' : IDL.Text,
    'address_1' : IDL.Text,
    'symbol_0' : IDL.Text,
    'symbol_1' : IDL.Text,
    'pool_id' : IDL.Nat32,
    'price' : IDL.Float64,
    'chain_0' : IDL.Text,
    'chain_1' : IDL.Text,
    'is_removed' : IDL.Bool,
    'symbol' : IDL.Text,
    'lp_fee_bps' : IDL.Nat8,
  });
  const PoolsResult = IDL.Variant({
    'Ok' : IDL.Vec(PoolReply),
    'Err' : IDL.Text,
  });
  const RemoveLiquidityArgs = IDL.Record({
    'signature_0' : IDL.Opt(IDL.Text),
    'signature_1' : IDL.Opt(IDL.Text),
    'token_0' : IDL.Text,
    'token_1' : IDL.Text,
    'payout_address_0' : IDL.Opt(IDL.Text),
    'payout_address_1' : IDL.Opt(IDL.Text),
    'remove_lp_token_amount' : IDL.Nat,
  });
  const RemoveLiquidityReply = IDL.Record({
    'ts' : IDL.Nat64,
    'request_id' : IDL.Nat64,
    'status' : IDL.Text,
    'tx_id' : IDL.Nat64,
    'transfer_ids' : IDL.Vec(TransferIdReply),
    'lp_fee_0' : IDL.Nat,
    'lp_fee_1' : IDL.Nat,
    'amount_0' : IDL.Nat,
    'amount_1' : IDL.Nat,
    'claim_ids' : IDL.Vec(IDL.Nat64),
    'address_0' : IDL.Text,
    'address_1' : IDL.Text,
    'symbol_0' : IDL.Text,
    'symbol_1' : IDL.Text,
    'chain_0' : IDL.Text,
    'chain_1' : IDL.Text,
    'remove_lp_token_amount' : IDL.Nat,
    'symbol' : IDL.Text,
  });
  const RemoveLiquidityResult = IDL.Variant({
    'Ok' : RemoveLiquidityReply,
    'Err' : IDL.Text,
  });
  const RemoveLiquidityAmountsReply = IDL.Record({
    'lp_fee_0' : IDL.Nat,
    'lp_fee_1' : IDL.Nat,
    'amount_0' : IDL.Nat,
    'amount_1' : IDL.Nat,
    'address_0' : IDL.Text,
    'address_1' : IDL.Text,
    'symbol_0' : IDL.Text,
    'symbol_1' : IDL.Text,
    'chain_0' : IDL.Text,
    'chain_1' : IDL.Text,
    'remove_lp_token_amount' : IDL.Nat,
    'symbol' : IDL.Text,
  });
  const RemoveLiquidityAmountsResult = IDL.Variant({
    'Ok' : RemoveLiquidityAmountsReply,
    'Err' : IDL.Text,
  });
  const RemoveLiquidityAsyncResult = IDL.Variant({
    'Ok' : IDL.Nat64,
    'Err' : IDL.Text,
  });
  const SwapArgs = IDL.Record({
    'receive_token' : IDL.Text,
    'max_slippage' : IDL.Opt(IDL.Float64),
    'pay_amount' : IDL.Nat,
    'referred_by' : IDL.Opt(IDL.Text),
    'receive_amount' : IDL.Opt(IDL.Nat),
    'receive_address' : IDL.Opt(IDL.Text),
    'pay_token' : IDL.Text,
    'pay_tx_id' : IDL.Opt(TxId),
    'pay_signature' : IDL.Opt(IDL.Text),
  });
  const RequestRequest = IDL.Variant({
    'AddLiquidity' : AddLiquidityArgs,
    'Swap' : SwapArgs,
    'AddPool' : AddPoolArgs,
    'RemoveLiquidity' : RemoveLiquidityArgs,
  });
  const SwapTxReply = IDL.Record({
    'ts' : IDL.Nat64,
    'receive_chain' : IDL.Text,
    'pay_amount' : IDL.Nat,
    'receive_amount' : IDL.Nat,
    'pay_symbol' : IDL.Text,
    'receive_symbol' : IDL.Text,
    'receive_address' : IDL.Text,
    'pool_symbol' : IDL.Text,
    'pay_address' : IDL.Text,
    'price' : IDL.Float64,
    'pay_chain' : IDL.Text,
    'lp_fee' : IDL.Nat,
    'gas_fee' : IDL.Nat,
  });
  const SwapReply = IDL.Record({
    'ts' : IDL.Nat64,
    'txs' : IDL.Vec(SwapTxReply),
    'request_id' : IDL.Nat64,
    'status' : IDL.Text,
    'tx_id' : IDL.Nat64,
    'transfer_ids' : IDL.Vec(TransferIdReply),
    'receive_chain' : IDL.Text,
    'mid_price' : IDL.Float64,
    'pay_amount' : IDL.Nat,
    'receive_amount' : IDL.Nat,
    'claim_ids' : IDL.Vec(IDL.Nat64),
    'pay_symbol' : IDL.Text,
    'receive_symbol' : IDL.Text,
    'receive_address' : IDL.Text,
    'pay_address' : IDL.Text,
    'price' : IDL.Float64,
    'pay_chain' : IDL.Text,
    'slippage' : IDL.Float64,
  });
  const RequestReply = IDL.Variant({
    'AddLiquidity' : AddLiquidityReply,
    'Swap' : SwapReply,
    'AddPool' : AddPoolReply,
    'RemoveLiquidity' : RemoveLiquidityReply,
    'Pending' : IDL.Null,
  });
  const RequestsReply = IDL.Record({
    'ts' : IDL.Nat64,
    'request_id' : IDL.Nat64,
    'request' : RequestRequest,
    'statuses' : IDL.Vec(IDL.Text),
    'reply' : RequestReply,
  });
  const RequestsResult = IDL.Variant({
    'Ok' : IDL.Vec(RequestsReply),
    'Err' : IDL.Text,
  });
  const SendArgs = IDL.Record({
    'token' : IDL.Text,
    'to_address' : IDL.Text,
    'amount' : IDL.Nat,
  });
  const SendReply = IDL.Record({
    'ts' : IDL.Nat64,
    'request_id' : IDL.Nat64,
    'status' : IDL.Text,
    'tx_id' : IDL.Nat64,
    'chain' : IDL.Text,
    'to_address' : IDL.Text,
    'amount' : IDL.Nat,
    'symbol' : IDL.Text,
  });
  const SendResult = IDL.Variant({ 'Ok' : SendReply, 'Err' : IDL.Text });
  const SwapResult = IDL.Variant({ 'Ok' : SwapReply, 'Err' : IDL.Text });
  const SwapAmountsTxReply = IDL.Record({
    'receive_chain' : IDL.Text,
    'pay_amount' : IDL.Nat,
    'receive_amount' : IDL.Nat,
    'pay_symbol' : IDL.Text,
    'receive_symbol' : IDL.Text,
    'receive_address' : IDL.Text,
    'pool_symbol' : IDL.Text,
    'pay_address' : IDL.Text,
    'price' : IDL.Float64,
    'pay_chain' : IDL.Text,
    'lp_fee' : IDL.Nat,
    'gas_fee' : IDL.Nat,
  });
  const SwapAmountsReply = IDL.Record({
    'txs' : IDL.Vec(SwapAmountsTxReply),
    'receive_chain' : IDL.Text,
    'mid_price' : IDL.Float64,
    'pay_amount' : IDL.Nat,
    'receive_amount' : IDL.Nat,
    'pay_symbol' : IDL.Text,
    'receive_symbol' : IDL.Text,
    'receive_address' : IDL.Text,
    'pay_address' : IDL.Text,
    'price' : IDL.Float64,
    'pay_chain' : IDL.Text,
    'slippage' : IDL.Float64,
  });
  const SwapAmountsResult = IDL.Variant({
    'Ok' : SwapAmountsReply,
    'Err' : IDL.Text,
  });
  const SwapAsyncResult = IDL.Variant({ 'Ok' : IDL.Nat64, 'Err' : IDL.Text });
  const LPTokenReply = IDL.Record({
    'fee' : IDL.Nat,
    'decimals' : IDL.Nat8,
    'token_id' : IDL.Nat32,
    'chain' : IDL.Text,
    'name' : IDL.Text,
    'address' : IDL.Text,
    'pool_id_of' : IDL.Nat32,
    'is_removed' : IDL.Bool,
    'total_supply' : IDL.Nat,
    'symbol' : IDL.Text,
  });
  const TokenReply = IDL.Variant({
    'IC' : ICTokenReply,
    'LP' : LPTokenReply,
    'Solana' : SolanaTokenReply,
  });
  const TokensResult = IDL.Variant({
    'Ok' : IDL.Vec(TokenReply),
    'Err' : IDL.Text,
  });
  const UpdateTokenArgs = IDL.Record({ 'token' : IDL.Text });
  const UpdateTokenReply = IDL.Variant({ 'IC' : ICTokenReply });
  const UpdateTokenResult = IDL.Variant({
    'Ok' : UpdateTokenReply,
    'Err' : IDL.Text,
  });
  const LPBalancesReply = IDL.Record({
    'ts' : IDL.Nat64,
    'usd_balance' : IDL.Float64,
    'balance' : IDL.Float64,
    'name' : IDL.Text,
    'amount_0' : IDL.Float64,
    'amount_1' : IDL.Float64,
    'address_0' : IDL.Text,
    'address_1' : IDL.Text,
    'symbol_0' : IDL.Text,
    'symbol_1' : IDL.Text,
    'usd_amount_0' : IDL.Float64,
    'usd_amount_1' : IDL.Float64,
    'chain_0' : IDL.Text,
    'chain_1' : IDL.Text,
    'symbol' : IDL.Text,
    'lp_token_id' : IDL.Nat64,
  });
  const UserBalancesReply = IDL.Variant({ 'LP' : LPBalancesReply });
  const UserBalancesResult = IDL.Variant({
    'Ok' : IDL.Vec(UserBalancesReply),
    'Err' : IDL.Text,
  });
  const ValidateAddLiquidityResult = IDL.Variant({
    'Ok' : IDL.Text,
    'Err' : IDL.Text,
  });
  const ValidateRemoveLiquidityResult = IDL.Variant({
    'Ok' : IDL.Text,
    'Err' : IDL.Text,
  });
  return IDL.Service({
    'add_liquidity' : IDL.Func([AddLiquidityArgs], [AddLiquidityResult], []),
    'add_liquidity_amounts' : IDL.Func(
        [IDL.Text, IDL.Nat, IDL.Text],
        [AddLiquiditAmountsResult],
        ['query'],
      ),
    'add_liquidity_async' : IDL.Func(
        [AddLiquidityArgs],
        [AddLiquidityAsyncResult],
        [],
      ),
    'add_pool' : IDL.Func([AddPoolArgs], [AddPoolResult], []),
    'add_token' : IDL.Func([AddTokenArgs], [AddTokenResult], []),
    'check_pools' : IDL.Func([], [CheckPoolsResult], []),
    'claim' : IDL.Func([IDL.Nat64], [ClaimResult], []),
    'claims' : IDL.Func([IDL.Text], [ClaimsResult], ['query']),
    'cleanup_expired_solana_jobs' : IDL.Func(
        [],
        [IDL.Variant({ 'Ok' : IDL.Nat32, 'Err' : IDL.Text })],
        [],
      ),
    'get_solana_address' : IDL.Func(
        [],
        [IDL.Text],
        ['query'],
      ),
    'get_user' : IDL.Func([], [UserResult], ['query']),
    'icrc10_supported_standards' : IDL.Func(
        [],
        [IDL.Vec(Icrc10SupportedStandards)],
        ['query'],
      ),
    'icrc1_name' : IDL.Func([], [IDL.Text], ['query']),
    'icrc21_canister_call_consent_message' : IDL.Func(
        [icrc21_consent_message_request],
        [icrc21_consent_message_response],
        [],
      ),
    'icrc28_trusted_origins' : IDL.Func([], [Icrc28TrustedOriginsResponse], []),
    'notify_solana_transfer' : IDL.Func(
        [IDL.Text, IDL.Opt(IDL.Text)],
        [IDL.Variant({ 'Ok' : IDL.Null, 'Err' : IDL.Text })],
        [],
      ),
    'pools' : IDL.Func([IDL.Opt(IDL.Text)], [PoolsResult], ['query']),
    'remove_liquidity' : IDL.Func(
        [RemoveLiquidityArgs],
        [RemoveLiquidityResult],
        [],
      ),
    'remove_liquidity_amounts' : IDL.Func(
        [IDL.Text, IDL.Text, IDL.Nat],
        [RemoveLiquidityAmountsResult],
        ['query'],
      ),
    'remove_liquidity_async' : IDL.Func(
        [RemoveLiquidityArgs],
        [RemoveLiquidityAsyncResult],
        [],
      ),
    'requests' : IDL.Func([IDL.Opt(IDL.Nat64)], [RequestsResult], ['query']),
    'send' : IDL.Func([SendArgs], [SendResult], []),
    'swap' : IDL.Func([SwapArgs], [SwapResult], []),
    'swap_amounts' : IDL.Func(
        [IDL.Text, IDL.Nat, IDL.Text],
        [SwapAmountsResult],
        ['query'],
      ),
    'swap_async' : IDL.Func([SwapArgs], [SwapAsyncResult], []),
    'tokens' : IDL.Func([IDL.Opt(IDL.Text)], [TokensResult], ['query']),
    'update_solana_latest_blockhash' : IDL.Func(
        [IDL.Text],
        [IDL.Variant({ 'Ok' : IDL.Null, 'Err' : IDL.Text })],
        [],
      ),
    'update_solana_swap' : IDL.Func(
        [IDL.Nat64, IDL.Text, IDL.Bool, IDL.Opt(IDL.Text)],
        [IDL.Variant({ 'Ok' : IDL.Null, 'Err' : IDL.Text })],
        [],
      ),
    'update_token' : IDL.Func([UpdateTokenArgs], [UpdateTokenResult], []),
    'user_balances' : IDL.Func([IDL.Text], [UserBalancesResult], ['query']),
    'validate_add_liquidity' : IDL.Func([], [ValidateAddLiquidityResult], []),
    'validate_remove_liquidity' : IDL.Func(
        [],
        [ValidateRemoveLiquidityResult],
        [],
      ),
  });
};
export const init = ({ IDL }) => { return []; };
