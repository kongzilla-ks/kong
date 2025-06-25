// UnifiedPoolService.ts
// Unified service that always uses kong_backend for pools

import { BackendTokenService } from '../tokens/BackendTokenService';

export class UnifiedPoolService {
  /**
   * Fetch all pools - always from backend
   */
  static async fetchPools(): Promise<BE.Pool[]> {
    try {
      // Use the BackendTokenService to get pools (it has fetchPools method)
      const pools = await BackendTokenService.fetchPools();
      
      // Transform the backend pool format to frontend format if needed
      return pools.map(this.transformPoolToFrontend);
    } catch (error) {
      console.error('[UnifiedPoolService] Error fetching pools:', error);
      return [];
    }
  }

  /**
   * Search pools by query - always from backend
   */
  static async searchPools(query: string): Promise<BE.Pool[]> {
    try {
      const allPools = await this.fetchPools();
      
      if (!query) return allPools;
      
      const lowerQuery = query.toLowerCase();
      return allPools.filter(pool => 
        pool.symbol.toLowerCase().includes(lowerQuery) ||
        pool.name?.toLowerCase().includes(lowerQuery) ||
        pool.address_0?.toLowerCase().includes(lowerQuery) ||
        pool.address_1?.toLowerCase().includes(lowerQuery)
      );
    } catch (error) {
      console.error('[UnifiedPoolService] Error searching pools:', error);
      return [];
    }
  }

  /**
   * Get pool by ID - always from backend
   */
  static async getPoolById(poolId: string): Promise<BE.Pool | null> {
    try {
      const allPools = await this.fetchPools();
      return allPools.find(pool => pool.pool_id.toString() === poolId) || null;
    } catch (error) {
      console.error('[UnifiedPoolService] Error getting pool by ID:', error);
      return null;
    }
  }

  /**
   * Transform backend pool response to frontend Pool format
   */
  private static transformPoolToFrontend(pool: any): BE.Pool {
    // If it's already in the right format, return as-is
    if (pool.pool_id && pool.symbol) {
      return pool;
    }

    // Transform if needed based on backend format
    return {
      pool_id: pool.pool_id || 0,
      symbol: pool.symbol || '',
      name: pool.name || '',
      address_0: pool.address_0 || '',
      address_1: pool.address_1 || '',
      balance_0: pool.balance_0 || '0',
      balance_1: pool.balance_1 || '0',
      lp_fee_0: pool.lp_fee_0 || '0',
      lp_fee_1: pool.lp_fee_1 || '0',
      lp_fee_bps: pool.lp_fee_bps || 0,
      fee_0: pool.fee_0 || '0',
      fee_1: pool.fee_1 || '0',
      fee_bps: pool.fee_bps || 0,
      tvl: pool.tvl || '0',
      volume_24h: pool.volume_24h || '0',
      price: pool.price || '0',
      on_kong: pool.on_kong || false,
      ...pool // Include any additional fields
    };
  }
}

// Export the service as default and named export for compatibility
export default UnifiedPoolService;

// Also create a function-based interface for compatibility with existing API client usage
export const fetchPools = () => UnifiedPoolService.fetchPools();
export const searchPools = (query: string) => UnifiedPoolService.searchPools(query);
export const getPoolById = (poolId: string) => UnifiedPoolService.getPoolById(poolId);