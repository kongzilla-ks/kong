#!/usr/bin/env bash

# Suppress DFX mainnet identity warning
export DFX_WARNING=-mainnet_plaintext_identity

# Script to add SOL token to Kong backend

NETWORK="${1:-local}"
NETWORK_FLAG=""
if [ "${NETWORK}" == "ic" ]; then
    NETWORK_FLAG="--ic"
elif [ "${NETWORK}" != "local" ]; then
    NETWORK_FLAG="--network ${NETWORK}"
fi
IDENTITY="--identity kong"
KONG_CANISTER=$(dfx canister id ${NETWORK_FLAG} kong_backend)

# Add SOL token
echo "Adding SOL token to Kong backend..."

dfx canister call ${NETWORK_FLAG} ${IDENTITY} ${KONG_CANISTER} add_token --output json "(record {
    token = \"SOL.11111111111111111111111111111111\";
    on_kong = opt true;
})" | jq

echo "SOL token added successfully!"