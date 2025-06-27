// BackendTokenService.ts
// Service to fetch tokens and pools directly from kong_backend canister

import { swapActor } from "$lib/stores/auth";
import type { TokenReply, PoolReply } from "../../../../../declarations/kong_backend/kong_backend.did";
import { LocalActorService } from '../actors/LocalActorService';
import { IcrcMetadataService } from './IcrcMetadataService';

export class BackendTokenService {
  /**
   * Fetches all tokens from kong_backend
   */
  static async fetchTokens(symbol?: string): Promise<Kong.Token[]> {
    try {
      
      // Use LocalActorService for local development, PNP for production
      let actor;
      if (process.env.DFX_NETWORK === 'local') {
        console.log('[BackendTokenService] Using LocalActorService for local development');
        actor = await LocalActorService.getKongBackendActor();
      } else {
        actor = swapActor({ anon: true, requiresSigning: false });
      }
      
      const result = await actor.tokens(symbol ? [symbol] : []);
      console.log('[BackendTokenService] Raw result from kong_backend:', result);
      console.log('[BackendTokenService] Result type:', typeof result);
      
      if ("Ok" in result) {
        const tokens = result.Ok.map(this.transformBackendToken).filter(Boolean) as Kong.Token[];
        console.log('[BackendTokenService] Transformed tokens count:', tokens.length);
        console.log('[BackendTokenService] First few tokens:', tokens.slice(0, 5).map(t => `${t.symbol} (${t.address})`));
        
        if (tokens.length > 50) {
          console.warn('[BackendTokenService] ⚠️  WARNING: Got', tokens.length, 'tokens - this suggests calling MAINNET instead of local!');
        } else {
          console.log('[BackendTokenService] ✅ Token count suggests local canister');
        }
        
        // Fetch logos asynchronously for IC tokens
        this.fetchLogosForTokens(tokens);
        
        return tokens;
      }
      
      throw new Error("Err" in result ? result.Err : "Failed to fetch tokens");
    } catch (error) {
      console.error("Error fetching tokens from backend:", error);
      throw error;
    }
  }

  /**
   * Fetches all pools from kong_backend
   */
  static async fetchPools(symbol?: string): Promise<PoolReply[]> {
    try {
      // Use LocalActorService for local development, PNP for production
      let actor;
      if (process.env.DFX_NETWORK === 'local') {
        actor = await LocalActorService.getKongBackendActor();
      } else {
        actor = swapActor({ anon: true, requiresSigning: false });
      }
      
      const result = await actor.pools(symbol ? [symbol] : []);
      
      if ("Ok" in result) {
        return result.Ok;
      }
      
      throw new Error("Err" in result ? result.Err : "Failed to fetch pools");
    } catch (error) {
      console.error("Error fetching pools from backend:", error);
      throw error;
    }
  }

  /**
   * Transforms backend token response to frontend Kong.Token format
   */
  private static transformBackendToken(token: TokenReply): Kong.Token | null {
    try {
      // Handle IC tokens
      if ("IC" in token) {
        const icToken = token.IC;
        return {
          id: Number(icToken.token_id),
          token_id: Number(icToken.token_id),
          name: icToken.name,
          symbol: icToken.symbol,
          address: icToken.canister_id,
          fee: Number(icToken.fee),
          fee_fixed: icToken.fee.toString(),
          decimals: Number(icToken.decimals),
          token_type: 'IC',
          chain: 'ICP',
          standards: BackendTokenService.getICStandards(icToken),
          logo_url: BackendTokenService.getTokenLogoUrl(icToken.symbol, 'ICP'),
          metrics: BackendTokenService.createEmptyMetrics(),
          timestamp: Date.now(),
        };
      }
      
      // Handle Solana tokens
      if ("Solana" in token) {
        const solanaToken = token.Solana;
        return {
          id: Number(solanaToken.token_id),
          token_id: Number(solanaToken.token_id),
          name: solanaToken.name,
          symbol: solanaToken.symbol,
          address: solanaToken.mint_address,
          fee: Number(solanaToken.fee),
          fee_fixed: solanaToken.fee.toString(),
          decimals: Number(solanaToken.decimals),
          token_type: solanaToken.is_spl_token ? 'SPL' : 'SOL', // Use 'SOL' for native SOL
          chain: 'Solana',
          standards: [], // Solana tokens don't have ICRC standards
          logo_url: BackendTokenService.getTokenLogoUrl(solanaToken.symbol, 'Solana'),
          metrics: BackendTokenService.createEmptyMetrics(),
          timestamp: Date.now(),
        };
      }
      
      // Handle LP tokens
      if ("LP" in token) {
        const lpToken = token.LP;
        return {
          id: Number(lpToken.token_id),
          token_id: Number(lpToken.token_id),
          name: lpToken.name,
          symbol: lpToken.symbol,
          address: lpToken.address,
          fee: Number(lpToken.fee),
          fee_fixed: lpToken.fee.toString(),
          decimals: Number(lpToken.decimals),
          token_type: 'LP',
          chain: 'ICP', // LP tokens are always on IC
          standards: [],
          logo_url: '',
          metrics: BackendTokenService.createEmptyMetrics(),
          timestamp: Date.now(),
        };
      }
      
      return null;
    } catch (error) {
      console.error("Error transforming token:", error, token);
      return null;
    }
  }

