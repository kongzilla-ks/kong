#!/usr/bin/env bash
set -euo pipefail

# swaps SOL to USDC
# usage: sh swap_sol_to_usdc.sh [local|ic]

# ============================ CONFIG ============================
NETWORK="${1:-local}"                   # "local" or "ic"
IDENTITY_FLAG="--identity kong_user1"

# Pay Token (SOL)
PAY_TOKEN="SOL"
SOL_CHAIN="SOL"
SOL_ADDRESS="11111111111111111111111111111111"  # Native SOL
PAY_AMOUNT=10000000          # 0.01 SOL (9 decimals)

# Receive Token (USDC on Solana)
RECEIVE_TOKEN="USDC"
USDC_CHAIN="SOL"
if [ "${NETWORK}" == "ic" ]; then
    USDC_ADDRESS="EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"  # Mainnet USDC
else
    USDC_ADDRESS="4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"  # Devnet USDC
fi
RECEIVE_AMOUNT=0             # Let system calculate optimal amount
MAX_SLIPPAGE=95.0            # 95% - high slippage for testing
# ===============================================================

NETWORK_FLAG=$([ "${NETWORK}" == "local" ] && echo "" || echo "--network ${NETWORK}")
KONG_BACKEND=$(dfx canister id ${NETWORK_FLAG} kong_backend)

# --- Helper to check for command success ---
check_ok() {
    local result="$1"; local context="$2"
    if ! echo "${result}" | grep -q -e "Ok" -e "ok"; then
        echo "Error: ${context}" >&2; echo "${result}" >&2; exit 1
    fi
}

echo "=============== SOL to USDC SWAP ==============="
echo "Network: ${NETWORK}"
echo "Pay Token: ${PAY_TOKEN}"
echo "Pay Amount: ${PAY_AMOUNT}"
echo "Receive Token: ${RECEIVE_TOKEN}"
echo "Max Slippage: ${MAX_SLIPPAGE}%"
echo "==============================================="

# --- 0. Setup: Get addresses ---
echo
echo "--- 0. Setup ---"
KONG_SOLANA_ADDRESS_RAW=$(dfx canister call ${NETWORK_FLAG} ${KONG_BACKEND} get_solana_address --output json)
check_ok "${KONG_SOLANA_ADDRESS_RAW}" "Failed to get Kong Solana address"
KONG_SOLANA_ADDRESS=$(echo "${KONG_SOLANA_ADDRESS_RAW}" | jq -r '.Ok')
USER_SOLANA_ADDRESS=$(solana address)
echo "Kong Solana address: ${KONG_SOLANA_ADDRESS}"
echo "User Solana address: ${USER_SOLANA_ADDRESS}"

# --- 1. Get swap amounts quote ---
echo
echo "--- 1. Getting swap quote ---"
SWAP_QUOTE=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} swap_amounts "(\"${PAY_TOKEN}\", ${PAY_AMOUNT}, \"${RECEIVE_TOKEN}\")")
echo "Swap quote: ${SWAP_QUOTE}"

# --- 2. Transfer SOL to Kong ---
echo
echo "--- 2. Transferring SOL to Kong ---"
SOL_DEC=$(bc <<< "scale=9; ${PAY_AMOUNT} / 1000000000")
echo "Transferring ${SOL_DEC} SOL..."
TRANSFER_OUTPUT=$(solana transfer --allow-unfunded-recipient "${KONG_SOLANA_ADDRESS}" "${SOL_DEC}")
SOL_TX_SIG=$(echo "${TRANSFER_OUTPUT}" | grep -o 'Signature: .*' | awk '{print $2}')
echo "SOL transferred. Tx: ${SOL_TX_SIG}"
echo "Waiting for kong_rpc processing..."
sleep 10

# --- 3. Sign message ---
echo
echo "--- 3. Signing message ---"
MESSAGE_JSON=$(printf '{"pay_token":"%s","pay_amount":%s,"pay_address":"%s","receive_token":"%s","receive_amount":%s,"receive_address":"%s","max_slippage":%s,"referred_by":null}' \
    "${PAY_TOKEN}" "${PAY_AMOUNT}" "${USER_SOLANA_ADDRESS}" \
    "${RECEIVE_TOKEN}" "${RECEIVE_AMOUNT}" "${USER_SOLANA_ADDRESS}" \
    "${MAX_SLIPPAGE}")
SIGNATURE=$(solana sign-offchain-message "${MESSAGE_JSON}")
echo "Message signed"

# --- 4. Execute swap ---
echo
echo "--- 4. Executing swap ---"
SWAP_RESULT=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} swap --output json "(record {
    pay_token = \"${PAY_TOKEN}\";
    pay_amount = ${PAY_AMOUNT};
    pay_tx_id = opt variant { TransactionId = \"${SOL_TX_SIG}\" };
    receive_token = \"${RECEIVE_TOKEN}\";
    receive_amount = opt ${RECEIVE_AMOUNT};
    max_slippage = opt ${MAX_SLIPPAGE};
    receive_address = opt \"${USER_SOLANA_ADDRESS}\";
    pay_signature = opt \"${SIGNATURE}\";
})")
check_ok "${SWAP_RESULT}" "Swap failed"

echo "Swap completed successfully!"
echo "${SWAP_RESULT}"

# --- 5. Check balances ---
echo
echo "--- 5. Checking balances ---"
echo "SOL balance:"
solana balance
echo "USDC balance:"
spl-token balance ${USDC_ADDRESS} || echo "No USDC balance found"