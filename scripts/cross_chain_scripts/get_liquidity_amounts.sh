#!/usr/bin/env bash
set -euo pipefail

# Get required liquidity amounts for adding to SOL/ckUSDT pool
# Usage:
#   ./get_liquidity_amounts.sh <sol_amount>         # Calculate ckUSDT needed for X SOL
#   ./get_liquidity_amounts.sh <usdt_amount> usdt   # Calculate SOL needed for X ckUSDT

# ============================ CONFIG ============================
KONG_BACKEND="dexls-paaaa-aaaau-acyiq-cai"  # Staging backend
NETWORK_FLAG="--network ic"
IDENTITY_FLAG="--identity glad"

# Tokens
SOL_TOKEN="SOL.11111111111111111111111111111111"
CKUSDT_TOKEN="IC.cngnf-vqaaa-aaaar-qag4q-cai"

SOL_DECIMALS=9
USDT_DECIMALS=6
# ===============================================================

# Parse arguments
AMOUNT="${1:-0.002}"
MODE="${2:-sol}"  # "sol" or "usdt"

# Convert to raw amount with decimals
if [ "$MODE" = "usdt" ]; then
    # User provided ckUSDT amount, calculate SOL needed
    RAW_AMOUNT=$(bc <<< "scale=0; ${AMOUNT} * 10^${USDT_DECIMALS} / 1")
    TOKEN_0="${CKUSDT_TOKEN}"
    TOKEN_1="${SOL_TOKEN}"
    INPUT_TOKEN="ckUSDT"
    OUTPUT_TOKEN="SOL"
    OUTPUT_DECIMALS=${SOL_DECIMALS}
else
    # User provided SOL amount, calculate ckUSDT needed
    RAW_AMOUNT=$(bc <<< "scale=0; ${AMOUNT} * 10^${SOL_DECIMALS} / 1")
    TOKEN_0="${SOL_TOKEN}"
    TOKEN_1="${CKUSDT_TOKEN}"
    INPUT_TOKEN="SOL"
    OUTPUT_TOKEN="ckUSDT"
    OUTPUT_DECIMALS=${USDT_DECIMALS}
fi

echo "================================================"
echo "Querying staging backend for liquidity amounts"
echo "Input: ${AMOUNT} ${INPUT_TOKEN}"
echo "================================================"
echo ""

# Call add_liquidity_amounts
export DFX_WARNING=-mainnet_plaintext_identity
RESULT=$(dfx canister call ${NETWORK_FLAG} ${KONG_BACKEND} add_liquidity_amounts "(\"${TOKEN_0}\", ${RAW_AMOUNT}, \"${TOKEN_1}\")" --output json 2>&1)

echo "Raw result:"
echo "${RESULT}"
echo ""

# Parse the result
if echo "${RESULT}" | grep -q "Ok"; then
    # Extract amounts from the Ok variant (remove underscores from numbers)
    AMOUNT_0=$(echo "${RESULT}" | jq -r '.Ok.amount_0 // .Ok.add_liquidity_amount_0' | tr -d '_')
    AMOUNT_1=$(echo "${RESULT}" | jq -r '.Ok.amount_1 // .Ok.add_liquidity_amount_1' | tr -d '_')

    # Convert to human-readable
    if [ "$MODE" = "usdt" ]; then
        # When input is ckUSDT: amount_0 is ckUSDT, amount_1 is SOL
        USDT_READABLE=$(bc <<< "scale=6; ${AMOUNT_0} / 10^${USDT_DECIMALS}")
        SOL_READABLE=$(bc <<< "scale=9; ${AMOUNT_1} / 10^${SOL_DECIMALS}")
        SOL_RAW="${AMOUNT_1}"
        USDT_RAW="${AMOUNT_0}"
    else
        # When input is SOL: amount_0 is SOL, amount_1 is ckUSDT
        SOL_READABLE=$(bc <<< "scale=9; ${AMOUNT_0} / 10^${SOL_DECIMALS}")
        USDT_READABLE=$(bc <<< "scale=6; ${AMOUNT_1} / 10^${USDT_DECIMALS}")
        SOL_RAW="${AMOUNT_0}"
        USDT_RAW="${AMOUNT_1}"
    fi

    echo "================================================"
    echo "✓ Liquidity Amounts for Pool:"
    echo "================================================"
    echo "SOL:    ${SOL_READABLE} (${SOL_RAW} raw)"
    echo "ckUSDT: ${USDT_READABLE} (${USDT_RAW} raw)"
    echo ""
    echo "Add these amounts together to maintain pool ratio"
    echo "================================================"
else
    echo "Error calculating liquidity amounts:"
    echo "${RESULT}"
    exit 1
fi
