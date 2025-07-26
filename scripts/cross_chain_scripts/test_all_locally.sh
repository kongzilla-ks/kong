#!/usr/bin/env bash
set -euo pipefail

echo "Starting cross-chain testing locally..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

NETWORK="local"

# Helper function to run a script with basic error handling
run_script() {
    local script_name="$1"
    local description="$2"
    
    echo "Running $description..."
    
    if sh "$script_name" "$NETWORK"; then
        echo "$description completed"
    else
        echo "$description failed"
        exit 1
    fi
}

# Add Pools
run_script "add_sol_pool.sh" "SOL pool creation"
run_script "add_usdc_pool.sh" "USDC pool creation"

# Add Liquidity
run_script "add_sol_lp.sh" "SOL liquidity provision"
run_script "add_usdc_lp.sh" "USDC liquidity provision"

# Test Swaps
run_script "swap_sol_to_usdc.sh" "SOL to USDC swap"
run_script "swap_usdt_to_sol.sh" "USDT to SOL swap"

# Remove Liquidity
run_script "remove_sol_lp.sh" "SOL liquidity removal"
run_script "remove_usdc_lp.sh" "USDC liquidity removal"

echo "All cross-chain tests completed successfully"
