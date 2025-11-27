#!/usr/bin/env bash
set -euo pipefail

# Creates a mainnet SOL/ckUSDT pool on STAGING kong_backend
# Usage: sh add_sol_pool_staging.sh

# ============================ CONFIG ============================
NETWORK="ic"                            # Always mainnet for staging
IDENTITY_FLAG="--identity glad"         # Using glad identity
NETWORK_FLAG="--network ic"

# STAGING CANISTER ID
KONG_BACKEND="dexls-paaaa-aaaau-acyiq-cai"  # Staging backend

# Token 0 (Solana SOL)
SOL_CHAIN="SOL"
SOL_ADDRESS="11111111111111111111111111111111"   # Native SOL mint
SOL_AMOUNT=2000000           # 0.002 SOL (9 decimals)

# Token 1 (ckUSDT on IC - mainnet)
USDT_CHAIN="IC"
USDT_SYMBOL="ckUSDT"
USDT_AMOUNT=1000000          # 1 USDT (6 decimals)
USDT_FEE=10000               # ICRC2 fee
USDT_LEDGER="cngnf-vqaaa-aaaar-qag4q-cai"  # ckUSDT mainnet
# ===============================================================

# Ensure Solana is on mainnet
echo "Ensuring Solana CLI is set to mainnet..."
solana config set --url https://api.mainnet-beta.solana.com

# --- Helper to check for command success ---
check_ok() {
    local result="$1"; local context="$2"
    if ! echo "${result}" | grep -q -e "Ok" -e "ok"; then
        echo "Error: ${context}" >&2; echo "${result}" >&2; exit 1
    fi
}

# --- 0. Setup: Fetch addresses ---
echo "Fetching Kong's Solana address from staging backend..."
KONG_SOLANA_ADDRESS_RAW=$(dfx canister call ${NETWORK_FLAG} ${KONG_BACKEND} get_solana_address --output json)
KONG_SOLANA_ADDRESS=$(echo "${KONG_SOLANA_ADDRESS_RAW}" | jq -r '.')
echo "Kong Solana Address: ${KONG_SOLANA_ADDRESS}"

# --- 1. Transfer SOL to Kong ---
SOL_DEC=$(bc <<< "scale=9; ${SOL_AMOUNT} / 1000000000")
echo "Transferring ${SOL_DEC} SOL to Kong..."
TRANSFER_OUTPUT=$(solana transfer --allow-unfunded-recipient "${KONG_SOLANA_ADDRESS}" "${SOL_DEC}")
SOL_TX_SIG=$(echo "${TRANSFER_OUTPUT}" | grep -o 'Signature: .*' | awk '{print $2}')
echo "SOL transferred. Tx: ${SOL_TX_SIG}"

# --- 2. Approve USDT spending ---
APPROVE_AMOUNT=$((USDT_AMOUNT + USDT_FEE))
echo "Approving ${APPROVE_AMOUNT} ckUSDT for Kong..."
APPROVE_RESULT=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${USDT_LEDGER} icrc2_approve "(record { amount = ${APPROVE_AMOUNT}; spender = record { owner = principal \"${KONG_BACKEND}\" }; })")
check_ok "${APPROVE_RESULT}" "ckUSDT approval failed"

# --- 3. Sign message ---
MESSAGE_JSON=$(printf '{"token_0":"%s.%s","amount_0":"%s","token_1":"%s.%s","amount_1":"%s","lp_fee_bps":30}' \
    "${SOL_CHAIN}" "${SOL_ADDRESS}" "${SOL_AMOUNT}" \
    "${USDT_CHAIN}" "${USDT_LEDGER}" "${USDT_AMOUNT}")
echo "Signing message: ${MESSAGE_JSON}"
SIGNATURE=$(solana sign-offchain-message "${MESSAGE_JSON}")
echo "Signature: ${SIGNATURE}"

# --- 4. Create pool with retry logic ---
echo "Creating SOL/ckUSDT pool on staging backend..."
MAX_RETRIES=5
RETRY_DELAY=2

for i in $(seq 1 $MAX_RETRIES); do
    echo "Pool creation attempt $i/$MAX_RETRIES"
    POOL_RESULT_RAW=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} add_pool --output json "(record {
        token_0 = \"${SOL_CHAIN}.${SOL_ADDRESS}\";
        amount_0 = ${SOL_AMOUNT};
        tx_id_0 = opt variant { TransactionId = \"${SOL_TX_SIG}\" };
        token_1 = \"${USDT_CHAIN}.${USDT_LEDGER}\";
        amount_1 = ${USDT_AMOUNT};
        signature_0 = opt \"${SIGNATURE}\";
    })" 2>&1 || true)

    if echo "$POOL_RESULT_RAW" | grep -q -e "Ok" -e "ok"; then
        break
    fi

    if echo "$POOL_RESULT_RAW" | grep -q "TRANSACTION_NOT_READY"; then
        echo "Transaction not ready, waiting..."
        if [ $i -lt $MAX_RETRIES ]; then
            echo "Retrying in $RETRY_DELAY seconds..."
            sleep $RETRY_DELAY
        fi
    else
        echo "Pool creation failed with error: $POOL_RESULT_RAW"
        break
    fi
done

check_ok "${POOL_RESULT_RAW}" "Pool creation failed after $MAX_RETRIES attempts"
POOL_ID=$(echo "${POOL_RESULT_RAW}" | jq -r '.Ok.pool_id // .pool_id // empty')
[[ -z "${POOL_ID}" || "${POOL_ID}" == "0" ]] && echo "Warning: could not parse pool_id" || echo "Pool created! ID: ${POOL_ID}"

# --- Verification ---
echo "Verifying pool creation..."
[[ -n "${POOL_ID}" && "${POOL_ID}" != "0" ]] && dfx canister call ${NETWORK_FLAG} ${KONG_BACKEND} pools | grep "pool_id = ${POOL_ID}" || true

echo ""
echo "===================================="
echo "STAGING SOL/ckUSDT POOL CREATED!"
echo "Backend: ${KONG_BACKEND}"
echo "Pool ID: ${POOL_ID}"
echo "SOL TX: ${SOL_TX_SIG}"
echo "===================================="
