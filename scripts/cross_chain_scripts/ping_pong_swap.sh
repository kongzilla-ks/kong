#!/usr/bin/env bash
set -euo pipefail

# Ping-pong swaps between ksUSDT and SOL every 30 seconds (RPC stress test)
# usage: bash ping_pong_swap.sh (actually use bash if you want the stats after closing it - Ctrl+C to stop)
# Logs to: logs/ping_pong_YYYYMMDD_HHMMSS.log

NETWORK="${1:-local}"
IDENTITY_FLAG="--identity kong_user1"
SWAP_INTERVAL=30  # 30 second interval for testing

# Setup logging
LOG_DIR="logs"
mkdir -p "$LOG_DIR"
LOG_FILE="${LOG_DIR}/ping_pong_$(date '+%Y%m%d_%H%M%S').log"

# Log helper - write to both stdout and log file
log() {
    echo "$@"
    echo "$@" >> "$LOG_FILE"
}

# CANISTER IDS
MAINNET_KONG_BACKEND="u6kfa-6aaaa-aaaam-qdxba-cai"
LOCAL_KONG_BACKEND="kong_backend"
MAINNET_USDT_LEDGER="cngnf-vqaaa-aaaar-qag4q-cai"
LOCAL_USDT_LEDGER="ksusdt_ledger"

NETWORK_FLAG=$([ "${NETWORK}" == "local" ] && echo "" || echo "--network ${NETWORK}")

if [ "${NETWORK}" == "ic" ]; then
    KONG_BACKEND="${MAINNET_KONG_BACKEND}"
    USDT_LEDGER="${MAINNET_USDT_LEDGER}"
    USDT_SYMBOL="ckUSDT"
else
    KONG_BACKEND=$(dfx canister id ${LOCAL_KONG_BACKEND})
    USDT_LEDGER=$(dfx canister id ${LOCAL_USDT_LEDGER})
    USDT_SYMBOL="ksUSDT"
    solana config set --url devnet >/dev/null 2>&1
fi

SOLANA_ADDRESS=$(solana address)
KONG_SOLANA_ADDRESS=$(dfx canister call ${NETWORK_FLAG} ${KONG_BACKEND} get_solana_address --output json | jq -r '.')

# Stats tracking
USDT_TO_SOL_SUCCESS=0
USDT_TO_SOL_FAIL=0
SOL_TO_USDT_SUCCESS=0
SOL_TO_USDT_FAIL=0
SOL_TO_USDT_RETRIES=0

LOG_FILE="${LOG_DIR}/ping_pong_$(date '+%Y%m%d_%H%M%S').log"

echo "========= PING-PONG SWAP (FAST MODE) ========="
echo "Network: ${NETWORK}"
echo "User Solana: ${SOLANA_ADDRESS}"
echo "Kong Solana: ${KONG_SOLANA_ADDRESS}"
echo "Interval: ${SWAP_INTERVAL}s | Retries: 10 | Initial wait: 3s"
echo "Logging to: ${LOG_FILE}"
echo "Press Ctrl+C to stop."
echo "==============================================="

