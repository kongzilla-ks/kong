#!/bin/bash

# Double spend test - try to use same Solana transaction signature 3 times
# This should show that only the first call succeeds, others fail

set -e

# Mainnet configuration
KONG_BACKEND="u6kfa-6aaaa-aaaam-qdxba-cai"
IDENTITY_FLAG="--identity default"

echo "DOUBLE SPEND TEST - Testing duplicate Solana transaction protection"
echo "Network: ic (mainnet)"
echo "Kong Backend: $KONG_BACKEND"
echo ""

# Swap parameters
PAY_TOKEN="SOL"
PAY_AMOUNT=1000000  # 0.001 SOL (9 decimals)
RECEIVE_TOKEN="SOL.USDC"
MAX_SLIPPAGE=95.0

# Helper function to check for success
check_ok() {
    local result="$1"; local context="$2"
    if ! echo "${result}" | grep -q -e "Ok" -e "ok"; then
        echo "Error: ${context}" >&2; echo "${result}" >&2; exit 1
    fi
}

echo "=============== SETUP PHASE ==============="
echo "Pay Token: ${PAY_TOKEN}"
echo "Pay Amount: ${PAY_AMOUNT}"
echo "Receive Token: ${RECEIVE_TOKEN}"
echo "Max Slippage: ${MAX_SLIPPAGE}%"
echo "============================================"

# --- 0. Setup: Get addresses ---
echo
echo "--- 0. Getting addresses ---"
KONG_SOLANA_ADDRESS_RAW=$(dfx canister call --network ic $KONG_BACKEND get_solana_address --output json)
KONG_SOLANA_ADDRESS=$(echo "${KONG_SOLANA_ADDRESS_RAW}" | jq -r '.')
USER_SOLANA_ADDRESS=$(solana address)
echo "Kong Solana address: $KONG_SOLANA_ADDRESS"
echo "User Solana address: $USER_SOLANA_ADDRESS"

# --- 1. Get swap amounts quote ---
echo
echo "--- 1. Getting swap quote ---"
SWAP_QUOTE=$(dfx canister call --network ic $IDENTITY_FLAG $KONG_BACKEND swap_amounts "(\"${PAY_TOKEN}\", ${PAY_AMOUNT}, \"${RECEIVE_TOKEN}\")")
echo "Swap quote: ${SWAP_QUOTE}"
check_ok "${SWAP_QUOTE}" "Swap quote failed"

# Extract receive_amount from the swap quote
RECEIVE_AMOUNT=$(echo "${SWAP_QUOTE}" | grep -o 'receive_amount = [0-9_]*' | tail -1 | sed 's/receive_amount = //' | tr -d '_')
if [ -z "${RECEIVE_AMOUNT}" ]; then
    echo "Error: Could not extract receive_amount from swap quote"
    exit 1
fi
echo "Expected receive amount: $RECEIVE_AMOUNT"

# --- 2. Transfer SOL to Kong ---
echo
echo "--- 2. Transferring SOL to Kong ---"
SOL_DEC=$(bc <<< "scale=9; ${PAY_AMOUNT} / 1000000000")
echo "Transferring ${SOL_DEC} SOL..."
TRANSFER_OUTPUT=$(solana transfer --allow-unfunded-recipient "${KONG_SOLANA_ADDRESS}" "${SOL_DEC}")
SOL_TX_SIG=$(echo "${TRANSFER_OUTPUT}" | grep -o 'Signature: .*' | awk '{print $2}')
echo "SOL transferred. Tx: ${SOL_TX_SIG}"

# --- 3. Sign message ---
echo
echo "--- 3. Signing message ---"
# Create the canonical message format that Kong expects
MESSAGE=$(printf '{"pay_token":"%s","pay_amount":"%s","pay_address":"%s","receive_token":"%s","receive_amount":"%s","receive_address":"%s","max_slippage":%.1f,"referred_by":null}' \
  "${PAY_TOKEN}" "${PAY_AMOUNT}" "${USER_SOLANA_ADDRESS}" "${RECEIVE_TOKEN}" "${RECEIVE_AMOUNT}" "${USER_SOLANA_ADDRESS}" "${MAX_SLIPPAGE}")

echo "Message to sign: $MESSAGE"
SIGNATURE=$(solana sign-offchain-message "${MESSAGE}")
echo "Message signed"
echo ""

echo "=============== ATTACK PHASE ==============="
echo "Rapidly firing 3 identical swap_async calls..."
echo "============================================="

