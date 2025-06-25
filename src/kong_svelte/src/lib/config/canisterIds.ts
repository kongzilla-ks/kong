// Centralized canister ID configuration
// This file determines which canister IDs to use based on the environment

// Get canister IDs from environment or use defaults
export const CANISTER_IDS = {
  // Always use the canister ID from the environment if available
  KONG_BACKEND: process.env.CANISTER_ID_KONG_BACKEND || 
    // For local development, use your local canister ID
    (process.env.DFX_NETWORK === 'local' ? 'be2us-64aaa-aaaaa-qaabq-cai' : 'u6kfa-6aaaa-aaaam-qdxba-cai'),
  
  KONG_SVELTE: process.env.CANISTER_ID_KONG_SVELTE || '3ldz4-aiaaa-aaaar-qaina-cai',
  KONG_FAUCET: process.env.CANISTER_ID_KONG_FAUCET || 'ohr23-xqaaa-aaaar-qahqq-cai',
  PREDICTION_MARKETS: process.env.CANISTER_ID_PREDICTION_MARKETS_BACKEND || 'xidgj-jyaaa-aaaad-qghpq-cai',
  TROLLBOX: process.env.CANISTER_ID_TROLLBOX || 'rchbn-fqaaa-aaaao-a355a-cai',
  SIWS_PROVIDER: process.env.CANISTER_ID_SIWS_PROVIDER || 'guktk-fqaaa-aaaao-a4goa-cai',
  
  // Ledger canister IDs
  ICP_LEDGER: 'ryjl3-tyaaa-aaaaa-aaaba-cai',
  CKUSDT_LEDGER: process.env.CANISTER_ID_KSUSDT_LEDGER || 'cngnf-vqaaa-aaaar-qag4q-cai',
  CKBTC_LEDGER: process.env.CANISTER_ID_KSBTC_LEDGER || 'zeyan-7qaaa-aaaar-qaibq-cai',
  CKETH_LEDGER: process.env.CANISTER_ID_KSETH_LEDGER || 'zr7ra-6yaaa-aaaar-qaica-cai',
  KSICP_LEDGER: process.env.CANISTER_ID_KSICP_LEDGER || 'nppha-riaaa-aaaal-ajf2q-cai',
  KSKONG_LEDGER: process.env.CANISTER_ID_KSKONG_LEDGER || 'uxrrr-q7777-77774-qaaaq-cai',
};

// Helper to get the correct backend canister ID
export function getKongBackendCanisterId(): string {
  console.log('[CanisterIDs] Environment check:');
  console.log('[CanisterIDs] DFX_NETWORK:', process.env.DFX_NETWORK);
  console.log('[CanisterIDs] CANISTER_ID_KONG_BACKEND from env:', process.env.CANISTER_ID_KONG_BACKEND);
  console.log('[CanisterIDs] Using canister ID:', CANISTER_IDS.KONG_BACKEND);
  
  return CANISTER_IDS.KONG_BACKEND;
}

// Helper to check if we're in local development
export function isLocalDevelopment(): boolean {
  return process.env.DFX_NETWORK === 'local';
}

// Export individual canister IDs for backward compatibility
export const KONG_BACKEND_CANISTER_ID = CANISTER_IDS.KONG_BACKEND;
export const KONG_SVELTE_CANISTER_ID = CANISTER_IDS.KONG_SVELTE;
export const KONG_FAUCET_CANISTER_ID = CANISTER_IDS.KONG_FAUCET;
