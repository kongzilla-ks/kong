import type { GlobalPnpConfig } from '@windoge98/plug-n-play';
import { createPNP, type PNP } from "@windoge98/plug-n-play";

// Canister Imports
import { idlFactory as kongFaucetIDL, canisterId as kongFaucetCanisterId } from "../../../../declarations/kong_faucet";
import type { _SERVICE as _KONG_FAUCET_SERVICE } from '../../../../declarations/kong_faucet/kong_faucet.did.d.ts';
import { idlFactory as kongDataIDL, canisterId as kongDataCanisterId } from "../../../../declarations/kong_data";
import type { _SERVICE as _KONG_DATA_SERVICE } from '../../../../declarations/kong_data/kong_data.did.d.ts';
import { canisterId as predictionMarketsBackendCanisterId } from "../../../../declarations/prediction_markets_backend_legacy";
import { idlFactory as predictionMarketsBackendIDL } from '../../../../declarations/prediction_markets_backend_legacy';
import type { _SERVICE as _PREDICTION_MARKETS_BACKEND_SERVICE } from '../../../../declarations/prediction_markets_backend_legacy/prediction_markets_backend.did.d.ts';
import {canisterId as kongBackendCanisterId, idlFactory as kongBackendIDL } from "../../../../declarations/kong_backend";
import type { _SERVICE as _KONG_SERVICE } from '../../../../declarations/kong_backend/kong_backend.did.d.ts';
import { canisterId as trollboxCanisterId, idlFactory as trollboxIDL } from "../../../../declarations/trollbox";
import type { _SERVICE as _TROLLBOX_SERVICE } from '../../../../declarations/trollbox/trollbox.did.d.ts';
// Hardcoded SIWS provider canister ID since declarations are missing
const siwsProviderCanisterId = 'guktk-fqaaa-aaaao-a4goa-cai'; 
import { canisterId as icpCanisterId } from "../../../../declarations/icp_ledger";
import { idlFactory as icrc2IDL } from '../../../../declarations/kong_ledger/kong_ledger.did.js';
import type { _SERVICE as _ICRC2_SERVICE } from '../../../../declarations/kong_ledger/kong_ledger.did.d.ts';
import { idlFactory as icpIDL } from '../../../../declarations/icp_ledger/icp_ledger.did.js';
import type { _SERVICE as _ICP_SERVICE } from '../../../../declarations/icp_ledger/icp_ledger.did.d.ts';
import { IDL } from '@dfinity/candid';
import { signatureModalStore } from "$lib/stores/signatureModalStore";
import { getKongBackendCanisterId } from '$lib/config/canisterIds';

// Consolidated canister types
export type CanisterType = {
  KONG_BACKEND: _KONG_SERVICE;
  KONG_FAUCET: _KONG_FAUCET_SERVICE;
  KONG_DATA: _KONG_DATA_SERVICE;
  ICP_LEDGER: _ICP_SERVICE;
  ICRC2_LEDGER: _ICRC2_SERVICE;
  PREDICTION_MARKETS: _PREDICTION_MARKETS_BACKEND_SERVICE;
  TROLLBOX: _TROLLBOX_SERVICE;
}

export type CanisterConfigs = {
  [key: string]: {
    canisterId?: string;
    idl: IDL.InterfaceFactory;
    type?: any;
  };
};

export const canisters: CanisterConfigs = {
  kongBackend: {
    // Use centralized canister ID configuration
    canisterId: getKongBackendCanisterId(),
    idl: kongBackendIDL,
    type: {} as CanisterType['KONG_BACKEND'],
  },
    kongFaucet: {
    canisterId: kongFaucetCanisterId,
    idl: kongFaucetIDL,
    type: {} as CanisterType['KONG_FAUCET'],
  },
  kongData: {
    canisterId: kongDataCanisterId,
    idl: kongDataIDL,
    type: {} as CanisterType['KONG_DATA'],
  },
  predictionMarkets: {
    canisterId: predictionMarketsBackendCanisterId,
    idl: predictionMarketsBackendIDL,
    type: {} as CanisterType['PREDICTION_MARKETS'],
  },
  trollbox: {
    canisterId: trollboxCanisterId,
    idl: trollboxIDL,
    type: {} as CanisterType['TROLLBOX'],
  },
  icp: {
    canisterId: icpCanisterId,
    idl: icpIDL,
    type: {} as CanisterType['ICP_LEDGER'],
  },
  icrc1: {
    idl: icrc2IDL,
    type: {} as CanisterType['ICRC2_LEDGER'],
  },
  icrc2: {
    idl: icrc2IDL,
    type: {} as CanisterType['ICRC2_LEDGER'],
  },
}

