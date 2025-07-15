#!/usr/bin/env bash
set -euo pipefail

# swaps USDT to SOL
# usage: sh swap_usdt_to_sol.sh [local|ic]

# ============================ CONFIG ============================
NETWORK="${1:-local}"                   # "local" or "ic"
IDENTITY_FLAG="--identity kong_user1"

# CANISTER IDS
MAINNET_KONG_BACKEND="u6kfa-6aaaa-aaaam-qdxba-cai"
LOCAL_KONG_BACKEND="kong_backend"  # Will use dfx canister id locally

# Pay Token (USDT on IC)
PAY_TOKEN_CHAIN="IC"
USDT_SYMBOL=$([ "${NETWORK}" == "local" ] && echo "ksUSDT" || echo "ckUSDT")
PAY_TOKEN="${USDT_SYMBOL}"
PAY_AMOUNT=10000000          # 10 USDT (6 decimals)

# Receive Token (SOL)
RECEIVE_TOKEN="SOL"
RECEIVE_AMOUNT=0             # Let system calculate optimal amount
MAX_SLIPPAGE=95.0            # 95%
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

echo "=============== ${USDT_SYMBOL} to SOL SWAP ==============="
echo "Network: ${NETWORK}"
echo "Pay Token: ${PAY_TOKEN}"
echo "Pay Amount: ${PAY_AMOUNT}"
echo "Receive Token: ${RECEIVE_TOKEN}"
echo "Max Slippage: ${MAX_SLIPPAGE}%"
echo "=================================================="

# --- 1. Get swap amounts quote ---
echo
echo "--- 1. Getting swap quote ---"
SWAP_QUOTE=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} swap_amounts "(\"${PAY_TOKEN}\", ${PAY_AMOUNT}, \"${RECEIVE_TOKEN}\")")
echo "Swap quote: ${SWAP_QUOTE}"

# --- 2. Calculate fee and approve spending ---
echo
echo "--- 2. Approving ${USDT_SYMBOL} spending ---"
FEE=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${USDT_LEDGER} icrc1_fee "()" | awk -F'[:]+' '{print $1}' | awk '{gsub(/\(/, ""); print}')
FEE=${FEE//_/}
APPROVE_AMOUNT=$((PAY_AMOUNT + FEE))
APPROVE_RESULT=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${USDT_LEDGER} icrc2_approve "(record {
    amount = ${APPROVE_AMOUNT};
    spender = record { owner = principal \"${KONG_BACKEND}\" };
})")
check_ok "${APPROVE_RESULT}" "${USDT_SYMBOL} approval failed"

# --- 3. Get Solana address ---
echo
echo "--- 3. Getting Solana address ---"
SOLANA_ADDRESS=$(solana address)
echo "Solana address: ${SOLANA_ADDRESS}"

# --- 4. Execute swap ---
echo
echo "--- 4. Executing swap ---"
SWAP_RESULT=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} swap "(record {
    pay_token = \"${PAY_TOKEN}\";
    pay_amount = ${PAY_AMOUNT};
    receive_token = \"${RECEIVE_TOKEN}\";
    receive_amount = opt ${RECEIVE_AMOUNT};
    max_slippage = opt ${MAX_SLIPPAGE};
    receive_address = opt \"${SOLANA_ADDRESS}\";
})")
check_ok "${SWAP_RESULT}" "Swap failed"

echo "Swap completed successfully!"
echo "${SWAP_RESULT}"