# --- 4. Execute 3 rapid swap_async calls with same signature ---
echo
echo "Attempt 1/3 - First call (should succeed)"
RESULT1=$(dfx canister call --network ic $IDENTITY_FLAG $KONG_BACKEND swap_async "(
    record {
        pay_token = \"${PAY_TOKEN}\";
        pay_amount = ${PAY_AMOUNT};
        pay_tx_id = opt variant { TransactionId = \"${SOL_TX_SIG}\" };
        receive_token = \"${RECEIVE_TOKEN}\";
        receive_amount = opt ${RECEIVE_AMOUNT};
        max_slippage = opt ${MAX_SLIPPAGE};
        receive_address = opt \"${USER_SOLANA_ADDRESS}\";
        pay_signature = opt \"${SIGNATURE}\";
    }
)" 2>&1 || true)

echo "Result 1: $RESULT1"
echo ""

echo "Attempt 2/3 - Second call (should fail with duplicate signature)"
RESULT2=$(dfx canister call --network ic $IDENTITY_FLAG $KONG_BACKEND swap_async "(
    record {
        pay_token = \"${PAY_TOKEN}\";
        pay_amount = ${PAY_AMOUNT};
        pay_tx_id = opt variant { TransactionId = \"${SOL_TX_SIG}\" };
        receive_token = \"${RECEIVE_TOKEN}\";
        receive_amount = opt ${RECEIVE_AMOUNT};
        max_slippage = opt ${MAX_SLIPPAGE};
        receive_address = opt \"${USER_SOLANA_ADDRESS}\";
        pay_signature = opt \"${SIGNATURE}\";
    }
)" 2>&1 || true)

echo "Result 2: $RESULT2"
echo ""

echo "Attempt 3/3 - Third call (should fail with duplicate signature)"
RESULT3=$(dfx canister call --network ic $IDENTITY_FLAG $KONG_BACKEND swap_async "(
    record {
        pay_token = \"${PAY_TOKEN}\";
        pay_amount = ${PAY_AMOUNT};
        pay_tx_id = opt variant { TransactionId = \"${SOL_TX_SIG}\" };
        receive_token = \"${RECEIVE_TOKEN}\";
        receive_amount = opt ${RECEIVE_AMOUNT};
        max_slippage = opt ${MAX_SLIPPAGE};
        receive_address = opt \"${USER_SOLANA_ADDRESS}\";
        pay_signature = opt \"${SIGNATURE}\";
    }
)" 2>&1 || true)

echo "Result 3: $RESULT3"
echo ""

echo "=============== ANALYSIS ==============="
echo "SUMMARY:"
echo "========"
if echo "$RESULT1" | grep -q "Ok\|ok"; then
    echo "✅ Attempt 1: SUCCESS (as expected - legitimate transaction)"
    REQUEST_ID1=$(echo "$RESULT1" | grep -o '[0-9]\+' | head -1)
    echo "   Request ID: $REQUEST_ID1"
else
    echo "❌ Attempt 1: FAILED (unexpected - should succeed)"
fi

if echo "$RESULT2" | grep -q "already used\|duplicate\|Solana transaction signature already used"; then
    echo "✅ Attempt 2: BLOCKED (duplicate protection working)"
elif echo "$RESULT2" | grep -q "Ok\|ok"; then
    echo "🚨 Attempt 2: SUCCESS (DOUBLE SPEND VULNERABILITY!)"
    REQUEST_ID2=$(echo "$RESULT2" | grep -o '[0-9]\+' | head -1)
    echo "   Request ID: $REQUEST_ID2"
else
    echo "⚠️  Attempt 2: FAILED (different error)"
fi

if echo "$RESULT3" | grep -q "already used\|duplicate\|Solana transaction signature already used"; then
    echo "✅ Attempt 3: BLOCKED (duplicate protection working)"
elif echo "$RESULT3" | grep -q "Ok\|ok"; then
    echo "🚨 Attempt 3: SUCCESS (DOUBLE SPEND VULNERABILITY!)"
    REQUEST_ID3=$(echo "$RESULT3" | grep -o '[0-9]\+' | head -1)
    echo "   Request ID: $REQUEST_ID3"
else
    echo "⚠️  Attempt 3: FAILED (different error)"
fi

echo ""
echo "Transaction signature used: $SOL_TX_SIG"
echo "To inspect requests, check the request IDs:"
echo "Example: dfx canister call --network ic $KONG_BACKEND requests \"(opt REQUEST_ID)\""