// --- PNP Initialization ---
let globalPnp: PNP | null = null;
const isDev = process.env.DFX_NETWORK === "local";
// Use the current kong_svelte canister ID
const frontendCanisterId = "uxjo4-iiaaa-aaaam-qdxaq-cai";

const delegationTargets = [
  'u6kfa-6aaaa-aaaam-qdxba-cai', // kongBackend mainnet
  predictionMarketsBackendCanisterId,
  trollboxCanisterId,
  kongDataCanisterId
].filter(Boolean) // Remove any undefined values

// Function to show signature modal
function showSignatureModal(message: string, onSignatureComplete?: () => void) {
  signatureModalStore.show(message, onSignatureComplete);
}

// Function to hide signature modal
function hideSignatureModal() {
  signatureModalStore.hide();
}

export function initializePNP(): PNP {
  if (globalPnp) {
    return globalPnp;
  }
  try {
    // Debug environment variables
    console.log('[PNP] Environment check:');
    console.log('[PNP] DFX_NETWORK:', process.env.DFX_NETWORK);
    console.log('[PNP] CANISTER_ID_KONG_BACKEND:', process.env.CANISTER_ID_KONG_BACKEND);
    console.log('[PNP] Kong backend canister ID from declarations:', kongBackendCanisterId);
    
    // Create a stable configuration object - always use mainnet
    const config = {
      dfxNetwork: 'ic',
      // Always use mainnet host
      hostUrl: 'https://icp0.io',
      frontendCanisterId,
      timeout: BigInt(30 * 24 * 60 * 60 * 1000 * 1000 * 1000), // 30 days
      delegationTimeout: BigInt(30 * 24 * 60 * 60 * 1000 * 1000 * 1000), // 30 days
      delegationTargets,
      derivationOrigin: `https://${frontendCanisterId}.icp0.io`,
      siwsProviderCanisterId,
      fetchRootKeys: false, // Never fetch root keys on mainnet
      adapters: {
        ii: {
          enabled: true,
          localIdentityCanisterId: "rdmx6-jaaaa-aaaaa-aaadq-cai",
        },
        plug: {
          enabled: true,
        },
        nfid: {
          enabled: true,
          rpcUrl: "https://nfid.one/rpc",
        },
        oisy: {
          enabled: true,
          signerUrl: "https://oisy.com/sign",
        },
        phantomSiws: {
          enabled: true,
        },
        solflareSiws: {
          enabled: true,
        },
        backpackSiws: {
          enabled: true
        },
        walletconnectSiws: {
          enabled: true,
          projectId: "77b77ffe1132244fe4a3ce38f01885d7",
          appName: "KongSwap",
          appDescription: 'Next gen multi-chain DeFi',
          appUrl: 'https://kongswap.io',
          appIcons: ['https://kongswap.io/images/kong_logo.png'],
          onSignatureRequired: (message: string) => {
            if (typeof window !== 'undefined') {
              showSignatureModal(message);
            }
          },
          onSignatureComplete: () => {
            if (typeof window !== 'undefined') {
              hideSignatureModal();
            }
          },
        },
      },
      localStorageKey: "kongSwapPnpState",
    } as GlobalPnpConfig;

    console.log('[PNP] Final config being used:');
    console.log('[PNP] - dfxNetwork:', config.dfxNetwork);
    console.log('[PNP] - host:', config.host);
    console.log('[PNP] - hostUrl:', config.hostUrl);
    console.log('[PNP] - replicaPort:', config.replicaPort);
    console.log('[PNP] - delegationTargets:', config.delegationTargets);

    // Initialize PNP with the stable config
    globalPnp = createPNP(config);

    return globalPnp;
  } catch (error) {
    console.error("Error initializing PNP:", error);
    throw error;
  }
}

export function getPnpInstance(): PNP {
  // Ensures initialization happens if called before explicit initialization
  if (!globalPnp) {
    return initializePNP();
  }
  return globalPnp;
}

// --- Exported PNP Instance ---
export const pnp = getPnpInstance();
