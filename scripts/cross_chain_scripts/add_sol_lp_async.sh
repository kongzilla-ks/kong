#!/usr/bin/env bash
set -euo pipefail

# adds liquidity to existing SOL/USDT pool using async add_liquidity
# usage: sh add_sol_lp_async.sh [local|ic]   (default local)

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

# --- Helper to check for command success ---
check_ok() {
    local result="$1"; local context="$2"
    if ! echo "${result}" | grep -q -e "Ok" -e "ok"; then
        echo "Error: ${context}" >&2; echo "${result}" >&2; exit 1
    fi
}

echo "=============== SOL/USDT ASYNC ADD LIQUIDITY ==============="
echo "Network: ${NETWORK}"
echo "SOL Amount: ${SOL_AMOUNT}"
echo "USDT Amount: ${USDT_AMOUNT}"
echo "USDT Symbol: ${USDT_SYMBOL}"
echo "=========================================================="

# --- 0. Setup ---
echo
echo "--- 0. Setup ---"
KONG_SOL_RAW=$(dfx canister call ${NETWORK_FLAG} ${KONG_BACKEND} get_solana_address --output json)
check_ok "${KONG_SOL_RAW}" "Failed to get Kong Solana address"
KONG_SOL_ADDR=$(echo "${KONG_SOL_RAW}" | jq -r '.Ok')
echo "Kong Solana Address: ${KONG_SOL_ADDR}"

# --- 1. Transfer SOL to Kong ---
echo
echo "--- 1. Transferring SOL to Kong ---"
SOL_DEC=$(bc <<< "scale=9; ${SOL_AMOUNT}/1000000000")
echo "Transferring ${SOL_DEC} SOL..."
TX_OUT=$(solana transfer --allow-unfunded-recipient "${KONG_SOL_ADDR}" "${SOL_DEC}")
SOL_TX_SIG=$(echo "${TX_OUT}" | grep -o 'Signature: .*' | awk '{print $2}')
echo "SOL transferred. Tx: ${SOL_TX_SIG}"

# --- 2. Approve USDT ---
echo
echo "--- 2. Approving USDT ---"
APPROVE_AMOUNT=$((USDT_AMOUNT+USDT_FEE))
echo "Approving ${APPROVE_AMOUNT} ${USDT_SYMBOL} for Kong backend..."
APR=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${USDT_LEDGER} icrc2_approve "(record { amount = ${APPROVE_AMOUNT}; spender = record { owner = principal \"${KONG_BACKEND}\" }; })")
check_ok "${APR}" "USDT approve failed"
echo "USDT approved successfully"

# --- 3. Sign message ---
echo
echo "--- 3. Signing message ---"
# Create the canonical message format that Kong expects for add_liquidity
MSG=$(printf '{"token_0":"%s.%s","amount_0":"%s","token_1":"%s.%s","amount_1":"%s"}' \
  "${SOL_CHAIN}" "${SOL_ADDRESS}" "${SOL_AMOUNT}" \
  "${USDT_CHAIN}" "${USDT_LEDGER}" "${USDT_AMOUNT}")

echo "Message to sign:"
echo "${MSG}"
echo "---"
SIG=$(solana sign-offchain-message "${MSG}")
echo "Message signed"

# --- 4. Execute async add liquidity ---
echo
echo "--- 4. Executing async add liquidity ---"

REQUEST_ID=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} add_liquidity_async "(record {
    token_0 = \"${SOL_CHAIN}.${SOL_ADDRESS}\";
    amount_0 = ${SOL_AMOUNT};
    tx_id_0 = opt variant { TransactionId = \"${SOL_TX_SIG}\" };
    token_1 = \"${USDT_CHAIN}.${USDT_LEDGER}\";
    amount_1 = ${USDT_AMOUNT};
    tx_id_1 = null;
    signature_0 = opt \"${SIG}\";
    signature_1 = null;
})")

echo "Async add liquidity request submitted!"
echo "Request ID: ${REQUEST_ID}"
REQUEST_ID_NUM=$(echo "${REQUEST_ID}" | grep -o '[0-9]\+' | head -1)

for i in {1..20}; do
    REQUEST_STATUS=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} requests "(opt ${REQUEST_ID_NUM})")
    STATUSES=$(echo "${REQUEST_STATUS}" | grep -A 10 'statuses = vec' | sed '/statuses = vec/,/}/!d')
    echo "Poll #${i}:"
    echo "${STATUSES}"
    
    if echo "${STATUSES}" | grep -q "LP token sent"; then
        break
    fi
    sleep 3
done