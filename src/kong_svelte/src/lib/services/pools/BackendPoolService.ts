// Service to fetch pools directly from kong_backend canister
import type { PoolReply } from '../../../../../declarations/kong_backend/kong_backend.did';
import { BackendTokenService } from '../tokens/BackendTokenService';
import { LocalActorService } from '../actors/LocalActorService';
import { swapActor } from '$lib/stores/auth';

export class BackendPoolService {
  /**
   * Fetches all pools from kong_backend
   */
  static async fetchPools(symbol?: string): Promise<BE.Pool[]> {
    try {
      console.log('[BackendPoolService] Fetching pools from kong_backend...');
      
      // Get pools from backend using BackendTokenService
      const poolReplies = await BackendTokenService.fetchPools(symbol);
      
      console.log('[BackendPoolService] Raw pool count:', poolReplies.length);
      
      // Transform backend pools to frontend format
      const pools = poolReplies.map(pool => this.transformBackendPool(pool)).filter(Boolean) as BE.Pool[];
      
      console.log('[BackendPoolService] Transformed pool count:', pools.length);
      console.log('[BackendPoolService] First few pools:', pools.slice(0, 3).map(p => `${p.symbol_0}/${p.symbol_1}`));
      
      return pools;
    } catch (error) {
      console.error('[BackendPoolService] Error fetching pools:', error);
      throw error;
    }
  }
  
  /**
   * Get pool totals (TVL, volume, etc)
   * Note: This method doesn't exist in the backend yet, so we calculate it from pools
   */
  static async getPoolTotals(): Promise<BE.PoolTotals> {
    try {
      const pools = await this.fetchPools();
      
      // Calculate totals from pools
      let totalTvl = 0;
      let totalVolume = 0;
      let totalFees = 0;
      
      pools.forEach(pool => {
        totalTvl += parseFloat(pool.tvl || '0');
        totalVolume += parseFloat(pool.total_volume_24h || '0');
        totalFees += parseFloat(pool.total_lp_fees_24h || '0');
      });
      
      return {
        total_tvl: totalTvl.toString(),
        total_volume: totalVolume.toString(),
        total_lp_fees: totalFees.toString(),
        unique_users: 0, // Not available from pools
        pools_count: pools.length,
      };
    } catch (error) {
      console.error('[BackendPoolService] Error calculating pool totals:', error);
      // Return default values on error
      return {
        total_tvl: '0',
        total_volume: '0',
        total_lp_fees: '0',
        unique_users: 0,
        pools_count: 0,
      };
    }
  }
  
  /**
   * Transform backend pool to frontend format
   */
  private static transformBackendPool(pool: PoolReply): BE.Pool | null {
    try {
      // Calculate TVL from balances (simplified calculation)
      // In reality, you'd need token prices to calculate proper TVL
      const balance0 = Number(pool.balance_0) / Math.pow(10, 8); // Assuming 8 decimals
      const balance1 = Number(pool.balance_1) / Math.pow(10, 8);
      const tvl = (balance0 * pool.price + balance1).toString();
      
      return {
        pool_id: Number(pool.pool_id),
        name: pool.name,
        symbol: pool.symbol,
        symbol_0: pool.symbol_0,
        symbol_1: pool.symbol_1,
        address: pool.lp_token_symbol, // Use lp_token_symbol as address
        address_0: pool.address_0,
        address_1: pool.address_1,
        chain_0: pool.chain_0,
        chain_1: pool.chain_1,
        balance_0: pool.balance_0.toString(),
        balance_1: pool.balance_1.toString(),
        lp_fee_0: pool.lp_fee_0.toString(),
        lp_fee_1: pool.lp_fee_1.toString(),
        price: pool.price.toString(),
        lp_total_supply: '0', // Not available from backend
        lp_token_symbol: pool.lp_token_symbol,
        total_volume_24h: '0', // Not available from backend
        total_lp_fees_24h: '0', // Not available from backend
        rolling_24h_volume: '0', // Not available from backend
        rolling_24h_lp_fees: '0', // Not available from backend
        tvl: tvl,
        apr: 0, // Not available from backend
        apy: 0, // Not available from backend
        lp_fee_bps: Number(pool.lp_fee_bps),
        updated_at: new Date().toISOString(),
      };
    } catch (error) {
      console.error('[BackendPoolService] Error transforming pool:', error, pool);
      return null;
    }
  }
}