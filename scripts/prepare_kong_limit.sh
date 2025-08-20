#!/usr/bin/env bash

bash create_canister_id.sh $1
NETWORK="--network $1"
KONG_LIMIT_CANISTER=$(dfx canister id ${NETWORK} kong_limit_order)

# 1. Add ksUSDT token
# only controller (kong) can add token
IDENTITY="--identity kong"


dfx canister call ${NETWORK} ${IDENTITY} ${KONG_LIMIT_CANISTER} add_available_orderbook --output json "(record {token_0 = \"ksKONG\"; token_1 = \"ksUSDT\"})"

