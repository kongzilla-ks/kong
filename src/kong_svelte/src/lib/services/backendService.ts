// services/backendService.ts
import { getActor } from '$lib/stores/walletStore';
import { walletValidator } from '$lib/validators/walletValidator';
import type { Token, Pool, SwapQuote, User } from '$lib/types/backend';
import type { Principal } from '@dfinity/principal';

class BackendService {
  private static instance: BackendService;
  private constructor() {}

  public static getInstance(): BackendService {
    if (!BackendService.instance) {
      BackendService.instance = new BackendService();
    }
    return BackendService.instance;
  }

  // Token Related Methods
  public async getTokens(): Promise<Token[]> {
    try {
      const actor = await getActor();
      const result = await actor.tokens(['']);
      
      if (result.Ok) {
        return result.Ok.map(token => {
          if ('IC' in token) {
            return {
              fee: token.IC.fee,
              decimals: token.IC.decimals,
              token: token.IC.token,
              tokenId: token.IC.token_id,
              chain: token.IC.chain,
              name: token.IC.name,
              canisterId: token.IC.canister_id,
              icrc1: token.IC.icrc1,
              icrc2: token.IC.icrc2, 
              icrc3: token.IC.icrc3,
              poolSymbol: token.IC.pool_symbol,
              symbol: token.IC.symbol,
              onKong: token.IC.on_kong
            };
          } else if ('LP' in token) {
            return {
              fee: token.LP.fee,
              decimals: token.LP.decimals,
              token: token.LP.token,
              tokenId: token.LP.token_id,
              chain: token.LP.chain,
              name: token.LP.name,
              address: token.LP.address,
              poolIdOf: token.LP.pool_id_of,
              poolSymbol: token.LP.pool_symbol,
              totalSupply: token.LP.total_supply,
              symbol: token.LP.symbol,
              onKong: token.LP.on_kong
            };
          }
        });
      }
      return [];
    } catch (error) {
      console.error('Error getting tokens:', error);
      throw error;
    }
  }


  // for LP i think
  public async getUserBalances(principal: Principal): Promise<Record<string, any>> {
    try {
      const actor = await getActor();
      const result = await actor.user_balances(['']);
      if (result.Ok) {
        const balances: Record<string, any> = {};
        result.Ok.forEach((lpToken) => {
          if ('LP' in lpToken) {
            const lp = lpToken.LP;
            balances[lp.symbol] = {
              balance: lp.balance,
              usdBalance: lp.usd_balance,
              token0Amount: lp.amount_0,
              token1Amount: lp.amount_1,
              token0Symbol: lp.symbol_0,
              token1Symbol: lp.symbol_1,
              token0UsdAmount: lp.usd_amount_0,
              token1UsdAmount: lp.usd_amount_1,
              timestamp: lp.ts
            };
          }
        });
        return balances;
      }
      return {};
    } catch (error) {
      console.error('Error getting user balances:', error);
      throw error;
    }
  }
  
  public async getTokenPrices(): Promise<Record<string, number>> {
    try {
        const actor = await getActor();
        // Use a default price map for now
        const defaultPrices = {
            "ICP": 1,
            "ckBTC": 1,
            "ckETH": 1,
            "ckUSDC": 1,
            "ckUSDT": 1,
            // Add other tokens as needed
        };
        return defaultPrices;
    } catch (error) {
        console.error('Error getting token prices:', error);
        throw error;
    }
}

  // User Related Methods
  public async getWhoami(): Promise<User> {
    await walletValidator.requireWalletConnection();
    try {
      const actor = await getActor();
      return await actor.get_user();
    } catch (error) {
      console.error('Error calling get_user method:', error);
      throw error;
    }
  }

  // Pool Related Methods
  public async getPools(): Promise<Pool[]> {
    try {
      const actor = await getActor();
      return await actor.pools([]);
    } catch (error) {
      console.error('Error calling pools method:', error);
      throw error;
    }
  }

