To use the Kong Swap Solana integration locally, make sure you have the following installed.

1. Rust Language - https://www.rust-lang.org/tools/install
2. Internet Computer SDK - https://internetcomputer.org/docs/building-apps/getting-started/install
3. Solana CLI - https://solana.com/docs/intro/installation
4. After installation you may need to start a new Terminal or source .bashrc to ensure the PATH environment variable
   is set properly. From the command line you should be able to run: cargo, rustc, dfx, solana, spl-token

## Internet Computer Setup

1. Create user ids - "scripts/create_identity.sh" will create the required user accounts with "kong" being the admin account
2. Start local replica - "dfx start --clean --background" will delete any replica and start a new local replica

## Solana CLI Setup

1. https://solana.com/docs/intro/installation half way down look for "Solana CLI Basic".

   - Run "solana config set --url devnet" to use DEVNET
   - Run "solana-keygen new" to create a new wallet
   - Run "solana config get" to get your local configuration
   - Run "solana address" and note this as the wallet address. You can send SOL and SPL tokens to this address
   - Run "solana airdrop 5" to get 5 SOL test tokens
   - Run "solana balance" to get your SOL balance

2. Solana also has SPL tokens, which is similar to ERC-20 for Ethereum which allows anyone to create their own tokens.
   For example USDC on Solana is a SPL token. Use the "spl-token" command line to interact with SPL tokens.
   - Run "spl-token accounts" to get balances of all your SPL tokens
   - https://faucet.circle.com - visit the faucet to get USDC test tokens. Select USDC and Network: Solana Devnet. Then
     paste your Solana wallet address. (from solana address). Re-run "spl-token accounts" and see that you received 10 USDC
     test tokens

## Kong Backend Deployment

With the IC local replica running ("dfx start"), to deploy the Kong backend, do the following

1. Run "scripts/deploy_kong.sh local" - compiles the backend, create the test tokens (ICP, ksUSDT, ksBTC, ksETH, ksKONG),
   initialize some pools and other setups. This may take some time, but when done the IC setup and canisters should be deployed

kong_rpc, also needs to be deployed for Solana to work. Do the following,

1. In a new Terminal, in kong_rpc directory, run "cargo run". This will output several logs regarding the configuration and then
   eventually the it will just keep sending Solana latest hash txs

## Test Solana transactions

In scripts/cross_chain_scripts are all the scripts to try

1. "add_sol_pool.sh local" this creates the SOL/ksUSDT liquidity pool
2. "add_usdc_pool.sh local" this creates the USDC/ksUSDT liquidity pool
3. "swap_usdt_to_sol.sh local" this swaps ksUSDT to SOL - an IC token to SOL token swap
4. "swap_sol_to_usdc.sh local" this swaps SOL to USDC - a SOL token to SPL tken swap

## Solana Integration Architecture Overview

The Kong Solana integration allows for fast native swaps between IC and SOL/SPL tokens. In many ways, the architecture is just an extension of the IC-only framework that KongSwap was built upon.

In the original KongSwap, all tokens were deposited to the kong_backend canister and then the accounting was kept in stable memory. This allowed for the single canister architecture where all deposits went to kong_backend, accounting was then in stable memory and settlement would also be done from kong_backend. We extend this for Solana, where kong_backend creates 1 Solana wallet address through the IC management canister and all users deposit SOL/SPL tokens into this address, accounting is done again in stable memory and if settlement is needed on the Solana chain, kong_backend's Solana wallet is used for the transfer. Unlike many examples and chain-key minting bridging, we do not generate a new address for every user.

From a user's point of view, the 1 Solana wallet address is really the "only" new enhancement for Kong. If a user wants to swap SOL for ICP, the user just needs to deposit SOL to Kong's Solana address, calls swap() on the kong_backend and then will receive ICP from kong_backend. For swapping ICP to SOL.USDC, the user deposits ICP to kong_backend, calls swap() on the kong_backend and then kong_backend will transfer from its Solana address the SOL.USDC to complete the swap. Therfore, SOL/SPL tokens stay on the Solana blockchain and IC tokens stay on the IC blockchain and allow users to swap tokens natively.

Behind the scenes, we had to create kong_rpc which acts as a proxy server to read and relay Solana transactions to the kong_backend. It's functions much like the Solana RPC canister, but we had to write our own initially until this was released. Here's some of the functionality,

1. Proxy between RPC node provider (Validation Cloud, QuickNode, ...)
2. Sends periodically the latest tx hash from the Solana chain to kong_backend as any transactions requires a recent tx hash
3. Monitors via websocket notifications any deposits kong_backend's wallet and notifies kong_backend on receipt
4. Monitors via websocket notifications any new SPL tokens deposited and creates new tokens on kong_backend
5. Relays any outgoing transactions from kong_backend to Solana blockchains. The message create and signing in done on kong_backend via IC
   management canister Schnorr signing but then encrypted message is sent to kong_rpc and then passed along to the Solana RPC node provider

## Code to Audit

1. kong_rpc directory. This is all new code. This is a console program that interacts with a Solana RPC node provider and calls the kong_backend api to notify or update states in kong_backend

2. src/kong_backend. The following functionality had to be upgraded to support Solana transactions
   - stable_token - added SOL token type
   - tokens
   - pools
   - add_token
   - add_pool
   - add_liquidity
   - remove_liquidity
   - swap

In stable_memory.rs there are several new Solana specific stable memory structures:

CACHED_SOLANA_ADDRESS - String to store the canister's Solana address to retrieve quickly. There is a canister api, cache_solana_address() which sets this, and get_solana_address() to retreive it
SOLANA_LATEST_BLOCKHASH - stores the latest Solana blockhash which is required for all Solana transactions
NEXT_SOLANA_SWAP_JOB_ID - counter for kong_rpc job id
SOLANA_SWAP_JOB_QUEUE - BTreeMap of pending outgoing Solana transaactions
SOLANA_TX_NOTIFICATIONS - BTreeMap of incoming Solana transactions

In src/kong_backend/solana directory is most of the Solana specific code

network.rs -
get_public_key() generation of Schnorr public key for Solana address

## Solana message signing

With kong_rpc we are able to detect when there is a Solana deposit to our address. However, the user needs to "claim" the transaction and tell us what to do with the funds. For example, if a user wants to swap SOL for ICP, the user sends SOL to our address, but then needs to call swap() on kong_backend providing instructions like what token to receive and the wallet address to send to. Therefore, to verify that the caller of swap() is indeed the same person that made the transfer, we require the user sign the arguments of the swap() with the private key. Then, we can verify if the signature is indeed the same as the public key of the sender on the tx hash. This message signing verification is required for all incoming SOL/SPL token transfers. The codefor this is in,

src/kong_backend/solana/signature_verification.rs - verify_canonical_message(). We support 2 message signing standards: ed25519_dalek and off-chain message with the different being a header prefix with off-chain messages. ed25519_dalek is more easy for javascript while off-chain messages is used by the Solana CLI.
