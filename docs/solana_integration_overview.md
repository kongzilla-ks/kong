## Kong Swap Solana Integration Overview

To use the Kong Swap Solana integration locally, make sure you have the following installed.

1. Rust Language - https://www.rust-lang.org/tools/install
2. Internet Computer SDK - https://internetcomputer.org/docs/building-apps/getting-started/install
3. Solana CLI - https://solana.com/docs/intro/installation
4. After installation, you may need to start a new Terminal or source .bashrc to ensure the PATH environment variable
   is set properly. From the command line you should be able to run: cargo, rustc, dfx, solana, spl-token

## Internet Computer Setup

1. Create user ids - "scripts/create_identity.sh" will create the required user accounts with "kong" being the admin account
2. Start local replica - "dfx start --clean --background" will delete any existing replica and start a new local replica

## Solana CLI Setup

1. https://solana.com/docs/intro/installation look for "Solana CLI Basic".

   - Run "solana config set --url devnet" to use DEVNET
   - Run "solana-keygen new" to create a new wallet
   - Run "solana config get" to get your local configuration
   - Run "solana address" and note this as the wallet address. You can send SOL and SPL tokens to this address
   - Run "solana airdrop 5" to get 5 SOL test tokens
   - Run "solana balance" to get your SOL balance

2. Solana also has SPL tokens, which is similar to ERC-20 for Ethereum and allows for custom tokens.
   For example USDC on Solana is a SPL token. Use the "spl-token" command line to interact with SPL tokens.

   - Run "spl-token accounts" to get balances of all your SPL tokens
   - https://faucet.circle.com - visit the faucet to get USDC test tokens. Select USDC and Network: Solana Devnet. Then
     paste your Solana wallet address. (from solana address). Re-run "spl-token accounts" and see that you received 10 USDC

## Kong Backend Deployment

With the IC local replica running ("dfx start"), to deploy the Kong backend, do the following

1. Run "scripts/deploy_kong.sh local" - compiles the backend, create the test tokens (ICP, ksUSDT, ksBTC, ksETH, ksKONG),
   initialize some pools and other setups. This may take some time, but when done the kong_backend and the test token
   ledger canisters should be deployed

kong_rpc, also needs to be deployed for Solana to work. Do the following,

2. In a new Terminal, in kong_rpc directory, run "cargo run". This will output logs with the current configuration and then
   just keep sending Solana latest hash txs to kong_backend

## Test Solana transactions

In scripts/cross_chain_scripts are all the scripts to try

1. "add_sol_pool.sh local" this creates the SOL/ksUSDT liquidity pool
2. "add_usdc_pool.sh local" this creates the USDC/ksUSDT liquidity pool
3. "swap_usdt_to_sol.sh local" this swaps ksUSDT to SOL - swap an IC token to SOL token
4. "swap_sol_to_usdc.sh local" this swaps SOL to USDC - swap SOL token to SPL token

## Solana Integration Architecture Overview

The Kong Solana integration allows for fast, native swaps between Internet Computer (IC) and Solana (SOL/SPL) tokens. This was
accomplished by extending the Kong's existing IC-only architecture to support the Solana network.

The current Kong Swap operates on a single-canister architecture. All tokens are deposited into the kong_backend canister, with accounting handled in stable memory. This setup allows deposits, accounting, and settlements to all be managed by the same canister.

To integrate Solana, we extended this architecture. The kong_backend canister uses the IC management canister to create a single Solana wallet address. This is only done once when the canister is initialized and is based on the canister's principal id and a fixed derivation path. All users deposit their SOL/SPL tokens into this one address, and accounting is still tracked in stable memory. For any settlements needed on the Solana chain, the same Solana wallet is used to complete the transfer.

This approach is different from many current code examples and bridging solutions such as chain-key tokens, as we do not create a new Solana wallet address for every user.

From a user's perspective, the integration adds a single Solana wallet address to Kong. The user experience remains simple and familiar.

If you want to swap SOL for ICP, you just deposit your SOL to Kong's Solana address and call the swap() function on the kong_backend canister. You'll then receive ICP directly from the same canister.

Conversely, to swap ICP for SOL.USDC, you deposit your ICP into the kong_backend canister and call swap(). The canister will then complete the transaction by transferring the SOL.USDC from its Solana wallet to your address.

This design keeps SOL/SPL tokens on the Solana network and IC tokens on the Internet Computer blockchain, enabling seamless, native token swaps for users.

Behind the scenes, we built kong_rpc, a proxy server that reads and relays Solana transfers to the kong_backend. We developed our own proxy
because the public Solana RPC canister was not available at the time.

Here's how kong_rpc works:

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

kong_rpc is able to detect when there is a Solana deposit to our address. However, the user needs to "claim" the transaction and tell us what to do with the funds. For example, if a user wants to swap SOL for ICP, the user sends SOL to our address, but then needs to call swap() on kong_backend providing instructions like what token to receive and the wallet address to send to. Therefore, to verify that the caller of
swap() is indeed the same person that made the transfer, we require the user sign the arguments of the swap() with their private key. Then, we can verify if the signature is indeed the same public key of the sender of the tx hash. This message signing verification is required for all incoming SOL/SPL token transfers. The code for this is in,

src/kong_backend/solana/signature_verification.rs - verify_canonical_message()
We support 2 message signing standards: ed25519_dalek and off-chain message with the different being a header prefix with off-chain
messages. ed25519_dalek is more easy for javascript while off-chain messages is used by the Solana CLI.
