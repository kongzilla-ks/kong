// Service to create actors that work with local development
import { Actor, HttpAgent } from '@dfinity/agent';
import { idlFactory as kongBackendIDL } from '../../../../../declarations/kong_backend/kong_backend.did.js';
import type { _SERVICE as KongBackendService } from '../../../../../declarations/kong_backend/kong_backend.did';
import { getKongBackendCanisterId } from '$lib/config/canisterIds';

let cachedActor: KongBackendService | null = null;
let cachedAgent: HttpAgent | null = null;

export class LocalActorService {
  static async getKongBackendActor(): Promise<KongBackendService> {
    // Return cached actor if available
    if (cachedActor && cachedAgent) {
      return cachedActor;
    }

    
    // Create the appropriate agent based on environment
    if (process.env.DFX_NETWORK === 'local') {
      // Local development - create direct agent
      cachedAgent = new HttpAgent({
        host: 'http://127.0.0.1:8000',
      });
      
      // CRITICAL: Fetch root key for local development
      try {
        await cachedAgent.fetchRootKey();
      } catch (error) {
        console.error('[LocalActorService] Failed to fetch root key:', error);
        throw error;
      }
    } else {
      // Production - use default host
      cachedAgent = new HttpAgent();
    }
    
    // Create actor with the proper canister ID
    const canisterId = getKongBackendCanisterId();
    
    console.log('[LocalActorService] Creating actor with canister ID:', canisterId);
    
    cachedActor = Actor.createActor<KongBackendService>(kongBackendIDL, {
      agent: cachedAgent,
      canisterId,
    });
    
    return cachedActor;
  }
  
  static clearCache() {
    cachedActor = null;
    cachedAgent = null;
  }
}