swap_usdt_to_sol() {
    local pay_amount=1000000  # 1 USDT
    echo ""
    echo "$(date '+%H:%M:%S') >>> ${USDT_SYMBOL} → SOL (pay: 1 ${USDT_SYMBOL})"

    # Approve
    local fee=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${USDT_LEDGER} icrc1_fee "()" 2>/dev/null | awk -F'[:]+' '{print $1}' | awk '{gsub(/\(/, ""); print}' | tr -d '_')
    local approve_amt=$((pay_amount + fee))
    dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${USDT_LEDGER} icrc2_approve "(record {
        amount = ${approve_amt};
        spender = record { owner = principal \"${KONG_BACKEND}\" };
    })" >/dev/null 2>&1

    # Swap
    local result=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} swap "(record {
        pay_token = \"${USDT_SYMBOL}\";
        pay_amount = ${pay_amount};
        receive_token = \"SOL\";
        receive_amount = opt 0;
        max_slippage = opt 95.0;
        receive_address = opt \"${SOLANA_ADDRESS}\";
    })" 2>&1 || echo "Err")

    if echo "$result" | grep -q "Ok"; then
        local request_id=$(echo "$result" | grep -o 'request_id = [0-9]*' | head -1 | sed 's/request_id = //')
        local tx_id=$(echo "$result" | grep -o 'tx_id = [0-9]*' | head -1 | sed 's/tx_id = //')
        local signature=$(echo "$result" | grep -o 'signature = "[^"]*"' | head -1 | sed 's/signature = //' | tr -d '"')
        local recv_amt=$(echo "$result" | grep -o 'receive_amount = [0-9_]*' | tail -1 | sed 's/receive_amount = //' | tr -d '_')
        local sol_amt=$(bc <<< "scale=9; ${recv_amt} / 1000000000" 2>/dev/null || echo "?")
        echo "$(date '+%H:%M:%S') ✓ Swap succeeded | req_id: ${request_id:-?} | tx_id: ${tx_id:-?} | sig: ${signature:-?} | receive: ${sol_amt} SOL"
        USDT_TO_SOL_SUCCESS=$((USDT_TO_SOL_SUCCESS + 1))
    else
        local err=$(echo "$result" | grep -o 'Err = "[^"]*"' | head -1 || echo 'unknown')
        echo "$(date '+%H:%M:%S') ✗ Swap failed | error: ${err}"
        USDT_TO_SOL_FAIL=$((USDT_TO_SOL_FAIL + 1))
    fi
}

swap_sol_to_usdt() {
    local pay_amount=200000  # 0.0002 SOL (2x stress test)
    local sol_dec=$(bc <<< "scale=9; ${pay_amount} / 1000000000")
    echo ""
    echo "$(date '+%H:%M:%S') <<< SOL → ${USDT_SYMBOL} (pay: ${sol_dec} SOL)"

    # Get quote
    local quote=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} swap_amounts "(\"SOL\", ${pay_amount}, \"${USDT_SYMBOL}\")" 2>&1)
    if ! echo "$quote" | grep -q "Ok"; then
        echo "$(date '+%H:%M:%S') ✗ Quote failed"
        return
    fi

    local receive_amt=$(echo "$quote" | grep -o 'receive_amount = [0-9_]*' | tail -1 | sed 's/receive_amount = //' | tr -d '_')
    local usdt_amt=$(bc <<< "scale=6; ${receive_amt} / 1000000" 2>/dev/null || echo "?")

    # Transfer SOL
    local tx_output=$(solana transfer --allow-unfunded-recipient "${KONG_SOLANA_ADDRESS}" "${sol_dec}" 2>&1)
    local tx_sig=$(echo "$tx_output" | grep -o 'Signature: .*' | awk '{print $2}')

    if [ -z "$tx_sig" ]; then
        echo "$(date '+%H:%M:%S') ✗ SOL transfer failed"
        return
    fi

    echo "$(date '+%H:%M:%S')   → SOL tx: ${tx_sig:0:8}... | quote: ${usdt_amt} ${USDT_SYMBOL}"

    # Get IC principal for receiving IC tokens
    local ic_principal=$(dfx identity ${IDENTITY_FLAG} get-principal)

    # Sign message (receive_address should be IC principal for IC tokens)
    local msg=$(printf '{"pay_token":"SOL","pay_amount":"%s","pay_address":"%s","receive_token":"%s","receive_amount":"%s","receive_address":"%s","max_slippage":95.0,"referred_by":null}' \
        "SOL" "${pay_amount}" "${SOLANA_ADDRESS}" "${USDT_SYMBOL}" "${receive_amt}" "${ic_principal}")
    local sig=$(solana sign-offchain-message "${msg}" 2>/dev/null)

    # Swap with retry
    sleep 3

    for retry in {1..30}; do
        local result=$(dfx canister call ${NETWORK_FLAG} ${IDENTITY_FLAG} ${KONG_BACKEND} swap "(record {
            pay_token = \"SOL\";
            pay_amount = ${pay_amount};
            pay_tx_id = opt variant { TransactionId = \"${tx_sig}\" };
            receive_token = \"${USDT_SYMBOL}\";
            receive_amount = opt ${receive_amt};
            max_slippage = opt 95.0;
            receive_address = opt \"${ic_principal}\";
            pay_signature = opt \"${sig}\";
        })" 2>&1 || echo "Err")

        if echo "$result" | grep -q "Ok"; then
            local request_id=$(echo "$result" | grep -o 'request_id = [0-9]*' | head -1 | sed 's/request_id = //')
            local tx_id=$(echo "$result" | grep -o 'tx_id = [0-9]*' | head -1 | sed 's/tx_id = //')
            echo "$(date '+%H:%M:%S') ✓ Swap succeeded | req_id: ${request_id:-?} | tx_id: ${tx_id:-?} | receive: ${usdt_amt} ${USDT_SYMBOL}"
            if [ $retry -gt 1 ]; then
                SOL_TO_USDT_RETRIES=$((SOL_TO_USDT_RETRIES + retry - 1))
            fi
            SOL_TO_USDT_SUCCESS=$((SOL_TO_USDT_SUCCESS + 1))
            return
        fi

        if echo "$result" | grep -q "TRANSACTION_NOT_READY"; then
            echo "$(date '+%H:%M:%S')   ⏳ TX not ready, retry $retry/10..."
            sleep 2
        else
            local err=$(echo "$result" | grep -o 'Err = "[^"]*"' | head -1 || echo 'unknown')
            echo "$(date '+%H:%M:%S') ✗ Swap failed | error: ${err}"
            SOL_TO_USDT_FAIL=$((SOL_TO_USDT_FAIL + 1))
            return
        fi
    done

    echo "$(date '+%H:%M:%S') ✗ Swap failed after 10 retries"
    SOL_TO_USDT_FAIL=$((SOL_TO_USDT_FAIL + 1))
}

