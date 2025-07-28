#!/usr/bin/env bash
set -euo pipefail

# adds liquidity to existing SOL/USDT pool
# usage: sh add_sol_lp_mainnet.sh [SOL_AMOUNT]
# SOL_AMOUNT: amount in human-readable SOL (e.g., 0.01 for 0.01 SOL)

# Ensure Solana CLI network is already configured

# ============================ CONFIG ============================
NETWORK="ic"                            # Always use IC mainnet
# Use current identity - no hardcoded identity flag
CURRENT_IDENTITY=$(dfx identity whoami)
echo "Using current identity: ${CURRENT_IDENTITY}"
IDENTITY_FLAG="" # No identity flag means use current identity

# CANISTER IDS
MAINNET_KONG_BACKEND="u6kfa-6aaaa-aaaam-qdxba-cai"
LOCAL_KONG_BACKEND="kong_backend"  # Will use dfx canister id locally

# Token 0 (Solana - SOL)
SOL_CHAIN="SOL"
SOL_ADDRESS="11111111111111111111111111111111"   # Native SOL mint
SOL_AMOUNT_HUMAN="${1:-0.01}"   # Default 0.01 SOL - human readable
# Convert to lamports (9 decimals)
SOL_AMOUNT_DESIRED=$(echo "${SOL_AMOUNT_HUMAN} * 1000000000" | bc | cut -d. -f1)

# Token 1 (USDT on IC)
USDT_CHAIN="IC"
USDT_SYMBOL="ckUSDT"
USDT_FEE=10000
# USDT LEDGER CANISTER ID
USDT_LEDGER="cngnf-vqaaa-aaaar-qag4q-cai"  # ckUSDT mainnet
# ===============================================================

NETWORK_FLAG="--network ic"
KONG_BACKEND="${MAINNET_KONG_BACKEND}"

# --- Helper ---
check_ok() { local r="$1"; local ctx="$2"; echo "$r" | grep -q -e "Ok" -e "ok" || { echo "Error: $ctx" >&2; echo "$r" >&2; exit 1; }; }

# --- 0. Setup ---
KONG_SOL_RAW=$(dfx canister call ${NETWORK_FLAG} ${KONG_BACKEND} get_solana_address --output json)
KONG_SOL_ADDR=$(echo "$KONG_SOL_RAW" | jq -r '.')

echo "Kong Solana Address: $KONG_SOL_ADDR"

# --- 0.5. Query exact amounts needed ---
echo "Querying required amounts for adding ${SOL_AMOUNT_HUMAN} SOL..."
AMOUNTS_RESULT=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} add_liquidity_amounts "(\"${SOL_CHAIN}.${SOL_ADDRESS}\", ${SOL_AMOUNT_DESIRED}, \"${USDT_CHAIN}.${USDT_LEDGER}\")" --output json)
check_ok "$AMOUNTS_RESULT" "add_liquidity_amounts failed"

# Parse the amounts from the result - remove underscores from numbers
SOL_AMOUNT=$(echo "$AMOUNTS_RESULT" | jq -r '.Ok.amount_0' | tr -d '_')
USDT_AMOUNT=$(echo "$AMOUNTS_RESULT" | jq -r '.Ok.amount_1' | tr -d '_')
LP_TOKENS=$(echo "$AMOUNTS_RESULT" | jq -r '.Ok.add_lp_token_amount' | tr -d '_')

SOL_AMOUNT_DISPLAY=$(echo "scale=9; ${SOL_AMOUNT}/1000000000" | bc)
USDT_AMOUNT_DISPLAY=$(echo "scale=6; ${USDT_AMOUNT}/1000000" | bc)
echo "Required amounts:"
echo "  SOL: ${SOL_AMOUNT_DISPLAY} SOL (${SOL_AMOUNT} lamports)"
echo "  ckUSDT: ${USDT_AMOUNT_DISPLAY} USDT (${USDT_AMOUNT} units)" 
echo "  Expected LP tokens: ${LP_TOKENS}"

# --- 1. Transfer SOL ---
SOL_DEC=$(bc <<< "scale=9; ${SOL_AMOUNT}/1000000000")
TX_OUT=$(solana transfer --allow-unfunded-recipient "$KONG_SOL_ADDR" "$SOL_DEC")
SOL_TX_SIG=$(echo "$TX_OUT" | grep -o 'Signature: .*' | awk '{print $2}')

echo "Transferred $SOL_DEC SOL (tx $SOL_TX_SIG)"
echo "Waiting for transaction to be processed by kong_rpc..."
sleep 4

# --- 2. Approve USDT ---
APPROVE_AMOUNT=$((USDT_AMOUNT+USDT_FEE))
APR=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${USDT_LEDGER} icrc2_approve "(record { amount = ${APPROVE_AMOUNT}; spender = record { owner = principal \"${KONG_BACKEND}\" }; })")
check_ok "$APR" "USDT approve failed"

# --- 3. Sign message ---
MSG=$(printf '{"token_0":"%s.%s","amount_0":"%s","token_1":"%s.%s","amount_1":"%s"}' \
  "$SOL_CHAIN" "$SOL_ADDRESS" "$SOL_AMOUNT" \
  "$USDT_CHAIN" "$USDT_LEDGER" "$USDT_AMOUNT")
echo "DEBUG: Message to sign: $MSG"
SIG=$(solana sign-offchain-message "$MSG")

# --- 4. Add liquidity ---
CALL="(record { token_0 = \"${SOL_CHAIN}.${SOL_ADDRESS}\"; amount_0=${SOL_AMOUNT}; tx_id_0 = opt variant { TransactionId = \"${SOL_TX_SIG}\" }; token_1 = \"${USDT_CHAIN}.${USDT_LEDGER}\"; amount_1=${USDT_AMOUNT}; tx_id_1 = null; signature_0 = opt \"${SIG}\"; signature_1 = null; })"
RES=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} add_liquidity --output json "$CALL")
check_ok "$RES" "add_liquidity failed"
REQ_ID=$(echo "$RES" | jq -r '.Ok.request_id // .request_id // empty')
[[ -n "$REQ_ID" ]] && echo "Liquidity add request submitted: $REQ_ID" || echo "$RES"
