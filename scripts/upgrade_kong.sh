#!/usr/bin/env bash

# Usage: ./upgrade_kong.sh [NETWORK] [PACKAGE]
#
# NETWORK: local, staging, ic
# PACKAGE: all, kong_backend, kong_data, kong_svelte
#

NETWORK=${1:-local}
PACKAGE=${2:-all}
IDENTITY="--identity kong"

echo "Upgrading KONG canisters to ${NETWORK}"

if [ "${PACKAGE}" == "all" ] || [ "${PACKAGE}" == "kong_backend" ]; then
    bash create_canister_id.sh ${NETWORK}
    KONG_BUILDENV=$1 dfx build kong_backend
    dfx canister install --network ${NETWORK} ${IDENTITY} kong_backend --mode upgrade
fi

if [ "${PACKAGE}" == "all" ] || [ "${PACKAGE}" == "kong_data" ]; then
    bash create_canister_id.sh ${NETWORK}
    KONG_BUILDENV=$1 dfx build kong_data
    dfx canister install --network ${NETWORK} ${IDENTITY} kong_data --mode upgrade
fi

if [ "${PACKAGE}" == "all" ] || [ "${PACKAGE}" == "kong_svelte" ]; then
    dfx build kong_svelte
    dfx canister install --network ${NETWORK} ${IDENTITY} kong_svelte --mode upgrade
fi