  /**
   * Get ICRC standards array from IC token
   */
  private static getICStandards(icToken: any): string[] {
    const standards: string[] = [];
    
    // Log the token info for debugging
    if (icToken.symbol === 'ICP' || icToken.symbol === 'ckUSDT' || icToken.symbol === 'ksUSDT') {
      console.log(`[BackendTokenService] Token ${icToken.symbol} standards:`, {
        icrc1: icToken.icrc1,
        icrc2: icToken.icrc2,
        icrc3: icToken.icrc3
      });
    }
    
    if (icToken.icrc1) standards.push('ICRC-1');
    if (icToken.icrc2) standards.push('ICRC-2');
    if (icToken.icrc3) standards.push('ICRC-3');
    return standards;
  }
  
  /**
   * Get logo URL for known tokens
   */
  private static getTokenLogoUrl(symbol: string, chain?: string): string {
    // For Solana tokens, use static assets
    if (chain === 'Solana') {
      const solanaLogoMap: Record<string, string> = {
        'SOL': '/tokens/solana_logo.png',
        'USDC': '/tokens/sol_usdc.webp',
        // Add more Solana tokens as needed
      };
      
      if (solanaLogoMap[symbol]) {
        return solanaLogoMap[symbol];
      }
      
      // Default Solana token logo if not verified
      return '/tokens/not_verified.webp';
    }
    
    // For known IC tokens with static logos
    const icLogoMap: Record<string, string> = {
      'ICP': '/tokens/icp_logo.webp',
      'ckUSDT': '/tokens/ckusdt_logo.svg',
      'KONG': '/tokens/kong_logo.png',
      // Add more IC tokens with static logos as needed
    };
    
    if (icLogoMap[symbol]) {
      return icLogoMap[symbol];
    }
    
    // For other IC tokens, return empty string - logos will be fetched from metadata
    return '';
  }

  /**
   * Create empty metrics object
   */
  private static createEmptyMetrics(): FE.TokenMetrics {
    return {
      total_supply: '0',
      price: '0',
      volume_24h: '0',
      market_cap: '0',
      tvl: '0',
      updated_at: new Date().toISOString(),
      price_change_24h: '0',
    };
  }

  /**
   * Format token ID for backend calls
   * Backend accepts: "SOL", "USDC", "ksUSDT", or with optional chain prefix
   */
  static formatTokenId(token: Kong.Token): string {
    // For Solana tokens, just use the symbol
    if (token.chain === 'Solana') {
      return token.symbol;
    }
    
    // For IC tokens, use the symbol
    // Backend's get_by_token function handles both symbol and address lookups
    return token.symbol;
  }

  /**
   * Fetch logos for IC tokens asynchronously
   */
  private static async fetchLogosForTokens(tokens: Kong.Token[]): Promise<void> {
    // Filter IC tokens that don't have logos yet and aren't LP tokens
    const icTokensWithoutLogos = tokens.filter(
      token => token.chain === 'ICP' && 
               (!token.logo_url || token.logo_url === '') && 
               token.address && 
               token.token_type !== 'LP' &&
               !token.address.includes('_') // Skip pool addresses like "2_1"
    );
    
    console.log('[BackendTokenService] Fetching logos for', icTokensWithoutLogos.length, 'IC tokens');
    
    // Fetch logos in parallel
    const logoPromises = icTokensWithoutLogos.map(async (token) => {
      try {
        const logoUrl = await IcrcMetadataService.getTokenLogoUrl(token.address);
        if (logoUrl) {
          // Update the token's logo_url
          token.logo_url = logoUrl;
          console.log('[BackendTokenService] Found logo for', token.symbol);
        }
      } catch (error) {
        console.error('[BackendTokenService] Error fetching logo for', token.symbol, ':', error);
      }
    });
    
    // Wait for all logos to be fetched
    await Promise.all(logoPromises);
  }
  
  /**
   * Get Kong's Solana address for receiving transfers
   */
  static async getKongSolanaAddress(): Promise<string> {
    try {
      // Use LocalActorService for local development, PNP for production
      let actor;
      if (process.env.DFX_NETWORK === 'local') {
        actor = await LocalActorService.getKongBackendActor();
      } else {
        actor = swapActor({ anon: true, requiresSigning: false });
      }
      
      const result = await actor.get_solana_address();
      
      if ("Ok" in result) {
        return result.Ok;
      }
      
      throw new Error("Err" in result ? result.Err : "Failed to get Solana address");
    } catch (error) {
      console.error("Error getting Kong Solana address:", error);
      throw error;
    }
  }
}