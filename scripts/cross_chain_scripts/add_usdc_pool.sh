#!/usr/bin/env bash
set -euo pipefail

# adds a devnet USDC/ksUSDT pool to kong for local development        with: sh add_usdc_pool.sh local
# adds a mainnet USDC/ckUSDT pool to kong for mainnet development     with: sh add_usdc_pool.sh ic

# devnet USDC: https://faucet.circle.com/

# make sure solana cli is set for the correct network, we don't change that here
# local:    solana config set --url https://api.devnet.solana.com OR https://devnet.solana.validationcloud.io/v1/2lqKhKl4hu9x55BrLRGMes6VD-tLXXR8PvWpZAd_IH8
# mainnet:  solana config set --url https://api.mainnet-beta.solana.com

# ============================ CONFIG ============================
NETWORK="${1:-local}"                   # "local" or "ic"
IDENTITY_FLAG="--identity glad"   # change if needed

# Token 0 (USDC on Solana)
USDC_CHAIN="SOL"
if [ "${NETWORK}" == "ic" ]; then
    USDC_ADDRESS="EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"   # Mainnet mint
else
    USDC_ADDRESS="4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"   # Devnet mint
fi
USDC_AMOUNT=1000           # 1 USDC (6 decimals)

# Token 1 (USDT on IC)
USDT_CHAIN="IC"
USDT_SYMBOL=$([ "${NETWORK}" == "local" ] && echo "ksUSDT" || echo "ckUSDT")
USDT_AMOUNT=1000000           # 1 USDT (6 decimals)
USDT_FEE=10000
# ckUSDT (ic): cngnf-vqaaa-aaaar-qag4q-cai
# ksUSDT (local): zdzgz-siaaa-aaaar-qaiba-cai
# ===============================================================

NETWORK_FLAG=$([ "${NETWORK}" == "local" ] && echo "" || echo "--network ${NETWORK}")
KONG_BACKEND=$(dfx canister id ${NETWORK_FLAG} kong_backend)
# Use fixed canister IDs for USDT ledger instead of trying to derive from canister_ids
if [ "${NETWORK}" == "ic" ]; then
    USDT_LEDGER="cngnf-vqaaa-aaaar-qag4q-cai"   # ckUSDT mainnet
else
    USDT_LEDGER="zdzgz-siaaa-aaaar-qaiba-cai"   # ksUSDT local/staging
fi

# --- Helper to check for command success ---
check_ok() {
    local result="$1"; local context="$2"
    if ! echo "${result}" | grep -q -e "Ok" -e "ok"; then
        echo "Error: ${context}" >&2; echo "${result}" >&2; exit 1
    fi
}

# --- 0. Setup: Fetch addresses ---
echo "Fetching addresses..."
KONG_SOLANA_ADDRESS_RAW=$(dfx canister call ${NETWORK_FLAG} ${KONG_BACKEND} get_solana_address --output json)
check_ok "${KONG_SOLANA_ADDRESS_RAW}" "Failed to get Kong Solana address"
KONG_SOLANA_ADDRESS=$(echo "${KONG_SOLANA_ADDRESS_RAW}" | jq -r '.Ok')
echo "Kong Solana Address: ${KONG_SOLANA_ADDRESS}"

# --- 1. Transfer USDC to Kong ---
USDC_DEC=$(bc <<< "scale=6; ${USDC_AMOUNT} / 1000000")
echo "Transferring ${USDC_DEC} USDC to Kong..."
TRANSFER_OUTPUT=$(spl-token transfer --allow-unfunded-recipient "${USDC_ADDRESS}" "${USDC_DEC}" "${KONG_SOLANA_ADDRESS}" --fund-recipient)
USDC_TX_SIG=$(echo "${TRANSFER_OUTPUT}" | grep -o 'Signature: .*' | awk '{print $2}')
echo "USDC transferred. Tx: ${USDC_TX_SIG}"; sleep 10

# --- 2. Approve USDT spending ---
APPROVE_AMOUNT=$((USDT_AMOUNT + USDT_FEE))
APPROVE_RESULT=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${USDT_LEDGER} icrc2_approve "(record { amount = ${APPROVE_AMOUNT}; spender = record { owner = principal \"${KONG_BACKEND}\" }; })")
check_ok "${APPROVE_RESULT}" "${USDT_SYMBOL} approval failed"

# --- 3. Sign message ---
TIMESTAMP=$(($(date +%s) * 1000))
MESSAGE_JSON=$(printf '{"token_0":"%s.%s","amount_0":[%s],"token_1":"%s.%s","amount_1":[%s],"lp_fee_bps":30,"timestamp":%s}' \
    "${USDC_CHAIN}" "${USDC_ADDRESS}" "${USDC_AMOUNT}" \
    "${USDT_CHAIN}" "${USDT_LEDGER}" "${USDT_AMOUNT}" \
    "${TIMESTAMP}")
SIGNATURE=$(solana sign-offchain-message "${MESSAGE_JSON}")

# --- 4. Create pool ---
echo "Creating USDC/${USDT_SYMBOL} pool..."
POOL_RESULT_RAW=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} add_pool --output json "(record {
    token_0 = \"${USDC_CHAIN}.${USDC_ADDRESS}\";
    amount_0 = ${USDC_AMOUNT};
    tx_id_0 = opt variant { TransactionId = \"${USDC_TX_SIG}\" };
    token_1 = \"${USDT_CHAIN}.${USDT_LEDGER}\";
    amount_1 = ${USDT_AMOUNT};
    signature_0 = opt \"${SIGNATURE}\";
    timestamp = opt ${TIMESTAMP};
})")
check_ok "${POOL_RESULT_RAW}" "Pool creation failed"
POOL_ID=$(echo "${POOL_RESULT_RAW}" | jq -r '.Ok.pool_id // .pool_id // empty')
[[ -z "${POOL_ID}" || "${POOL_ID}" == "0" ]] && echo "Warning: could not parse pool_id" || echo "Pool created! ID: ${POOL_ID}"

# --- Verification ---
[[ -n "${POOL_ID}" && "${POOL_ID}" != "0" ]] && dfx canister call ${NETWORK_FLAG} ${KONG_BACKEND} pools | grep "pool_id = ${POOL_ID}" || true