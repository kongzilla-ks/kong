# Solana Cross-Chain Integration Guide

This document provides instructions for setting up Solana cross-chain functionality with Kong's backend services.

## Prerequisites
1. Install DFX development environment
2. Set up Solana CLI with devnet configuration (`solana config set --url devnet`)

## Setup Procedure

### 1. Start DFX Environment
```sh
dfx start --clean --background
```

### 2. Deploy Kong Backend
```sh
cd scripts/ && sh deploy_kong.sh local
```

### 3. Navigate to Kong RPC Directory
```sh
cd ../kong_rpc
```

### 4. Initialize Service
```sh
cp example.env .env
cargo run -r
```

## Key Operations

### Retrieve Solana Address
```sh
dfx canister call kong_backend get_solana_address
```

### Verify Token Balances
```sh
dfx canister call kong_backend tokens '(null)'
```
*Expected: Both ICP and Solana token balances*

### Manage Liquidity Pools
```sh
# Add SOL pool
sh scripts/cross_chain_scripts/add_sol_pool.sh local

# Add USDC pool
sh scripts/cross_chain_scripts/add_usdc_pool.sh local
```

### Liquidity Operations
```sh
# Add liquidity
sh scripts/cross_chain_scripts/add_sol_lp.sh local
sh scripts/cross_chain_scripts/add_usdc_lp.sh local

# Remove liquidity
sh scripts/cross_chain_scripts/remove_sol_lp.sh local
sh scripts/cross_chain_scripts/remove_usdc_lp.sh local
```

### Token Swaps
```sh
# SOL to USDC
sh scripts/cross_chain_scripts/swap_sol_to_usdc.sh local

# USDT to SOL
sh scripts/cross_chain_scripts/swap_usdt_to_sol.sh local
```

## Notes
- Use Solana devnet (faucet available via `solana airdrop 1`)
- USDC faucet: https://faucet.circle.com
- All commands assume execution from project root directory
- Ensure `.env` is properly configured with RPC endpoints