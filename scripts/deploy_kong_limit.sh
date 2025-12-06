#!/usr/bin/env bash

NETWORK="--network $1"
IDENTITY="--identity kong"
KONG_LIMIT=kong_limit_order

if [ "$1" == "ic" ]; then
    bash create_canister_id.sh ic
    KONG_BUILDENV="ic" dfx build ${KONG_LIMIT} ${NETWORK}
    bash gzip_kong_limit.sh ic
elif [ "$1" == "staging" ]; then
    bash create_canister_id.sh staging
    KONG_BUILDENV="staging" dfx deploy ${KONG_LIMIT} ${NETWORK} ${IDENTITY}
elif [ "$1" == "local" ]; then
    original_dir=$(pwd)
    root_dir="${original_dir}"/..
    if CANISTER_ID=$(jq -r ".[\"kong_limit_order\"][\"local\"]" "${root_dir}"/canister_ids.all.json); then
        [ "${CANISTER_ID}" != "null" ] && {
            SPECIFIED_ID="--specified-id ${CANISTER_ID}"
            KONG_BUILDENV="local" dfx deploy ${KONG_LIMIT} ${NETWORK} ${IDENTITY} ${SPECIFIED_ID} || true
        }
    fi
fi
