// services/PoolService.ts
import { auth, requireWalletConnection} from '$lib/services/auth';
import { get } from 'svelte/store';
import { PoolResponseSchema, UserPoolBalanceSchema } from './poolSchema';
import { IcrcService } from '../icrc/IcrcService';
import { canisterId as kongBackendCanisterId } from '../../../../../declarations/kong_backend';
import { canisterIDLs } from '../pnp/PnpInitializer';
import { PoolSerializer } from './PoolSerializer';
import { createAnonymousActorHelper } from '$lib/utils/actorUtils';

export class PoolService {
  protected static instance: PoolService;

  public static getInstance(): PoolService {
    if (!PoolService.instance) {
      PoolService.instance = new PoolService();
    }
    return PoolService.instance;
  }

  // Data Fetching
  public static async fetchPoolsData(): Promise<BE.PoolResponse> {
    try {
      const actor = await createAnonymousActorHelper(kongBackendCanisterId, canisterIDLs.kong_backend);
      const result = await actor.pools([]);
      
      if (!result.Ok) {
        throw new Error('Failed to fetch pools');
      }

      return PoolSerializer.serializePoolsResponse(result.Ok);
    } catch (error) {
      console.error('Error fetching pools:', error);
      throw error;
    }
  }

  // Pool Operations
  public static async getPoolDetails(poolId: string): Promise<BE.Pool> {
    try {
      const actor =  await auth.pnp.getActor(kongBackendCanisterId, canisterIDLs.kong_backend, {anon: true});
      return await actor.get_by_pool_id(poolId);
    } catch (error) {
      console.error('Error fetching pool details:', error);
      throw new Error(`Failed to fetch details for pool ${poolId}`);
    }
  }

  /**
   * Calculate required amounts for adding liquidity
   */
  public static async calculateLiquidityAmounts(
    token0Symbol: string,
    amount0: bigint,
    token1Symbol: string
  ): Promise<any> {
    try {
      const actor =  await auth.pnp.getActor(kongBackendCanisterId, canisterIDLs.kong_backend, {anon: true, requiresSigning: false});
      const result = await actor.add_liquidity_amounts(
        token0Symbol,
        amount0,
        token1Symbol
      );
      
      if (!result.Ok) {
        throw new Error(result.Err || 'Failed to calculate liquidity amounts');
      }
      
      return result;
    } catch (error) {
      console.error('Error calculating liquidity amounts:', error);
      throw error;
    }
  }

  /**
   * Calculate amounts that would be received when removing liquidity
   */
  public static async calculateRemoveLiquidityAmounts(
    token0Symbol: string,
    token1Symbol: string,
    lpTokenAmount: bigint
  ): Promise<any> {
    try {
      const actor =  await auth.pnp.getActor(kongBackendCanisterId, canisterIDLs.kong_backend, {anon: true});
      const result = await actor.remove_liquidity_amounts(
        token0Symbol,
        token1Symbol,
        lpTokenAmount
      );
      
      if (!result.Ok) {
        throw new Error(result.Err || 'Failed to calculate removal amounts');
      }
      
      return result;
    } catch (error) {
      console.error('Error calculating removal amounts:', error);
      throw error;
    }
  }


  public static async addLiquidityAmounts(
    token0Symbol: string,
    amount0: bigint,
    token1Symbol: string
  ): Promise<any> {
    return this.calculateLiquidityAmounts(token0Symbol, amount0, token1Symbol);
  }

  /**
   * Add liquidity to a pool with ICRC2 approval
   */
  public static async addLiquidity(params: {
    token_0: FE.Token;
    amount_0: bigint;
    token_1: FE.Token;
    amount_1: bigint;
    tx_id_0?: number[];
    tx_id_1?: number[];
  }): Promise<bigint> {
    await requireWalletConnection();
    
    try {
      if (!params.token_0 || !params.token_1) {
        throw new Error('Invalid token configuration');
      }
      
      const [_approval0, _approval1, actor] = await Promise.all([
        IcrcService.checkAndRequestIcrc2Allowances(
          params.token_0,
          params.amount_0,
        ),
        IcrcService.checkAndRequestIcrc2Allowances(
          params.token_1,
          params.amount_1,
        ),
        auth.pnp.getActor(kongBackendCanisterId, canisterIDLs.kong_backend)
      ]);

      const result = await actor.add_liquidity_async({
        token_0: params.token_0.symbol,
        amount_0: params.amount_0,
        token_1: params.token_1.symbol,
        amount_1: params.amount_1,
        tx_id_0: params.tx_id_0 || [],
        tx_id_1: params.tx_id_1 || []
      });

      if ('Err' in result) {
        throw new Error(result.Err || 'Failed to add liquidity');
      }

      return result.Ok;
    } catch (error) {
      console.error('Error adding liquidity:', error);
      throw error;
    }
  }

  /**
   * Poll for request status
   */
  public static async pollRequestStatus(requestId: bigint): Promise<any> {
    try {
      const actor =  await auth.pnp.getActor(kongBackendCanisterId, canisterIDLs.kong_backend, {anon: false});
      const result = await actor.requests([requestId]);
      
      if (!result.Ok || result.Ok.length === 0) {
        throw new Error('Failed to get request status');
      }
      
      return result.Ok[0];
    } catch (error) {
      console.error('Error polling request status:', error);
      throw error;
    }
  }

  public static async removeLiquidity(params: any): Promise<string> {
    await requireWalletConnection();
    try {
      const actor =  await auth.pnp.getActor(kongBackendCanisterId, canisterIDLs.kong_backend, {anon: false});
      const result = await actor.remove_liquidity_async({
        token_0: params.token0,
        token_1: params.token1,
        remove_lp_token_amount: params.lpTokenAmount
      });
      return result.Ok;
    } catch (error) {
      console.error('Error removing liquidity:', error);
      throw new Error('Failed to remove liquidity');
    }
  }

  /**
   * Fetch the user's pool balances.
   */
  public static async fetchUserPoolBalances(): Promise<FE.UserPoolBalance[]> {
    try {
      const wallet = get(auth);
      console.log('[PoolService] Fetching user pool balances, wallet state:', {
        isConnected: wallet.isConnected,
        hasAccount: !!wallet.account,
        accountOwner: wallet.account?.owner
      });

      if (!wallet.isConnected || !wallet.account?.owner) {
        console.log('[PoolService] Wallet not connected or no account owner, returning empty balances');
        return [];
      }
      
      console.log('[PoolService] Creating actor...');
      const actor = await auth.pnp.getActor(kongBackendCanisterId, canisterIDLs.kong_backend, {anon: false, requiresSigning: false});
      
      if (!actor) {
        console.error('[PoolService] Actor creation failed');
        throw new Error('Actor not available');
      }
      
      console.log('[PoolService] Actor created successfully, fetching balances...');
      const balances = await actor.user_balances([]);
      console.log('[PoolService] Balances fetched successfully:', balances);
      
      return balances;
    } catch (error) {
      if (error.message?.includes('Anonymous user')) {
        console.log('[PoolService] Anonymous user detected, returning empty balances');
        return [];
      }
      console.error('[PoolService] Error fetching user pool balances:', error);
      throw error;
    }
  }
}
