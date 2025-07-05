# hints in running the project locally
-- copy the example.env into an .env at the same place as the example.env, change accordingly
-- some nuance logic about kong_rpc is also explain in /kong_rpc/readme.md

# easiest way to setup kong locally:
## terminal: 1
### with two terminals/consoles its nice to see what the kong_backend is doing / if transactions are being pushed
cd kong_rpc
cargo run -r

## terminal 2:
cd scripts
sh deploy_kong.sh local
### cache the solana address into stable memory
dfx canister call kong_backend cache_solana_address --identity kong

### frontend 
dfx deploy kong_svelte


# should have icp native pools + devnet sol/local ckusdt (ksUSDT) pool
dfx canister call kong_backend pools '(null)'
dfx canister call kong_backend tokens '(null)'