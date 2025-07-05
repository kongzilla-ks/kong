#!/usr/bin/env bash
set -euo pipefail

# removes 10% of liquidity from SOL/USDT pool
# usage: sh remove_sol_lp.sh [local|ic]

# ============================ CONFIG ============================
NETWORK="${1:-local}"
IDENTITY_FLAG="--identity kong_user1"
REMOVE_PERCENTAGE=10  # Remove 10% of LP tokens

# Token configuration
SOL_CHAIN="SOL"
SOL_ADDRESS="11111111111111111111111111111111"
USDT_CHAIN="IC"
USDT_SYMBOL=$([ "${NETWORK}" == "local" ] && echo "ksUSDT" || echo "ckUSDT")
# ===============================================================

NETWORK_FLAG=$([ "${NETWORK}" == "local" ] && echo "" || echo "--network ${NETWORK}")
KONG_BACKEND=$(dfx canister id ${NETWORK_FLAG} kong_backend)
USDT_LEDGER_NAME="$(echo ${USDT_SYMBOL} | tr '[:upper:]' '[:lower:]')_ledger"
USDT_LEDGER=$(dfx canister id ${NETWORK_FLAG} ${USDT_LEDGER_NAME})

# Helper function
check_ok() { local r="$1"; local ctx="$2"; echo "$r" | grep -q -e "Ok" -e "ok" || { echo "Error: $ctx" >&2; echo "$r" >&2; exit 1; }; }

# Get current principal and Solana address
PRINCIPAL=$(dfx identity get-principal ${IDENTITY_FLAG})
SOLANA_ADDRESS=$(solana address)
echo "Principal: $PRINCIPAL"
echo "Solana address: $SOLANA_ADDRESS"

# 1. Check initial balance
echo ""
echo "=== Initial LP Balance ==="
INITIAL_BALANCE_RAW=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} user_balances "(\"$PRINCIPAL\")" --output json)
check_ok "$INITIAL_BALANCE_RAW" "Failed to get user balances"

# Extract SOL_ksUSDT LP balance
LP_BALANCE=$(echo "$INITIAL_BALANCE_RAW" | jq -r '.Ok[] | select(.LP.symbol == "SOL_'"${USDT_SYMBOL}"'") | .LP.balance' 2>/dev/null || echo "0")
if [ "$LP_BALANCE" == "0" ] || [ -z "$LP_BALANCE" ]; then
    echo "No SOL_${USDT_SYMBOL} LP tokens found"
    exit 1
fi

echo "Current SOL_${USDT_SYMBOL} LP balance: $LP_BALANCE"

# 2. Calculate 10% of LP tokens to remove
# The balance is already in decimal format, we need to:
# 1. Calculate 10% of the decimal balance
# 2. Convert to integer units (multiply by 10^8 for LP tokens)
REMOVE_AMOUNT_DEC=$(echo "scale=8; $LP_BALANCE * $REMOVE_PERCENTAGE / 100" | bc)
REMOVE_AMOUNT=$(echo "$REMOVE_AMOUNT_DEC * 100000000" | bc | cut -d'.' -f1)

echo "Removing $REMOVE_PERCENTAGE% = $REMOVE_AMOUNT_DEC LP tokens ($REMOVE_AMOUNT units)"

# 3. Check expected amounts
echo ""
echo "=== Expected Removal Amounts ==="
AMOUNTS_RAW=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} remove_liquidity_amounts "(\"${SOL_CHAIN}.${SOL_ADDRESS}\", \"${USDT_CHAIN}.${USDT_LEDGER}\", $REMOVE_AMOUNT)")
check_ok "$AMOUNTS_RAW" "Failed to get removal amounts"

# Parse amounts using grep and awk
SOL_AMOUNT=$(echo "$AMOUNTS_RAW" | grep -o 'amount_0 = [0-9_]*' | awk '{print $3}' | tr -d '_')
USDT_AMOUNT=$(echo "$AMOUNTS_RAW" | grep -o 'amount_1 = [0-9_]*' | awk '{print $3}' | tr -d '_')
SOL_DEC=$(echo "scale=9; $SOL_AMOUNT / 1000000000" | bc)
USDT_DEC=$(echo "scale=6; $USDT_AMOUNT / 1000000" | bc)

echo "Expected to receive:"
echo "  - SOL: $SOL_DEC ($SOL_AMOUNT units) → to $SOLANA_ADDRESS"
echo "  - ${USDT_SYMBOL}: $USDT_DEC ($USDT_AMOUNT units)"

# 4. Sign message for Solana payout
echo ""
echo "=== Signing Message for Solana Payout ==="
MESSAGE_JSON=$(printf '{"token_0":"%s.%s","token_1":"%s.%s","remove_lp_token_amount":[%s],"payout_address_0":"%s","payout_address_1":null}' \
    "${SOL_CHAIN}" "${SOL_ADDRESS}" \
    "${USDT_CHAIN}" "${USDT_LEDGER}" \
    "${REMOVE_AMOUNT}" \
    "${SOLANA_ADDRESS}")
echo "Message to sign: $MESSAGE_JSON"
SIGNATURE=$(solana sign-offchain-message "$MESSAGE_JSON")
echo "Signature generated"

# 5. Remove liquidity
echo ""
echo "=== Removing Liquidity ==="
REMOVE_RESULT=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} remove_liquidity "(record {
    token_0 = \"${SOL_CHAIN}.${SOL_ADDRESS}\";
    token_1 = \"${USDT_CHAIN}.${USDT_LEDGER}\";
    remove_lp_token_amount = $REMOVE_AMOUNT;
    payout_address_0 = opt \"${SOLANA_ADDRESS}\";
    payout_address_1 = null;
    signature_0 = opt \"${SIGNATURE}\";
    signature_1 = null;
})" --output json)
check_ok "$REMOVE_RESULT" "Remove liquidity failed"

REQUEST_ID=$(echo "$REMOVE_RESULT" | jq -r '.Ok.request_id // .request_id // empty')
echo "Remove liquidity request submitted: $REQUEST_ID"

# Wait for processing
echo "Waiting for transaction to process..."
sleep 5

# 6. Check final balance
echo ""
echo "=== Final LP Balance ==="
FINAL_BALANCE_RAW=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} user_balances "(\"$PRINCIPAL\")" --output json)
check_ok "$FINAL_BALANCE_RAW" "Failed to get final balances"

NEW_LP_BALANCE=$(echo "$FINAL_BALANCE_RAW" | jq -r '.Ok[] | select(.LP.symbol == "SOL_'"${USDT_SYMBOL}"'") | .LP.balance' 2>/dev/null || echo "0")
echo "New SOL_${USDT_SYMBOL} LP balance: $NEW_LP_BALANCE"

# Calculate difference
DIFF=$(echo "$LP_BALANCE - $NEW_LP_BALANCE" | bc -l)
echo "LP tokens removed: $DIFF"
echo "Expected removal: $REMOVE_AMOUNT_DEC"