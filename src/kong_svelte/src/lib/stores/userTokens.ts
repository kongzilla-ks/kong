import { browser } from '$app/environment';
import { fetchTokensByCanisterId } from '$lib/api/tokens';
import { writable, get, derived } from 'svelte/store';
import { syncTokens as analyzeTokens, applyTokenChanges } from '$lib/utils/tokenSyncUtils';
import { debounce } from '$lib/utils/debounce';
import { BackendTokenService } from '$lib/services/tokens/BackendTokenService';
import { testLocalCanisterConnection } from '$lib/utils/testLocalCanister';
import { solanaPollingService } from '$lib/services/solana/SolanaPollingService';

// Define types for sync operations
interface SyncTokensResult {
  tokensToAdd: Kong.Token[];
  tokensToRemove: Kong.Token[];
  syncStatus: { added: number; removed: number } | null;
}

// Define updated state interface using Set for enabled tokens
interface UserTokensState {
  enabledTokens: Set<string>;
  tokens: Kong.Token[];
  tokenData: Map<string, Kong.Token>;
  isAuthenticated: boolean;
  lastUpdated: number;
}

// Keys for storage
const STORAGE_KEY_PREFIX = 'userTokens';
const CURRENT_PRINCIPAL_KEY = 'principals:current_principal';

// Cache for token details to avoid redundant fetches
const tokenDetailsCache = new Map<string, Kong.Token>();

// Flag to use backend tokens instead of API - ALWAYS use backend for local development
const USE_BACKEND_TOKENS = true;

// Memoization utility
function memoize<T>(fn: (...args: any[]) => T): (...args: any[]) => T {
  const cache = new Map<string, T>();
  return (...args: any[]): T => {
    const key = JSON.stringify(args);
    if (cache.has(key)) return cache.get(key)!;
    const result = fn(...args);
    cache.set(key, result);
    return result;
  };
}

// Helper function to safely convert BigInt values to strings for JSON serialization
function safeStringify(value: any): any {
  if (typeof value === 'bigint') {
    return value.toString();
  } else if (Array.isArray(value)) {
    return value.map(safeStringify);
  } else if (value !== null && typeof value === 'object') {
    const result: Record<string, any> = {};
    for (const key in value) {
      result[key] = safeStringify(value[key]);
    }
    return result;
  }
  return value;
}

// Serialize state for storage
function serializeState(state: UserTokensState): any {
  return {
    enabledTokens: Array.from(state.enabledTokens),
    tokenData: Array.from(state.tokenData.values()),
    isAuthenticated: state.isAuthenticated,
    lastUpdated: state.lastUpdated,
  };
}

// Deserialize state from storage
function deserializeState(serialized: any): UserTokensState {
  if (!serialized) {
    return {
      enabledTokens: new Set<string>(),
      tokenData: new Map<string, Kong.Token>(),
      tokens: [],
      isAuthenticated: false,
      lastUpdated: Date.now(),
    };
  }

  const enabledTokens = new Set<string>(serialized.enabledTokens || []);
  const tokenData = new Map<string, Kong.Token>();

  // Add tokens to tokenData Map
  if (Array.isArray(serialized.tokenData)) {
    serialized.tokenData.forEach((token: Kong.Token) => {
      if (token && token.address) {
        tokenData.set(token.address, token);
      }
    });
  }

  return {
    enabledTokens,
    tokenData,
    tokens: Array.from(tokenData.values()),
    isAuthenticated: !!serialized.isAuthenticated,
    lastUpdated: serialized.lastUpdated || Date.now(),
  };
}

