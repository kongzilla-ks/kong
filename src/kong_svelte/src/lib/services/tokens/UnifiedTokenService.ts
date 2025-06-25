// UnifiedTokenService.ts
// Unified service that always uses kong_backend, providing same interface as API client

import { BackendTokenService } from './BackendTokenService';

export class UnifiedTokenService {
  /**
   * Fetch all tokens - always from backend
   */
  static async fetchTokens(): Promise<Kong.Token[]> {
    try {
      return await BackendTokenService.fetchTokens();
    } catch (error) {
      console.error('[UnifiedTokenService] Error fetching tokens:', error);
      return [];
    }
  }

  /**
   * Fetch tokens by canister IDs - always from backend
   */
  static async fetchTokensByCanisterId(canisterIds: string[]): Promise<Kong.Token[]> {
    try {
      // Filter out undefined, null, empty strings, and invalid canister IDs
      const validCanisterIds = canisterIds.filter(id => 
        id !== undefined && 
        id !== null && 
        typeof id === 'string' && 
        id.trim().length > 0
      );
      
      if (validCanisterIds.length === 0) {
        console.log(`[UnifiedTokenService] No valid canister IDs provided from:`, canisterIds);
        return [];
      }
      
      // Get all tokens from backend
      const allTokens = await BackendTokenService.fetchTokens();
      
      // Filter by requested canister IDs
      const requestedTokens = allTokens.filter(token => 
        validCanisterIds.includes(token.address)
      );
      
      console.log(`[UnifiedTokenService] Fetched ${requestedTokens.length} tokens for canister IDs:`, validCanisterIds);
      return requestedTokens;
    } catch (error) {
      console.error('[UnifiedTokenService] Error fetching tokens by canister ID:', error);
      return [];
    }
  }

  /**
   * Fetch token by symbol - always from backend
   */
  static async fetchTokenBySymbol(symbol: string): Promise<Kong.Token | null> {
    try {
      const tokens = await BackendTokenService.fetchTokens(symbol);
      return tokens.find(token => token.symbol === symbol) || null;
    } catch (error) {
      console.error('[UnifiedTokenService] Error fetching token by symbol:', error);
      return null;
    }
  }

  /**
   * Search tokens by query - always from backend
   */
  static async searchTokens(query: string): Promise<Kong.Token[]> {
    try {
      const allTokens = await BackendTokenService.fetchTokens();
      
      if (!query) return allTokens;
      
      const lowerQuery = query.toLowerCase();
      return allTokens.filter(token => 
        token.symbol.toLowerCase().includes(lowerQuery) ||
        token.name.toLowerCase().includes(lowerQuery) ||
        token.address.toLowerCase().includes(lowerQuery)
      );
    } catch (error) {
      console.error('[UnifiedTokenService] Error searching tokens:', error);
      return [];
    }
  }

  /**
   * Get token by address - always from backend
   */
  static async getTokenByAddress(address: string): Promise<Kong.Token | null> {
    try {
      const allTokens = await BackendTokenService.fetchTokens();
      return allTokens.find(token => token.address === address) || null;
    } catch (error) {
      console.error('[UnifiedTokenService] Error getting token by address:', error);
      return null;
    }
  }

  /**
   * Check if token exists - always from backend
   */
  static async tokenExists(address: string): Promise<boolean> {
    try {
      const token = await this.getTokenByAddress(address);
      return token !== null;
    } catch (error) {
      console.error('[UnifiedTokenService] Error checking token existence:', error);
      return false;
    }
  }

  /**
   * Fetch token metadata - always from backend
   */
  static async fetchTokenMetadata(address: string): Promise<Kong.Token | null> {
    try {
      return await this.getTokenByAddress(address);
    } catch (error) {
      console.error('[UnifiedTokenService] Error fetching token metadata:', error);
      return null;
    }
  }
}

// Export the service as default and named export for compatibility
export default UnifiedTokenService;

// Also create a function-based interface for compatibility with existing API client usage
export const fetchTokens = () => UnifiedTokenService.fetchTokens();
export const fetchTokensByCanisterId = (canisterIds: string[]) => UnifiedTokenService.fetchTokensByCanisterId(canisterIds);
export const fetchTokenBySymbol = (symbol: string) => UnifiedTokenService.fetchTokenBySymbol(symbol);
export const searchTokens = (query: string) => UnifiedTokenService.searchTokens(query);
export const fetchTokenMetadata = (address: string) => UnifiedTokenService.fetchTokenMetadata(address);