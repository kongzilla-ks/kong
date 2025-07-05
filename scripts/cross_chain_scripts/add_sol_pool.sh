#!/usr/bin/env bash
set -euo pipefail

# adds a devnet SOL/ksUSDT pool to kong for local development       with: sh add_sol_pool.sh local
# adds a mainnet SOL/ckUSDT pool to kong for mainnet development    with: sh add_sol_pool.sh ic

# make sure solana cli is set for the correct network, we don't change that here 
# local:    solana config set --url https://api.devnet.solana.com
# mainnet:  solana config set --url https://api.mainnet-beta.solana.com

# ============================ CONFIG ============================
NETWORK="${1:-local}"                   # "local" or "ic"
IDENTITY_FLAG="--identity kong_user1"   # change if needed

# Token 0 (Solana)
SOL_CHAIN="SOL"
SOL_ADDRESS="11111111111111111111111111111111"   # Native SOL mint
SOL_AMOUNT=14000000          # 0.014 SOL (9 decimals)

# Token 1 (USDT on IC)
USDT_CHAIN="IC"
USDT_SYMBOL=$([ "${NETWORK}" == "local" ] && echo "ksUSDT" || echo "ckUSDT")
USDT_AMOUNT=1000000          # 1 USDT (6 decimals)
USDT_FEE=10000               # ICRC2 fee
# ckUSDT (ic): cngnf-vqaaa-aaaar-qag4q-cai
# ksUSDT (local): zdzgz-siaaa-aaaar-qaiba-cai
# ===============================================================

NETWORK_FLAG=$([ "${NETWORK}" == "local" ] && echo "" || echo "--network ${NETWORK}")
KONG_BACKEND=$(dfx canister id ${NETWORK_FLAG} kong_backend)
USDT_LEDGER_NAME="$(echo ${USDT_SYMBOL} | tr '[:upper:]' '[:lower:]')_ledger"
USDT_LEDGER=$(dfx canister id ${NETWORK_FLAG} ${USDT_LEDGER_NAME})

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

# --- 1. Transfer SOL to Kong ---
SOL_DEC=$(bc <<< "scale=9; ${SOL_AMOUNT} / 1000000000")
echo "Transferring ${SOL_DEC} SOL to Kong..."
TRANSFER_OUTPUT=$(solana transfer --allow-unfunded-recipient "${KONG_SOLANA_ADDRESS}" "${SOL_DEC}")
SOL_TX_SIG=$(echo "${TRANSFER_OUTPUT}" | grep -o 'Signature: .*' | awk '{print $2}')
echo "SOL transferred. Tx: ${SOL_TX_SIG}"; sleep 5

# --- 2. Approve USDT spending ---
APPROVE_AMOUNT=$((USDT_AMOUNT + USDT_FEE))
APPROVE_RESULT=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${USDT_LEDGER} icrc2_approve "(record { amount = ${APPROVE_AMOUNT}; spender = record { owner = principal \"${KONG_BACKEND}\" }; })")
check_ok "${APPROVE_RESULT}" "${USDT_SYMBOL} approval failed"

# --- 3. Sign message ---
MESSAGE_JSON=$(printf '{"token_0":"%s.%s","amount_0":[%s],"token_1":"%s.%s","amount_1":[%s],"lp_fee_bps":30}' \
    "${SOL_CHAIN}" "${SOL_ADDRESS}" "${SOL_AMOUNT}" \
    "${USDT_CHAIN}" "${USDT_LEDGER}" "${USDT_AMOUNT}")
SIGNATURE=$(solana sign-offchain-message "${MESSAGE_JSON}")

# --- 4. Create pool ---
POOL_RESULT_RAW=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} add_pool --output json "(record {
    token_0 = \"${SOL_CHAIN}.${SOL_ADDRESS}\";
    amount_0 = ${SOL_AMOUNT};
    tx_id_0 = opt variant { TransactionId = \"${SOL_TX_SIG}\" };
    token_1 = \"${USDT_CHAIN}.${USDT_LEDGER}\";
    amount_1 = ${USDT_AMOUNT};
    signature_0 = opt \"${SIGNATURE}\";
})")
check_ok "${POOL_RESULT_RAW}" "Pool creation failed"
POOL_ID=$(echo "${POOL_RESULT_RAW}" | jq -r '.Ok.pool_id // .pool_id // empty')
[[ -z "${POOL_ID}" || "${POOL_ID}" == "0" ]] && echo "Warning: could not parse pool_id" || echo "Pool created! ID: ${POOL_ID}"

# --- Verification ---
[[ -n "${POOL_ID}" && "${POOL_ID}" != "0" ]] && dfx canister call ${NETWORK_FLAG} ${KONG_BACKEND} pools | grep "pool_id = ${POOL_ID}" || true