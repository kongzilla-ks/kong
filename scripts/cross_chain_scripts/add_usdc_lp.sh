#!/usr/bin/env bash
set -euo pipefail

# adds liquidity to existing USDC/USDT pool
# usage: sh add_lp_usdc.sh [local|ic]
# ============================ CONFIG ============================
NETWORK="${1:-local}"
IDENTITY_FLAG="--identity kong_user1"

# CANISTER IDS
MAINNET_KONG_BACKEND="u6kfa-6aaaa-aaaam-qdxba-cai"
LOCAL_KONG_BACKEND="kong_backend"  # Will use dfx canister id locally

# Token 0 (USDC on Solana)
USDC_CHAIN="SOL"
if [ "${NETWORK}" == "ic" ]; then
    USDC_ADDRESS="EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
else
    USDC_ADDRESS="4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
fi
USDC_AMOUNT=1000000        # 1 USDC (6 decimals)
# Token 1 (USDT on IC)
USDT_CHAIN="IC"
USDT_SYMBOL=$([ "${NETWORK}" == "local" ] && echo "ksUSDT" || echo "ckUSDT")
USDT_AMOUNT=1000000        # 1 USDT (6 decimals)
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

# Force Solana to use devnet for local testing
if [ "$NETWORK" = "local" ]; then
    echo "Switching Solana config to devnet for local testing..."
    solana config set --url devnet
fi

check_ok(){ local r="$1"; local c="$2"; echo "$r" | grep -q -e "Ok" -e "ok" || { echo "Error: $c" >&2; echo "$r" >&2; exit 1; }; }
# 0. Fetch Kong Solana address
KONG_SOL_RAW=$(dfx canister call ${NETWORK_FLAG} ${KONG_BACKEND} get_solana_address --output json)
KONG_SOL_ADDR=$(echo "$KONG_SOL_RAW" | jq -r '.')

# 1. Transfer USDC
USDC_DEC=$(bc <<< "scale=6; ${USDC_AMOUNT}/1000000")
TX_OUT=$(spl-token transfer --allow-unfunded-recipient "${USDC_ADDRESS}" "${USDC_DEC}" "${KONG_SOL_ADDR}" --fund-recipient)
USDC_TX_SIG=$(echo "$TX_OUT" | grep -o 'Signature: .*' | awk '{print $2}')

echo "Transferred $USDC_DEC USDC (tx $USDC_TX_SIG)"
echo "hi"

# 2. Approve USDT
APPROVE_AMOUNT=$((USDT_AMOUNT+USDT_FEE))
APR=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${USDT_LEDGER} icrc2_approve "(record { amount = ${APPROVE_AMOUNT}; spender = record { owner = principal \"${KONG_BACKEND}\" }; })")
check_ok "$APR" "USDT approve failed"

# 3. Sign message
MSG=$(printf '{"token_0":"%s.%s","amount_0":"%s","token_1":"%s.%s","amount_1":"%s"}' \
 "$USDC_CHAIN" "$USDC_ADDRESS" "$USDC_AMOUNT" \
 "$USDT_CHAIN" "$USDT_LEDGER" "$USDT_AMOUNT")
SIG=$(solana sign-offchain-message "$MSG")

# 4. Add liquidity with retry logic
CALL="(record { token_0 = \"${USDC_CHAIN}.${USDC_ADDRESS}\"; amount_0=${USDC_AMOUNT}; tx_id_0=opt variant { TransactionId = \"${USDC_TX_SIG}\" }; token_1 = \"${USDT_CHAIN}.${USDT_LEDGER}\"; amount_1=${USDT_AMOUNT}; tx_id_1 = null; signature_0 = opt \"${SIG}\"; signature_1 = null; })"
MAX_RETRIES=5
RETRY_DELAY=2

for i in $(seq 1 $MAX_RETRIES); do
    echo "Add liquidity attempt $i/$MAX_RETRIES"
    RES=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} add_liquidity --output json "$CALL" 2>&1 || true)
    
    if echo "$RES" | grep -q -e "Ok" -e "ok"; then
        break
    fi
    
    if echo "$RES" | grep -q "TRANSACTION_NOT_READY"; then
        echo "Transaction not ready, waiting..."
        if [ $i -lt $MAX_RETRIES ]; then
            echo "Retrying in $RETRY_DELAY seconds..."
            sleep $RETRY_DELAY
        fi
    else
        echo "Add liquidity failed with error: $RES"
        break
    fi
done

check_ok "$RES" "add_liquidity failed after $MAX_RETRIES attempts"
RID=$(echo "$RES" | jq -r '.Ok.request_id // .request_id // empty')
[[ -n "$RID" ]] && echo "Liquidity add request submitted: $RID" || echo "$RES"
