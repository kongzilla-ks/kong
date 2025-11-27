#!/usr/bin/env bash
set -euo pipefail

# Add liquidity to SOL/ckUSDT pool on STAGING backend
# Usage: sh add_sol_lp_staging.sh [sol_amount] [usdt_amount]
# Example: sh add_sol_lp_staging.sh 0.01 5

# ============================ CONFIG ============================
KONG_BACKEND="dexls-paaaa-aaaau-acyiq-cai"  # Staging backend
NETWORK_FLAG="--network ic"
IDENTITY_FLAG="--identity glad"

# Token 0 (Solana - SOL)
SOL_CHAIN="SOL"
SOL_ADDRESS="11111111111111111111111111111111"   # Native SOL mint
SOL_DECIMALS=9

# Token 1 (ckUSDT on IC)
USDT_CHAIN="IC"
USDT_SYMBOL="ckUSDT"
USDT_LEDGER="cngnf-vqaaa-aaaar-qag4q-cai"  # ckUSDT mainnet
USDT_DECIMALS=6
USDT_FEE=10000
# ===============================================================

# Get amounts from arguments or use defaults
SOL_AMOUNT_READABLE="${1:-0.01}"
USDT_AMOUNT_READABLE="${2:-5}"

# Convert to raw amounts
SOL_AMOUNT=$(bc <<< "scale=0; ${SOL_AMOUNT_READABLE} * 10^${SOL_DECIMALS} / 1")
USDT_AMOUNT=$(bc <<< "scale=0; ${USDT_AMOUNT_READABLE} * 10^${USDT_DECIMALS} / 1")

echo "================================================"
echo "Adding Liquidity to SOL/ckUSDT Pool (STAGING)"
echo "================================================"
echo "SOL:    ${SOL_AMOUNT_READABLE} (${SOL_AMOUNT} raw)"
echo "ckUSDT: ${USDT_AMOUNT_READABLE} (${USDT_AMOUNT} raw)"
echo ""

# Ensure mainnet
solana config set --url https://api.mainnet-beta.solana.com > /dev/null

# --- Helper ---
check_ok() {
    local r="$1"; local ctx="$2";
    echo "$r" | grep -q -e "Ok" -e "ok" || {
        echo "Error: $ctx" >&2;
        echo "$r" >&2;
        exit 1;
    }
}

# --- 0. Setup ---
KONG_SOL_RAW=$(dfx canister call ${NETWORK_FLAG} ${KONG_BACKEND} get_solana_address --output json)
KONG_SOL_ADDR=$(echo "$KONG_SOL_RAW" | jq -r '.')
echo "Kong Solana Address: $KONG_SOL_ADDR"
echo ""

# --- 1. Transfer SOL ---
echo "Step 1: Transferring ${SOL_AMOUNT_READABLE} SOL to Kong..."
SOL_DEC=$(bc <<< "scale=9; ${SOL_AMOUNT}/1000000000")
TX_OUT=$(solana transfer --allow-unfunded-recipient "$KONG_SOL_ADDR" "$SOL_DEC")
SOL_TX_SIG=$(echo "$TX_OUT" | grep -o 'Signature: .*' | awk '{print $2}')
echo "✓ SOL transferred. Tx: ${SOL_TX_SIG}"
echo ""

# --- 2. Approve USDT ---
echo "Step 2: Approving ${USDT_AMOUNT_READABLE} ckUSDT..."
APPROVE_AMOUNT=$((USDT_AMOUNT+USDT_FEE))
export DFX_WARNING=-mainnet_plaintext_identity
APR=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${USDT_LEDGER} icrc2_approve "(record { amount = ${APPROVE_AMOUNT}; spender = record { owner = principal \"${KONG_BACKEND}\" }; })")
check_ok "$APR" "ckUSDT approval failed"
echo "✓ ckUSDT approved"
echo ""

# --- 3. Sign message ---
echo "Step 3: Signing liquidity message..."
MSG=$(printf '{"token_0":"%s.%s","amount_0":"%s","token_1":"%s.%s","amount_1":"%s"}' \
  "$SOL_CHAIN" "$SOL_ADDRESS" "$SOL_AMOUNT" \
  "$USDT_CHAIN" "$USDT_LEDGER" "$USDT_AMOUNT")
echo "Message: $MSG"
SIG=$(solana sign-offchain-message "$MSG")
echo "Signature: $SIG"
echo ""

# --- 4. Add liquidity with retry logic ---
echo "Step 4: Adding liquidity to pool..."
CALL="(record { token_0 = \"${SOL_CHAIN}.${SOL_ADDRESS}\"; amount_0=${SOL_AMOUNT}; tx_id_0 = opt variant { TransactionId = \"${SOL_TX_SIG}\" }; token_1 = \"${USDT_CHAIN}.${USDT_LEDGER}\"; amount_1=${USDT_AMOUNT}; tx_id_1 = null; signature_0 = opt \"${SIG}\"; signature_1 = null; })"

MAX_RETRIES=5
RETRY_DELAY=2

for i in $(seq 1 $MAX_RETRIES); do
    echo "Attempt $i/$MAX_RETRIES..."
    RESULT=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} add_liquidity --output json "$CALL" 2>&1 || true)

    if echo "$RESULT" | grep -q -e "Ok" -e "ok"; then
        break
    fi

    if echo "$RESULT" | grep -q "TRANSACTION_NOT_READY"; then
        echo "Transaction not ready, waiting..."
        if [ $i -lt $MAX_RETRIES ]; then
            echo "Retrying in $RETRY_DELAY seconds..."
            sleep $RETRY_DELAY
        fi
    else
        echo "Add liquidity failed: $RESULT"
        break
    fi
done

check_ok "$RESULT" "Add liquidity failed after $MAX_RETRIES attempts"

echo ""
echo "================================================"
echo "✓ LIQUIDITY ADDED TO STAGING POOL!"
echo "================================================"
echo "$RESULT" | jq '.'
echo ""
echo "Pool balances updated on staging backend:"
echo "Backend: ${KONG_BACKEND}"
echo "================================================"
