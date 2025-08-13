#!/usr/bin/env bash

if [ "$#" -lt 1 ]; then
    echo "Error: network (local/staging/ic) is required."
    exit 1
fi

bash create_canister_id.sh $1
NETWORK="--network $1"
KONG_CANISTER=$(dfx canister id ${NETWORK} kong_backend)
# only controller (kong) can add reward_info
IDENTITY="--identity kong"


# first swap: 1 kong
dfx canister call ${NETWORK} ${IDENTITY} ${KONG_CANISTER} add_new_reward_info --output json "(record {
    reward_volume = 1_00000000 : nat;
    reward_asset_id = null : opt nat32;
    is_active = true : bool;
    reward_rules = variant { UserSwapCount = record {
        swap_count = 1: nat32;
    }};
})" | jq


# 100 ckUSDT volume: 10 KONG
dfx canister call ${NETWORK} ${IDENTITY} ${KONG_CANISTER} add_new_reward_info --output json "(record {
    reward_volume = 10_00000000 : nat;
    reward_asset_id = null : opt nat32;
    is_active = true : bool;
    reward_rules = variant { ReferredNotionalVolume = record {
        desired_volume = 100_000000: nat;
    }};
})" | jq


# TODO: temporary, remove it
# 10 ckUSDT volume: 9 KONG
dfx canister call ${NETWORK} ${IDENTITY} ${KONG_CANISTER} add_new_reward_info --output json "(record {
    reward_volume = 9_00000000 : nat;
    reward_asset_id = null : opt nat32;
    is_active = true : bool;
    reward_rules = variant { UserNotionalVolume = record {
        desired_volume = 10_000000: nat;
    }};
})" | jq

# TODO: temporary, remove it
# 10 ckUSDT volume: 11 KONG
dfx canister call ${NETWORK} ${IDENTITY} ${KONG_CANISTER} add_new_reward_info --output json "(record {
    reward_volume = 11_00000000 : nat;
    reward_asset_id = null : opt nat32;
    is_active = true : bool;
    reward_rules = variant { ReferredNotionalVolume = record {
        desired_volume = 10_000000: nat;
    }};
})" | jq


# 100 KONG for 30 days of activity
dfx canister call ${NETWORK} ${IDENTITY} ${KONG_CANISTER} add_new_reward_info --output json "(record {
    reward_volume = 100_00000000 : nat;
    reward_asset_id = null : opt nat32;
    is_active = true : bool;
    reward_rules = variant { ConsecutiveDays = record {
        consecutive_days = 30: nat32;
    }};
})" | jq