print_summary() {
    echo ""
    echo "================= SUMMARY ================="
    echo "Total Rounds: $counter"
    echo ""
    echo "ksUSDT → SOL:"
    echo "  Success: $USDT_TO_SOL_SUCCESS"
    echo "  Failed:  $USDT_TO_SOL_FAIL"
    local usdt_total=$((USDT_TO_SOL_SUCCESS + USDT_TO_SOL_FAIL))
    if [ $usdt_total -gt 0 ]; then
        local usdt_rate=$(bc <<< "scale=1; $USDT_TO_SOL_SUCCESS * 100 / $usdt_total")
        echo "  Rate:    ${usdt_rate}%"
    fi
    echo ""
    echo "SOL → ksUSDT:"
    echo "  Success: $SOL_TO_USDT_SUCCESS"
    echo "  Failed:  $SOL_TO_USDT_FAIL"
    echo "  Retries: $SOL_TO_USDT_RETRIES"
    local sol_total=$((SOL_TO_USDT_SUCCESS + SOL_TO_USDT_FAIL))
    if [ $sol_total -gt 0 ]; then
        local sol_rate=$(bc <<< "scale=1; $SOL_TO_USDT_SUCCESS * 100 / $sol_total")
        local avg_retries=$(bc <<< "scale=2; $SOL_TO_USDT_RETRIES / $sol_total")
        echo "  Rate:    ${sol_rate}%"
        echo "  Avg Retries: ${avg_retries}"
    fi
    echo ""
    echo "Log saved: ${LOG_FILE}"
    echo "==========================================="
}

trap 'print_summary; exit 0' INT TERM

counter=0
while true; do
    counter=$((counter + 1))
    echo ""
    echo "========== Round $counter =========="

    swap_usdt_to_sol
    
    swap_sol_to_usdt
    sleep $SWAP_INTERVAL
done
