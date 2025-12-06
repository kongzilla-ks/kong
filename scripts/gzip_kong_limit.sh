#!/usr/bin/env bash

original_dir=$(pwd)
root_dir="${original_dir}"/..

if [ "$1" == "ic" ]; then
    [ -f "${root_dir}"/.dfx/ic/canisters/kong_limit_order/kong_limit_order.wasm ] && {
        ic-wasm "${root_dir}"/.dfx/ic/canisters/kong_limit_order/kong_limit_order.wasm -o "${root_dir}"/.dfx/ic/canisters/kong_limit_order/kong_limit_order_opt.wasm shrink --optimize O3
        gzip -c "${root_dir}"/.dfx/ic/canisters/kong_limit_order/kong_limit_order_opt.wasm > "${root_dir}"/.dfx/ic/canisters/kong_limit_order/kong_limit_order.wasm.gz
        rm "${root_dir}"/.dfx/ic/canisters/kong_limit_order/kong_limit_order_opt.wasm
    }
fi
