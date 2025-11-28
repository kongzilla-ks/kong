#!/bin/bash
set -e

# docker ps -a --filter volume=kong_dfx_state -q | xargs docker rm -f
# docker-compose down -v 
# docker-compose up --build
# ==============================================================================
# CONFIGURATION - Update this with your local principal!
# Get it by running: dfx identity get-principal --identity kong_user1
# ==============================================================================
KONG_USER1_PRINCIPAL="am3ul-zatzc-omilc-xparc-qfslm-xxm7m-irgrw-t3za7-gmluh-uqx7m-sae"
# ==============================================================================

# 1: cd into scripts directory
# 2: bash cross_chain_scripts/add_sol_pool.sh   
# 3: bash cross_chain_scripts/ping_pong_swap.sh


echo "========================================="
echo " KongSwap DFX Replica Container"
echo "========================================="

# Change to app directory
cd /app

# Remove any stale deployment signal from previous runs
rm -f /tmp/deployment-complete

# Copy canister IDs
cp canister_ids.json canister_ids.all.json 2>/dev/null || true

# Create networks.json with proper domain configuration for Docker
# This is critical for dfx 0.29+ PocketIC HTTP gateway to accept all hostnames
echo "[0/7] Configuring DFX network for Docker..."
mkdir -p /root/.config/dfx
cat > /root/.config/dfx/networks.json << 'NETJSON'
{
  "local": {
    "bind": "0.0.0.0:4943",
    "type": "ephemeral",
    "replica": {
      "subnet_type": "system"
    },
    "proxy": {
      "domain": ["localhost", "127.0.0.1", "dfx-replica", "kong-dfx", "0.0.0.0"]
    }
  }
}
NETJSON
echo "  Created /root/.config/dfx/networks.json"

# Start DFX replica in background with 0.0.0.0 binding for container access
echo "[1/7] Starting DFX replica on 0.0.0.0:4943..."
# Set DFX_NETWORK to avoid domain routing issues
export DFX_NETWORK=local
# dfx 0.29+ uses PocketIC HTTP gateway - must include 0.0.0.0 since that's the bind address
dfx start --host 0.0.0.0:4943 --domain localhost --domain 127.0.0.1 --domain 0.0.0.0 --domain dfx-replica --domain kong-dfx --background --clean 2>&1 || echo "  dfx start returned non-zero but replica may be running, checking..."

# Wait for replica to be ready
echo "[2/7] Waiting for replica to be ready..."
MAX_RETRIES=30
RETRY_COUNT=0
until curl -sf http://127.0.0.1:4943/api/v2/status > /dev/null 2>&1; do
    RETRY_COUNT=$((RETRY_COUNT + 1))
    if [ $RETRY_COUNT -ge $MAX_RETRIES ]; then
        echo "ERROR: DFX replica failed to start after ${MAX_RETRIES} attempts"
        exit 1
    fi
    echo "  Waiting for replica... (attempt $RETRY_COUNT/$MAX_RETRIES)"
    sleep 2
done
echo "  Replica is ready!"

# Create identities
echo "[3/7] Creating DFX identities..."
for identity in kong kong_token_minter kong_user1 kong_user2; do
    if ! dfx identity list 2>/dev/null | grep -q "^${identity}$"; then
        echo "  Creating identity: $identity"
        dfx identity new "$identity" --storage-mode=plaintext 2>/dev/null || true
    else
        echo "  Identity exists: $identity"
    fi
done

# Generate secrets file
echo "[4/7] Generating secrets..."
bash scripts/generate_secrets.sh 2>/dev/null || echo "  Secrets generation skipped"

# Source secrets if available
if [ -f .secrets ]; then
    set -a
    source .secrets
    set +a
fi

# Copy environment file
cp .env_local .env 2>/dev/null || true

# Deploy canisters
echo "[5/7] Deploying canisters..."

# Deploy ksUSDT ledger FIRST
echo "  Deploying ksusdt_ledger..."
bash scripts/deploy_ksusdt_ledger.sh local 2>&1 | tail -3

# Mint ksUSDT to kong_user1 for testing (principal defined at top of file)
echo "  Minting ksUSDT to kong_user1..."
# Mint 1,000,000 ksUSDT (6 decimals = 1_000_000_000_000)
dfx canister call --network local --identity kong_token_minter ksusdt_ledger icrc1_transfer "(record {
    to=record {owner=principal \"${KONG_USER1_PRINCIPAL}\"; subaccount=null};
    amount=1_000_000_000_000;
},)" 2>&1 | tail -1

# Deploy kong_backend AFTER ledger
echo "  Deploying kong_backend..."
dfx deploy kong_backend --network local --identity kong 2>&1 | tail -5

# Cache Solana address (REQUIRED before using cross-chain features)
echo "  Caching Solana address..."
CACHE_RESULT=$(dfx canister call --network local --identity kong kong_backend cache_solana_address 2>&1)
echo "  $CACHE_RESULT"

# Verify and display Solana address
SOLANA_ADDR=$(dfx canister call --network local kong_backend get_solana_address 2>&1 | tr -d '"()')
echo "  Kong Solana address: ${SOLANA_ADDR}"

# Add ksUSDT token to kong_backend
echo "  Adding ksUSDT token to kong_backend..."
KSUSDT_LEDGER=$(dfx canister id ksusdt_ledger --network local)
dfx canister call --network local --identity kong kong_backend add_token "(record {token=\"IC.${KSUSDT_LEDGER}\"})" 2>&1 | tail -1

# Export canister IDs to shared volume for kong-rpc
echo "[6/7] Exporting canister IDs to shared volume..."
mkdir -p /app/canister-ids

KONG_BACKEND_ID=$(dfx canister id kong_backend --network local 2>/dev/null || echo "")
KSUSDT_LEDGER_ID=$(dfx canister id ksusdt_ledger --network local 2>/dev/null || echo "")
ICP_LEDGER_ID=$(dfx canister id icp_ledger --network local 2>/dev/null || echo "")

if [ -n "$KONG_BACKEND_ID" ]; then
    echo "  kong_backend: $KONG_BACKEND_ID"
    # Write environment file for kong-rpc to source
    cat > /app/canister-ids/env << EOF
export KONG_BACKEND_CANISTER_ID=$KONG_BACKEND_ID
export KSUSDT_LEDGER_CANISTER_ID=$KSUSDT_LEDGER_ID
export ICP_LEDGER_CANISTER_ID=$ICP_LEDGER_ID
EOF
    # Also write JSON for other tools
    cat > /app/canister-ids/local.json << EOF
{
  "kong_backend": "$KONG_BACKEND_ID",
  "ksusdt_ledger": "$KSUSDT_LEDGER_ID",
  "icp_ledger": "$ICP_LEDGER_ID"
}
EOF
    echo "  Canister IDs exported to /app/canister-ids/"
else
    echo "  WARNING: Could not get kong_backend canister ID"
fi

# Create deployment complete signal
echo "[7/7] Signaling deployment complete..."
touch /tmp/deployment-complete

echo ""
echo "========================================="
echo " DFX Replica Ready!"
echo " Endpoint: http://0.0.0.0:4943"
echo "========================================="
echo ""

# Show deployed canisters
if [ -n "$KONG_BACKEND_ID" ]; then
    echo "  kong_backend: $KONG_BACKEND_ID"
fi
if [ -n "$KSUSDT_LEDGER_ID" ]; then
    echo "  ksusdt_ledger: $KSUSDT_LEDGER_ID"
fi

# Keep container alive
echo "Container running. Press Ctrl+C to stop."
tail -f /dev/null
