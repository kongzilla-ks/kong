// Test direct connection to local canister bypassing PNP
import { Actor, HttpAgent } from '@dfinity/agent';
import { idlFactory } from '../../../../declarations/kong_backend/kong_backend.did.js';
import { getKongBackendCanisterId } from '$lib/config/canisterIds';

export async function testLocalCanisterConnection() {
  console.log('[TestLocal] Starting direct canister connection test...');
  
  try {
    // Create agent directly with local host
    const agent = new HttpAgent({
      host: 'http://127.0.0.1:4943',
    });
    
    // Fetch root key for local development
    if (process.env.DFX_NETWORK === 'local') {
      await agent.fetchRootKey();
      console.log('[TestLocal] Root key fetched successfully');
    }
    
    // Create actor directly
    const canisterId = getKongBackendCanisterId();
    const actor = Actor.createActor(idlFactory, {
      agent,
      canisterId,
    });
    
    console.log('[TestLocal] Actor created successfully');
    console.log('[TestLocal] Agent host:', (agent as any)._host);
    console.log('[TestLocal] Canister ID:', canisterId);
    
    // Call tokens method
    const result = await actor.tokens([]);
    console.log('[TestLocal] Raw result from direct call:', result);
    
    if ('Ok' in result) {
      console.log('[TestLocal] ✅ SUCCESS: Got', result.Ok.length, 'tokens from LOCAL canister');
      console.log('[TestLocal] First few tokens:', result.Ok.slice(0, 3).map((t: any) => {
        if ('IC' in t) return `IC: ${t.IC.symbol}`;
        if ('Solana' in t) return `Solana: ${t.Solana.symbol}`;
        if ('LP' in t) return `LP: ${t.LP.symbol}`;
        return 'Unknown type';
      }));
      return result.Ok.length;
    } else {
      console.error('[TestLocal] ❌ ERROR:', result);
      return 0;
    }
  } catch (error) {
    console.error('[TestLocal] ❌ Connection error:', error);
    throw error;
  }
}

// Export for use in browser console
if (typeof window !== 'undefined') {
  (window as any).testLocalCanister = testLocalCanisterConnection;
}
