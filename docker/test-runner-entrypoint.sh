#!/bin/bash
set -e

echo "========================================="
echo " KongSwap Test Runner"
echo "========================================="

cd /app

# Wait for dfx-replica to be accessible
echo "[1/5] Waiting for DFX replica..."
MAX_RETRIES=60
RETRY_COUNT=0
until curl -sf http://dfx-replica:4943/api/v2/status > /dev/null 2>&1; do
    RETRY_COUNT=$((RETRY_COUNT + 1))
    if [ $RETRY_COUNT -ge $MAX_RETRIES ]; then
        echo "ERROR: DFX replica not accessible after ${MAX_RETRIES} attempts"
        exit 1
    fi
    echo "  Waiting for dfx-replica:4943... (attempt $RETRY_COUNT/$MAX_RETRIES)"
    sleep 5
done
echo "  DFX replica is accessible!"

# Create a dfx.json that points to dfx-replica container
echo "[2/5] Configuring DFX network..."
cat > /app/dfx.json << 'EOF'
{
  "canisters": {
    "kong_backend": {
      "type": "custom",
      "candid": "kong_backend.did",
      "wasm": "kong_backend.wasm"
    },
    "ksusdt_ledger": {
      "type": "custom",
      "candid": "ledger.did",
      "wasm": "ledger.wasm"
    }
  },
  "networks": {
    "local": {
      "providers": ["http://dfx-replica:4943"],
      "type": "persistent"
    }
  },
  "version": 1
}
EOF

# Link or copy canister IDs from shared volume
echo "[3/5] Setting up canister IDs..."
if [ -d "/root/.local/share/dfx" ]; then
    # Create .dfx directory structure
    mkdir -p /app/.dfx/local

    # Copy canister IDs if they exist
    if [ -f "/root/.local/share/dfx/local/canister_ids.json" ]; then
        cp /root/.local/share/dfx/local/canister_ids.json /app/.dfx/local/
        echo "  Copied canister_ids.json from shared volume"
    fi

    # Also create a project-level canister_ids.json with static IDs
    cat > /app/canister_ids.json << 'EOF'
{
  "kong_backend": {
    "local": "2ipq2-uqaaa-aaaar-qailq-cai"
  },
  "ksusdt_ledger": {
    "local": "zdzgz-siaaa-aaaar-qaiba-cai"
  }
}
EOF
    echo "  Created static canister_ids.json"
fi

# Verify canister IDs are accessible
echo "  Testing canister ID resolution..."
KONG_BACKEND_ID=$(dfx canister id kong_backend --network local 2>/dev/null || echo "")
if [ -n "$KONG_BACKEND_ID" ]; then
    echo "  kong_backend: $KONG_BACKEND_ID"
else
    echo "  WARNING: Could not resolve kong_backend canister ID"
fi

# Configure Solana
echo "[4/5] Configuring Solana CLI..."
solana config set --url "${SOLANA_URL:-https://api.devnet.solana.com}" > /dev/null 2>&1

# Check Solana balance
SOLANA_BALANCE=$(solana balance 2>/dev/null | awk '{print $1}' || echo "0")
SOLANA_ADDR=$(solana address 2>/dev/null || echo "unknown")
echo "  Solana address: ${SOLANA_ADDR}"
echo "  Solana balance: ${SOLANA_BALANCE} SOL"

# Try to airdrop if balance is low
if [ "$(echo "$SOLANA_BALANCE < 0.5" | bc -l 2>/dev/null || echo "1")" = "1" ]; then
    echo "  Attempting Solana airdrop (2 SOL)..."
    solana airdrop 2 2>/dev/null || echo "  Airdrop failed (rate limited or unavailable)"
fi

# Ensure kong-rpc has fully initialized by checking if it can respond
echo "[5/6] Waiting for kong-rpc to fully initialize..."
# Give kong-rpc time to connect to database and add SOL token
sleep 15
echo "  kong-rpc should be ready now"

# Verify SOL token was added by kong-rpc
echo "  Checking if SOL token exists..."
SOL_EXISTS=$(dfx canister call --network local ${KONG_BACKEND_ID:-kong_backend} tokens --output json 2>/dev/null | grep -c '"SOL"' | head -1 | tr -d '[:space:]' || echo "0")
SOL_EXISTS=${SOL_EXISTS:-0}
if [ "$SOL_EXISTS" -gt 0 ] 2>/dev/null; then
    echo "  ✓ SOL token found in kong_backend"
else
    echo "  ⚠ Warning: SOL token not yet in kong_backend (kong-rpc may still be initializing)"
fi

# Create SOL/ksUSDT pool if it doesn't exist
echo "[6/6] Creating SOL/ksUSDT liquidity pool..."
if [ -f /app/scripts/cross_chain_scripts/add_sol_pool.sh ]; then
    echo "  Running add_sol_pool.sh..."
    cd /app
    bash /app/scripts/cross_chain_scripts/add_sol_pool.sh local 2>&1 | tail -10 || echo "  Pool creation failed or pool already exists"
else
    echo "  ⚠ Warning: add_sol_pool.sh not found, skipping pool creation"
fi

# Run the test command
echo ""
echo "========================================="
echo " Starting Ping-Pong Tests"
echo "========================================="
echo ""

exec "$@"