  public async getPoolInfo(poolId: string): Promise<Pool> {
    try {
      const actor = await getActor();
      return await actor.pool_info(poolId);
    } catch (error) {
      console.error('Error getting pool info:', error);
      throw error;
    }
  }

  // Swap Related Methods
  public async getSwapQuote(params: {
    payToken: string;
    payAmount: bigint;
    receiveToken: string;
  }): Promise<SwapQuote> {
    try {
      const actor = await getActor();
      return await actor.swap_amounts(
        params.payToken,
        params.payAmount,
        params.receiveToken
      );
    } catch (error) {
      console.error('Error getting swap quote:', error);
      throw error;
    }
  }

  public async executeSwap(params: {
    payToken: string;
    payAmount: bigint;
    receiveToken: string;
    receiveAmount: bigint;
    slippage: number;
  }): Promise<{ requestId: string }> {
    await walletValidator.requireWalletConnection();
    try {
      const actor = await getActor();
      const result = await actor.swap_async({
        pay_token: params.payToken,
        pay_amount: params.payAmount,
        receive_token: params.receiveToken,
        receive_amount: [params.receiveAmount],
        max_slippage: [params.slippage],
        pay_tx_id: [],
        referred_by: []
      });
      return { requestId: result.Ok };
    } catch (error) {
      console.error('Error executing swap:', error);
      throw error;
    }
  }

  // Liquidity Related Methods
  public async addLiquidity(params: {
    token0: string;
    amount0: bigint;
    token1: string;
    amount1: bigint;
  }): Promise<{ requestId: string }> {
    await walletValidator.requireWalletConnection();
    try {
      const actor = await getActor();
      const result = await actor.add_liquidity_async({
        token_0: params.token0,
        amount_0: params.amount0,
        token_1: params.token1,
        amount_1: params.amount1,
        tx_id_0: [],
        tx_id_1: []
      });
      return { requestId: result.Ok };
    } catch (error) {
      console.error('Error adding liquidity:', error);
      throw error;
    }
  }

  public async removeLiquidity(params: {
    token0: string;
    token1: string;
    lpTokenAmount: bigint;
  }): Promise<{ requestId: string }> {
    await walletValidator.requireWalletConnection();
    try {
      const actor = await getActor();
      const result = await actor.remove_liquidity_async({
        token_0: params.token0,
        token_1: params.token1,
        remove_lp_token_amount: params.lpTokenAmount
      });
      return { requestId: result.Ok };
    } catch (error) {
      console.error('Error removing liquidity:', error);
      throw error;
    }
  }

  // Transaction Related Methods
  public async getTransactionHistory(principal: Principal): Promise<any[]> {
    try {
      const actor = await getActor();
      const result = await actor.txs([true]);
      return result.Ok || [];
    } catch (error) {
      console.error('Error getting transaction history:', error);
      throw error;
    }
  }

  public async getTransactionStatus(requestId: string): Promise<any> {
    try {
      const actor = await getActor();
      const result = await actor.requests([requestId]);
      return result.Ok?.[0];
    } catch (error) {
      console.error('Error getting transaction status:', error);
      throw error;
    }
  }

  // Token Approval Methods
  public async approveToken(params: {
    token: string;
    amount: bigint;
    spender: Principal;
  }): Promise<boolean> {
    await walletValidator.requireWalletConnection();
    try {
      const actor = await getActor();
      const result = await actor.icrc2_approve({
        amount: params.amount,
        spender: { owner: params.spender, subaccount: [] },
        expires_at: [BigInt(Date.now() * 1000000 + 60000000000)],
        expected_allowance: [],
        memo: [],
        fee: [],
        created_at_time: []
      });
      return !!result.Ok;
    } catch (error) {
      console.error('Error approving token:', error);
      throw error;
    }
  }
}

export const backendService = BackendService.getInstance();