function createUserTokensStore() {
  // Current principal will be loaded asynchronously
  let currentPrincipal: string | null = null;
  
  // Default initial state
  const defaultState: UserTokensState = {
    enabledTokens: new Set<string>(),
    tokenData: new Map<string, Kong.Token>(),
    tokens: [],
    isAuthenticated: false,
    lastUpdated: Date.now(),
  };
  
  // Create the main state store
  const state = writable<UserTokensState>(defaultState);
  
  // Create derived stores for specific aspects of the state
  const enabledTokens = derived(state, $state => $state.enabledTokens);
  const tokens = derived(state, $state => Array.from($state.tokenData.values()));
  const isAuthenticated = derived(state, $state => $state.isAuthenticated);
  const tokenData = derived(state, $state => $state.tokenData);
  
  // Storage key helper
  const getStorageKey = (principal?: string) => {
    const id = principal || currentPrincipal || 'default';
    return `${id}`;
  };
  
  // Debounced storage update
  const debouncedUpdateStorage = debounce(async (newState: UserTokensState, principal?: string) => {
    if (!browser) return;
    
    try {
      const key = getStorageKey(principal);
      const serialized = safeStringify(serializeState(newState));
      localStorage.setItem(`${STORAGE_KEY_PREFIX}:${key}`, JSON.stringify(serialized));
    } catch (error) {
      console.error('[UserTokens] Error updating storage:', error);
    }
  }, 300);

  // Helper function to load state from storage
  const loadFromStorage = async (principal?: string): Promise<UserTokensState> => {
    if (!browser) return defaultState;
    
    try {
      const key = getStorageKey(principal);
      const stored = localStorage.getItem(`${STORAGE_KEY_PREFIX}:${key}`);
      const storedData = stored ? JSON.parse(stored) : null;
      
      // Otherwise deserialize using the new format
      return deserializeState(storedData);
    } catch (error) {
      console.error('[UserTokens] Error loading from storage:', error);
      return defaultState;
    }
  };
  
  // Helper function to check if principal has saved tokens
  const hasSavedTokens = async (principalId: string): Promise<boolean> => {
    // Always return false to force loading from backend
    // This ensures consistency between logged in and logged out states
    return false;
  };
  
  // Load default tokens if none are found
  const loadDefaultTokensIfNeeded = async (principalId?: string): Promise<UserTokensState> => {
    try {
      // ALWAYS load tokens from backend - it's the source of truth
      // This ensures consistency between logged in and logged out states
      const allTokens = await loadAllTokensFromBackend();
      
      // Filter out LP tokens (they have token_type: 'LP')
      const defaultTokensList = allTokens.filter(token => 
        token.token_type !== 'LP' &&
        !token.symbol.includes('_') // LP tokens typically have underscore in symbol like "ICP_ksUSDT"
      );
      
      console.log('[UserTokens] Loading backend tokens (auth state: ' + (principalId ? 'logged in' : 'logged out') + '):', defaultTokensList.length, 'out of', allTokens.length);
      
      // Create new state with backend tokens
      const enabledTokensSet = new Set<string>();
      const tokenDataMap = new Map<string, Kong.Token>();
      
      // ALWAYS enable all non-LP tokens from backend
      // This ensures consistency regardless of auth state
      defaultTokensList.forEach(token => {
        tokenDataMap.set(token.address, token);
        tokenDetailsCache.set(token.address, token);
        enabledTokensSet.add(token.address);
      });
      
      const newState: UserTokensState = {
        enabledTokens: enabledTokensSet,
        tokens: Array.from(tokenDataMap.values()),
        tokenData: tokenDataMap,
        isAuthenticated: !!principalId,
        lastUpdated: Date.now(),
      };
      
      // Save to storage if we have a principal
      if (principalId || currentPrincipal) {
        await debouncedUpdateStorage(newState, principalId);
      }
      
      return newState;
    } catch (error) {
      console.error('[UserTokens] Error loading default tokens:', error);
      return {
        ...defaultState,
        isAuthenticated: !!principalId,
        lastUpdated: Date.now(),
      };
    }
  };
  
  // Prevent multiple initializations
  let isInitializing = false;
  let isInitialized = false;
  
  // Initialize current principal
  const initializePrincipal = async () => {
    if (!browser) return;
    
    // Prevent multiple concurrent initializations
    if (isInitializing || isInitialized) {
      console.log('[UserTokens] Skipping initialization - already initialized or in progress');
      return;
    }
    
    isInitializing = true;
    
    try {
      const stored = localStorage.getItem(CURRENT_PRINCIPAL_KEY);
      currentPrincipal = stored ? JSON.parse(stored) : null;
      
      // FORCE reload from backend by clearing cache for development
      console.log('[UserTokens] Force loading fresh tokens from backend...');
      
      // Test direct local canister connection
      if (process.env.DFX_NETWORK === 'local') {
        console.log('[UserTokens] Testing direct local canister connection...');
        try {
          await testLocalCanisterConnection();
        } catch (error) {
          console.error('[UserTokens] Direct connection test failed:', error);
        }
      }
      
      const defaultTokensState = await loadDefaultTokensIfNeeded();
      state.set(defaultTokensState);
      isInitialized = true;
    } catch (error) {
      console.error('[UserTokens] Error initializing principal:', error);
      // Fallback to default tokens on error
      try {
        const defaultTokensState = await loadDefaultTokensIfNeeded();
        state.set(defaultTokensState);
        isInitialized = true;
      } catch (fallbackError) {
        console.error('[UserTokens] Fallback initialization also failed:', fallbackError);
      }
    } finally {
      isInitializing = false;
    }
  };
  
  // Initialize on store creation with delay to prevent immediate loops
  if (browser) {
    // Use setTimeout to prevent immediate execution during store creation
    setTimeout(() => {
      initializePrincipal();
    }, 0);
  }

  // Optimized token filtering logic with search index
  const createSearchIndex = memoize((tokens: Kong.Token[]) => {
    return tokens.reduce<Record<string, string>>((acc, token) => {
      if (token && token.address) {
        const searchTerms = `${token.symbol || ''} ${token.name || ''} ${token.address}`.toLowerCase();
        acc[token.address] = searchTerms;
      }
      return acc;
    }, {});
  });

  // Memoized search function using the search index
  const searchTokens = memoize((query: string, tokens: Kong.Token[]): Kong.Token[] => {
    if (!query) return tokens;
    
    const searchIndex = createSearchIndex(tokens);
    const lowerQuery = query.toLowerCase();
    
    return tokens.filter(token => {
      const terms = searchIndex[token.address];
      return terms && terms.includes(lowerQuery);
    });
  });

  // Lazy load token details
  const getTokenDetails = async (canisterId: string): Promise<Kong.Token | null> => {
    // Check cache first
    if (tokenDetailsCache.has(canisterId)) {
      return tokenDetailsCache.get(canisterId) || null;
    }
    
    // Check local store
    const currentState = get(state);
    if (currentState.tokenData.has(canisterId)) {
      const token = currentState.tokenData.get(canisterId);
      if (token) {
        tokenDetailsCache.set(canisterId, token);
        return token;
      }
    }
    
    // Fetch from API or backend as last resort
    try {
      let token: Kong.Token | null = null;
      
      if (USE_BACKEND_TOKENS) {
        // Try to find token in backend
        const allTokens = await loadAllTokensFromBackend();
        token = allTokens.find(t => t.address === canisterId) || null;
      } else {
        // Use existing API method
        const [fetchedToken] = await fetchTokensByCanisterId([canisterId]);
        token = fetchedToken || null;
      }
      
      if (token) {
        tokenDetailsCache.set(canisterId, token);
        
        // Update store with this token
        state.update(state => {
          const newTokenData = new Map(state.tokenData);
          newTokenData.set(canisterId, token);
          const newState = {
            ...state,
            tokenData: newTokenData,
            lastUpdated: Date.now(),
          };
          debouncedUpdateStorage(newState);
          return newState;
        });
        
        return token;
      }
    } catch (error) {
      console.error(`[UserTokens] Error fetching token details for ${canisterId}:`, error);
    }
    
    return null;
  };

  // Load visible tokens with priority queue
  const loadVisibleTokens = async (visibleTokenIds: string[]): Promise<void> => {
    // Skip if no visible tokens
    if (!visibleTokenIds.length) return;
    
    // Create a copy of the array for our queue
    const queue = [...visibleTokenIds];
    const batchSize = 5;
    
    // Function to load a batch of tokens
    const loadBatch = async () => {
      if (queue.length === 0) return;
      
      const batch = queue.splice(0, batchSize);
      const tokenPromises = batch.map(id => getTokenDetails(id));
      
      try {
        await Promise.all(tokenPromises);
        
        // If there are more tokens to load, schedule the next batch
        if (queue.length > 0) {
          setTimeout(loadBatch, 100);
        }
      } catch (error) {
        console.error('[UserTokens] Error loading token batch:', error);
      }
    };
    
    // Start loading the first batch
    await loadBatch();
  };

  // Retry configuration
  const MAX_RETRIES = 3;
  const RETRY_DELAY_MS = 1000;
  let retryCount = 0;
  let lastRetryTime = 0;

  // Load all tokens from backend with retry logic
  const loadAllTokensFromBackend = async (): Promise<Kong.Token[]> => {
    const currentTime = Date.now();
    
    // Reset retry count if enough time has passed
    if (currentTime - lastRetryTime > 60000) { // 1 minute cooldown
      retryCount = 0;
    }
    
    // Stop retrying if max attempts reached
    if (retryCount >= MAX_RETRIES) {
      console.warn('[UserTokens] Max retries reached for backend token loading. Skipping until cooldown.');
      return [];
    }
    
    try {
      const tokens = await BackendTokenService.fetchTokens();
      console.log('[UserTokens] Loaded tokens from backend:', tokens.length);
      
      // Reset retry count on success
      retryCount = 0;
      
      // Update cache
      tokens.forEach(token => {
        if (token && token.address) {
          tokenDetailsCache.set(token.address, token);
        }
      });
      
      return tokens;
    } catch (error) {
      retryCount++;
      lastRetryTime = currentTime;
      console.error(`[UserTokens] Error loading tokens from backend (attempt ${retryCount}/${MAX_RETRIES}):`, error);
      
      // Add exponential backoff delay
      if (retryCount < MAX_RETRIES) {
        const delay = RETRY_DELAY_MS * Math.pow(2, retryCount - 1);
        console.log(`[UserTokens] Retrying in ${delay}ms...`);
        await new Promise(resolve => setTimeout(resolve, delay));
      }
      
      return [];
    }
  };

  // Prevent multiple concurrent refresh operations
  let isRefreshing = false;
  
  // Refresh token data
  const refreshTokenData = async () => {
    // Prevent multiple concurrent refreshes
    if (isRefreshing) {
      console.log('[UserTokens] Refresh already in progress, skipping...');
      return;
    }
    
    const currentState = get(state);
    const canisterIds = Array.from(currentState.enabledTokens);
    
    if (canisterIds.length === 0) return;
    
    isRefreshing = true;
    
    try {
      let tokens: Kong.Token[] = [];
      
      if (USE_BACKEND_TOKENS) {
        // Load all tokens from backend and filter by enabled
        const allTokens = await loadAllTokensFromBackend();
        tokens = allTokens.filter(token => canisterIds.includes(token.address));
      } else {
        // Use existing API method
        tokens = await fetchTokensByCanisterId(canisterIds);
      }
      
      state.update(state => {
        const newTokenData = new Map(state.tokenData);
        
        tokens.forEach(token => {
          if (token && token.address) {
            newTokenData.set(token.address, token);
            tokenDetailsCache.set(token.address, token);
          }
        });
        
        const newState = {
          ...state,
          tokenData: newTokenData,
          lastUpdated: Date.now(),
        };
        
        debouncedUpdateStorage(newState);
        return newState;
      });
    } catch (error) {
      console.error('[UserTokens] Error refreshing token data:', error);
    } finally {
      isRefreshing = false;
    }
  };

  return {
    subscribe: state.subscribe,
    
    // Directly export the derived stores (readables)
    enabledTokens: enabledTokens,
    tokens: tokens,
    isAuthenticated: isAuthenticated,
    tokenData: tokenData,
    
    reset: () => {
      const newState = {
        enabledTokens: new Set<string>(),
        tokenData: new Map<string, Kong.Token>(),
        tokens: [],
        isAuthenticated: false,
        lastUpdated: Date.now(),
      };
      state.set(newState);
      debouncedUpdateStorage(newState);
      return Promise.resolve();
    },
    
    setPrincipal: (principalId: string | null) => {
      return new Promise<void>((resolve, reject) => {
        try {
          if (browser && principalId) {
            Promise.resolve(localStorage.setItem(CURRENT_PRINCIPAL_KEY, JSON.stringify(principalId)))
              .then(() => {
                currentPrincipal = principalId;
                return loadDefaultTokensIfNeeded(principalId);
              })
              .then((loadedState) => {
                state.set({...loadedState, isAuthenticated: true});
                resolve();
              })
              .catch(reject);
          } else if (browser && principalId === null) {
            Promise.resolve(localStorage.removeItem(CURRENT_PRINCIPAL_KEY))
              .then(() => {
                currentPrincipal = null;
                return loadDefaultTokensIfNeeded();
              })
              .then((loadedState) => {
                state.set({...loadedState, isAuthenticated: false});
                resolve();
              })
              .catch(reject);
          } else {
            resolve();
          }
        } catch (error) {
          console.error('[UserTokens] Error setting principal:', error);
          reject(error);
        }
      });
    },
    
    enableToken: async (token: Kong.Token) => {
      if (!token || !token.address) return;
      
      state.update(state => {
        // Create copies of current state
        const newEnabledTokens = new Set(state.enabledTokens);
        const newTokenData = new Map(state.tokenData);
        
        // Update state
        newEnabledTokens.add(token.address);
        newTokenData.set(token.address, token);
        
        // Update cache
        tokenDetailsCache.set(token.address, token);
        
        const newState = {
          ...state,
          enabledTokens: newEnabledTokens,
          tokenData: newTokenData,
          lastUpdated: Date.now(),
        };

        debouncedUpdateStorage(newState);
        return newState;
      });
      
      // Subscribe to Solana token balance updates via WebSocket
      if (token.chain === 'Solana') {
        await solanaPollingService.subscribeToToken(token);
      }
    },
    
    enableTokens: async (tokens: Kong.Token[]) => {
      if (!tokens.length) return;
      
      state.update(state => {
        // Create copies of current state
        const newEnabledTokens = new Set(state.enabledTokens);
        const newTokenData = new Map(state.tokenData);
        
        // Update state with batch operations
        tokens.forEach(token => {
          if (token && token.address) {
            newEnabledTokens.add(token.address);
            newTokenData.set(token.address, token);
            tokenDetailsCache.set(token.address, token);
          }
        });
        
        const newState = {
          ...state,
          enabledTokens: newEnabledTokens,
          tokenData: newTokenData,
          lastUpdated: Date.now(),
        };
        
        debouncedUpdateStorage(newState);
        return newState;
      });
      
      // Subscribe to Solana token balance updates via Polling Service
      const solanaTokens = tokens.filter(token => token.chain === 'Solana');
      for (const token of solanaTokens) {
        await solanaPollingService.subscribeToToken(token);
      }
    },
    
    disableToken: async (canisterId: string) => {
      // Get the token before updating state
      const currentState = get(state);
      const token = currentState.tokenData.get(canisterId);
      
      state.update(state => {
        // Create copies of current state
        const newEnabledTokens = new Set(state.enabledTokens);
        const newTokenData = new Map(state.tokenData);
        
        // Update state - only remove from enabledTokens, keep in tokenData
        newEnabledTokens.delete(canisterId);
        // Don't remove from tokenData: newTokenData.delete(canisterId);
        
        const newState = {
          ...state,
          enabledTokens: newEnabledTokens,
          tokenData: newTokenData,
          lastUpdated: Date.now(),
        };

        debouncedUpdateStorage(newState);
        return newState;
      });
      
      // Unsubscribe from Solana token balance updates via WebSocket
      if (token && token.chain === 'Solana') {
        await solanaPollingService.unsubscribeFromToken(token);
      }
    },
    
    isTokenEnabled: (canisterId: string): Promise<boolean> => {
      return new Promise<boolean>((resolve) => {
        if (!browser) {
          resolve(true); // Default to enabled if not in browser
          return;
        }
        
        const currentState = get(state);
        resolve(currentState.enabledTokens.has(canisterId));
      });
    },
    
    refreshTokenData,
    isDefaultToken: (canisterId: string) => {
      // Check if token exists in backend tokens
      const currentState = get(state);
      return currentState.tokenData.has(canisterId);
    },
    hasSavedTokens,
    loadDefaultTokensIfNeeded,
    loadAllTokensFromBackend,
    
    // New search utility with memoization
    searchTokens: (query: string) => {
      const currentState = get(state);
      const allTokens = Array.from(currentState.tokenData.values());
      return searchTokens(query, allTokens);
    },
    
    // Load visible tokens with priority queue
    loadVisibleTokens,
    
    // Get token details with caching
    getTokenDetails,
    
    // Sync methods
    analyzeUserTokens: async (principalId: string): Promise<SyncTokensResult> => {
      try {
        // Get token candidates
        const syncResults = await analyzeTokens(principalId);
        
        // Find the full token objects for tokens to remove
        const currentState = get(state);
        const tokensToRemoveObjects = syncResults.tokensToRemove
          .map(canisterId => currentState.tokenData.get(canisterId))
          .filter((token): token is Kong.Token => token !== undefined);
        
        return {
          tokensToAdd: syncResults.tokensToAdd,
          tokensToRemove: tokensToRemoveObjects,
          syncStatus: syncResults.stats
        };
      } catch (error) {
        console.error("Error analyzing user tokens:", error);
        return {
          tokensToAdd: [],
          tokensToRemove: [],
          syncStatus: { added: 0, removed: 0 }
        };
      }
    },
    
    applyTokenSync: async (tokensToAdd: Kong.Token[], tokensToRemove: Kong.Token[]): Promise<{ added: number; removed: number }> => {
      try {
        // Extract canister IDs from tokensToRemove for the API call
        const tokensToRemoveIds = tokensToRemove.map(token => token.address);
        
        // Apply the changes via API
        const result = await applyTokenChanges(tokensToAdd, tokensToRemoveIds);
        
        // Also update local state
        state.update(state => {
          const newEnabledTokens = new Set(state.enabledTokens);
          const newTokenData = new Map(state.tokenData);
          
          // Add new tokens
          tokensToAdd.forEach(token => {
            if (token && token.address) {
              newEnabledTokens.add(token.address);
              newTokenData.set(token.address, token);
              tokenDetailsCache.set(token.address, token);
            }
          });
          
          // Remove tokens
          tokensToRemoveIds.forEach(id => {
            newEnabledTokens.delete(id);
            newTokenData.delete(id);
          });
          
          const newState = {
            ...state,
            enabledTokens: newEnabledTokens,
            tokenData: newTokenData,
            lastUpdated: Date.now(),
          };
          
          debouncedUpdateStorage(newState);
          return newState;
        });
        
        return result;
      } catch (error) {
        console.error("Error applying token changes:", error);
        return { added: 0, removed: 0 };
      }
    }
  };
}

export const userTokens = createUserTokensStore();
