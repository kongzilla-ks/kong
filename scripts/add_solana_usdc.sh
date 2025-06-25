#!/usr/bin/env bash

# Suppress DFX mainnet identity warning
export DFX_WARNING=-mainnet_plaintext_identity

# Script to add USDC token to Kong backend
# For production: Uses mainnet USDC (EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)
# For local/staging: Uses devnet USDC (4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU)
# $: solana address
# SPL devnet usdc faucet: https://faucet.circle.com/

NETWORK="${1:-local}"
NETWORK_FLAG=""
if [ "${NETWORK}" != "local" ]; then
    NETWORK_FLAG="--network ${NETWORK}"
fi
IDENTITY="--identity kong"
KONG_CANISTER=$(dfx canister id ${NETWORK_FLAG} kong_backend)

# Add USDC token - will use appropriate address based on build features
echo "Adding USDC token to Kong backend..."

# Use mainnet USDC for ic network, devnet for others
if [ "${NETWORK}" == "ic" ]; then
    USDC_ADDRESS="EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    echo "Using mainnet USDC address: ${USDC_ADDRESS}"
else
    USDC_ADDRESS="4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
    echo "Using devnet USDC address: ${USDC_ADDRESS}"
fi

dfx canister call ${NETWORK_FLAG} ${IDENTITY} ${KONG_CANISTER} add_token --output json "(record {
    token = \"SOL.${USDC_ADDRESS}\";
    on_kong = opt true;
})" | jq

echo "USDC token added successfully!"
