#!/usr/bin/env bash
set -euo pipefail

# adds liquidity to existing SOL/USDT pool
# usage: sh add_lp_sol.sh [local|ic]   (default local)

# Ensure Solana CLI network is already configured

# ============================ CONFIG ============================
NETWORK="${1:-local}"                   # "local" or "ic"
IDENTITY_FLAG="--identity kong_user1" # make sure this identity has the IC funds

# CANISTER IDS
MAINNET_KONG_BACKEND="u6kfa-6aaaa-aaaam-qdxba-cai"
LOCAL_KONG_BACKEND="kong_backend"  # Will use dfx canister id locally

# Token 0 (Solana - SOL)
SOL_CHAIN="SOL"
SOL_ADDRESS="11111111111111111111111111111111"   # Native SOL mint
SOL_AMOUNT=4000000            # 0.004 SOL (9 decimals)

# Token 1 (USDT on IC)
USDT_CHAIN="IC"
USDT_SYMBOL=$([ "${NETWORK}" == "local" ] && echo "ksUSDT" || echo "ckUSDT")
USDT_AMOUNT=1000000           # 1 USDT (6 decimals)
USDT_FEE=10000
# USDT LEDGER CANISTER IDS
MAINNET_USDT_LEDGER="cngnf-vqaaa-aaaar-qag4q-cai"  # ckUSDT
LOCAL_USDT_LEDGER="ksusdt_ledger"  # Will use dfx canister id locally
# ===============================================================

NETWORK_FLAG=$([ "${NETWORK}" == "local" ] && echo "" || echo "--network ${NETWORK}")

# Set canister IDs based on network
if [ "${NETWORK}" == "ic" ]; then
    KONG_BACKEND="${MAINNET_KONG_BACKEND}"
    USDT_LEDGER="${MAINNET_USDT_LEDGER}"
else
    KONG_BACKEND=$(dfx canister id ${LOCAL_KONG_BACKEND})
    USDT_LEDGER=$(dfx canister id ${LOCAL_USDT_LEDGER})
fi

# --- Helper ---
check_ok() { local r="$1"; local ctx="$2"; echo "$r" | grep -q -e "Ok" -e "ok" || { echo "Error: $ctx" >&2; echo "$r" >&2; exit 1; }; }

# --- 0. Setup ---
KONG_SOL_RAW=$(dfx canister call ${NETWORK_FLAG} ${KONG_BACKEND} get_solana_address --output json)
check_ok "$KONG_SOL_RAW" "get_solana_address failed"
KONG_SOL_ADDR=$(echo "$KONG_SOL_RAW" | jq -r '.Ok')

echo "Kong Solana Address: $KONG_SOL_ADDR"

# --- 1. Transfer SOL ---
SOL_DEC=$(bc <<< "scale=9; ${SOL_AMOUNT}/1000000000")
TX_OUT=$(solana transfer --allow-unfunded-recipient "$KONG_SOL_ADDR" "$SOL_DEC")
SOL_TX_SIG=$(echo "$TX_OUT" | grep -o 'Signature: .*' | awk '{print $2}')

echo "Transferred $SOL_DEC SOL (tx $SOL_TX_SIG)"
echo "Waiting for transaction to be processed by kong_rpc..."
sleep 30

# --- 2. Approve USDT ---
APPROVE_AMOUNT=$((USDT_AMOUNT+USDT_FEE))
APR=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${USDT_LEDGER} icrc2_approve "(record { amount = ${APPROVE_AMOUNT}; spender = record { owner = principal \"${KONG_BACKEND}\" }; })")
check_ok "$APR" "USDT approve failed"

# --- 3. Sign message ---
MSG=$(printf '{"token_0":"%s.%s","amount_0":[%s],"token_1":"%s.%s","amount_1":[%s]}' \
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
