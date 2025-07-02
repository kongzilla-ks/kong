#!/usr/bin/env bash

# Mainnet SOL/ckUSDT pool creation script
# Creates pool with 0.03 SOL and 4.39 ckUSDT

set -eu

# Configuration - Always mainnet with --ic
KONG_BACKEND="u6kfa-6aaaa-aaaam-qdxba-cai"  # Current Kong backend mainnet canister

# Token configurations
SOL_AMOUNT=1_000_000  # 0.001 SOL (9 decimals)
SOL_AMOUNT=${SOL_AMOUNT//_/}  # remove underscore
SOL_CHAIN="SOL"
SOL_ADDRESS="11111111111111111111111111111111"  # Native SOL

# ckUSDT configuration
CKUSDT_SYMBOL="ckUSDT"
CKUSDT_CHAIN="IC"
CKUSDT_LEDGER="cngnf-vqaaa-aaaar-qag4q-cai"  # ckUSDT mainnet canister
if [ -z "$CKUSDT_LEDGER" ] || [ "$CKUSDT_LEDGER" = "null" ] || [ "$CKUSDT_LEDGER" = "" ]; then
    echo "Error: Could not get ckUSDT ledger canister ID"
    exit 1
fi
CKUSDT_AMOUNT=1_000_000  # 1.0 ckUSDT (6 decimals)
CKUSDT_AMOUNT=${CKUSDT_AMOUNT//_/}
CKUSDT_FEE=10000  # Standard fee

# Get Kong's Solana address
echo "SETUP"
KONG_SOLANA_ADDRESS=$(dfx canister call kong_backend get_solana_address --ic --output json | jq -r '.Ok // .Err')
if [ -z "$KONG_SOLANA_ADDRESS" ] || [ "$KONG_SOLANA_ADDRESS" = "null" ] || echo "$KONG_SOLANA_ADDRESS" | grep -q "Error"; then
    echo "Failed to get Kong Solana address" >&2
    exit 1
fi
echo "Kong Solana address: $KONG_SOLANA_ADDRESS"

# Get user's Solana address
USER_SOLANA_ADDRESS=$(solana address)
echo "User Solana address: $USER_SOLANA_ADDRESS"

# Get user's IC principal (current identity)
USER_IC_PRINCIPAL=$(dfx identity get-principal)
echo "User IC principal: $USER_IC_PRINCIPAL"

# Step 1: Transfer SOL to Kong's address
echo "STEP 1: TRANSFER SOL"
echo "Transferring $(echo "scale=9; $SOL_AMOUNT / 1000000000" | bc) SOL to Kong..."

# Transfer SOL
TRANSFER_OUTPUT=$(solana transfer --allow-unfunded-recipient "$KONG_SOLANA_ADDRESS" $(echo "scale=9; $SOL_AMOUNT / 1000000000" | bc) 2>&1)
SOL_TX_SIG=$(echo "$TRANSFER_OUTPUT" | grep "Signature" | awk '{print $2}' | tr -d '[]"')

if [ -z "$SOL_TX_SIG" ]; then
    echo "SOL transfer failed" >&2
    echo "$TRANSFER_OUTPUT"
    exit 1
fi

echo "SOL transferred!"
echo "Transaction: $SOL_TX_SIG"

# Step 2: Wait for transaction confirmation
echo "STEP 2: WAIT FOR CONFIRMATION"
echo "Waiting for transaction confirmation..."
sleep 5

# Step 3: Approve ckUSDT spending
echo "STEP 3: APPROVE CKUSDT"
EXPIRES_AT=$(echo "($(date +%s) + 300) * 1000000000" | bc)  # 5 minutes from now in nanoseconds
APPROVE_AMOUNT=$((10 * (CKUSDT_AMOUNT + CKUSDT_FEE)))

echo "Approving $APPROVE_AMOUNT ckUSDT..."
APPROVE_RESULT=$(dfx canister call --ic ${CKUSDT_LEDGER} icrc2_approve "(record {
    amount = ${APPROVE_AMOUNT};
    expires_at = opt ${EXPIRES_AT};
    spender = record {
        owner = principal \"${KONG_BACKEND}\";
    };
})" 2>&1)

if echo "$APPROVE_RESULT" | grep -q "Err"; then
    echo "ckUSDT approval failed: $APPROVE_RESULT" >&2
    exit 1
fi
echo "ckUSDT approved!"

# Step 4: Create canonical pool message and sign it
echo "STEP 4: CREATE SIGNATURE"

# Create timestamp (milliseconds)
TIMESTAMP=$(echo "$(date +%s) * 1000" | bc)

# Create canonical pool message
MESSAGE_JSON=$(cat <<EOF
{
  "token_0": "${SOL_CHAIN}.${SOL_ADDRESS}",
  "amount_0": [$SOL_AMOUNT],
  "token_1": "${CKUSDT_CHAIN}.${CKUSDT_LEDGER}",
  "amount_1": [$CKUSDT_AMOUNT],
  "lp_fee_bps": 30,
  "timestamp": $TIMESTAMP
}
EOF
)

MESSAGE_COMPACT=$(echo "$MESSAGE_JSON" | jq -c .)
echo "Signing canonical pool message..."
echo "Message: $MESSAGE_COMPACT"

SIGNATURE=$(solana sign-offchain-message "$MESSAGE_COMPACT" 2>&1)
if [ -z "$SIGNATURE" ] || echo "$SIGNATURE" | grep -q "Error"; then
    echo "Failed to sign message" >&2
    echo "Signature output: $SIGNATURE"
    exit 1
fi

echo "Message signed"

# Step 5: Create the pool with proper cross-chain data
echo "STEP 5: CREATE POOL"
echo "Creating SOL/ckUSDT pool..."

POOL_CALL="(record {
    token_0 = \"${SOL_CHAIN}.${SOL_ADDRESS}\";
    amount_0 = ${SOL_AMOUNT};
    tx_id_0 = opt variant { TransactionId = \"${SOL_TX_SIG}\" };
    token_1 = \"${CKUSDT_CHAIN}.${CKUSDT_LEDGER}\";
    amount_1 = ${CKUSDT_AMOUNT};
    tx_id_1 = null;
    lp_fee_bps = opt 30;
    signature_0 = opt \"${SIGNATURE}\";
    signature_1 = null;
    timestamp = opt ${TIMESTAMP};
})"

echo "Pool call:"
echo "$POOL_CALL"

echo "Submitting pool creation..."
POOL_RESULT=$(dfx canister call --ic ${KONG_BACKEND} add_pool --output json "$POOL_CALL" 2>&1)

# Check result
if echo "${POOL_RESULT}" | grep -q "\"Ok\""; then
    echo "Pool created successfully!"
    echo "${POOL_RESULT}" | jq
else
    echo "Pool creation failed:" >&2
    echo "${POOL_RESULT}"
fi

echo "VERIFICATION"
echo "Checking if SOL pool exists..."
dfx canister call --ic kong_backend pools | grep -A10 -B10 "SOL" || echo "SOL pool not found"