import { userTokens } from '$lib/stores/userTokens';
import { auth } from '$lib/stores/auth';
import { currentUserBalancesStore } from '$lib/stores/balancesStore';
import { get } from 'svelte/store';
import { Connection, PublicKey, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, AccountLayout } from '@solana/spl-token';
import type { Kong } from '$lib/types';

// Hardcoded Solana RPC endpoints
const SOLANA_RPC_ENDPOINT = 'https://mainnet.solana.validationcloud.io/v1/vEH0znkrmOFeAxXCJAyIGLQFSPVY38NzDe2NkNvNQt4';
const SOLANA_WS_ENDPOINT = 'wss://mainnet.solana.validationcloud.io/v1/vEH0znkrmOFeAxXCJAyIGLQFSPVY38NzDe2NkNvNQt4';

class SolanaPollingService {
  private pollingInterval: NodeJS.Timeout | null = null;
  private userSolanaAddress: string | null = null;
  private isPolling = false;
  private lastBalanceUpdate: number = 0;
  private lastBalanceHashes: Map<string, string> = new Map();
  private updateRateLimit = 3000; // 3 seconds minimum between updates
  private connection: Connection;
  private silentMode = false; // Reduce logging during swaps

  constructor() {
    this.connection = new Connection(SOLANA_RPC_ENDPOINT, 'confirmed');
  }

  /**
   * Enable/disable silent mode to reduce logging
   */
  setSilentMode(silent: boolean): void {
    this.silentMode = silent;
  }


  /**
   * Initialize polling for a specific wallet
   */
  async initialize(walletAddress: string): Promise<void> {
    try {
      console.log('[SolanaPolling] 🚀 Starting initialization for wallet:', walletAddress);
      
      // Cleanup any existing polling first
      if (this.isPolling) {
        console.log('[SolanaPolling] Cleaning up existing polling before reinitializing');
        await this.cleanup();
      }
      
      this.userSolanaAddress = walletAddress;
      this.startPolling();
      console.log('[SolanaPolling] ✅ Successfully initialized and started polling for wallet:', walletAddress);
    } catch (error) {
      console.error('[SolanaPolling] ❌ Failed to initialize:', error);
      throw error;
    }
  }

  /**
   * Start polling for all enabled Solana tokens
   */
  private startPolling(): void {
    if (this.isPolling) {
      console.log('[SolanaPolling] Polling already in progress, skipping start');
      return;
    }
    
    console.log('[SolanaPolling] 🔄 Starting balance polling...');
    this.isPolling = true;
    
    // Poll immediately
    console.log('[SolanaPolling] Running initial balance poll...');
    this.pollBalances();
    
    // Then poll every 3 seconds to reduce load
    this.pollingInterval = setInterval(() => {
      this.pollBalances();
    }, 3000);
    
    console.log('[SolanaPolling] ✅ Polling started successfully (3s interval)');
  }

  /**
   * Poll balances for all enabled Solana tokens with rate limiting
   */
  private async pollBalances(): Promise<void> {
    const now = Date.now();
    
    // Rate limit updates to prevent excessive polling
    if (now - this.lastBalanceUpdate < this.updateRateLimit) {
      return;
    }
    
    // Check if we have a valid Solana address
    if (!this.userSolanaAddress) {
      console.log('[SolanaPolling] No Solana address available');
      return;
    }

    try {
      const publicKey = new PublicKey(this.userSolanaAddress);
      
      if (!this.silentMode) {
        console.log('[SolanaPolling] 🔄 Fetching balances for:', this.userSolanaAddress);
      }
      
      // Fetch SOL balance and SPL token accounts in parallel
      const [solBalance, tokenAccounts] = await Promise.all([
        this.connection.getBalance(publicKey),
        this.connection.getTokenAccountsByOwner(publicKey, {
          programId: TOKEN_PROGRAM_ID
        })
      ]);

      // Convert SOL balance from lamports to SOL
      const solBalanceInSol = solBalance / LAMPORTS_PER_SOL;
      
      // Only log if balances have actually changed
      const solBalanceHash = this.hashBalance('SOL', solBalance);
      const splBalanceHash = this.hashBalance('SPL', tokenAccounts.value.length);
      
      if (this.lastBalanceHashes.get('SOL') !== solBalanceHash) {
        console.log('[SolanaPolling] SOL Balance changed:', solBalanceInSol, 'SOL');
        this.lastBalanceHashes.set('SOL', solBalanceHash);
      }
      
      if (this.lastBalanceHashes.get('SPL') !== splBalanceHash) {
        console.log('[SolanaPolling] SPL Token accounts found:', tokenAccounts.value.length);
        this.lastBalanceHashes.set('SPL', splBalanceHash);
      }

      // Process SOL balance
      await this.processSolBalance(solBalanceInSol);
      
      // Process SPL token balances
      await this.processSplTokenAccounts(tokenAccounts.value);
      
      this.lastBalanceUpdate = now;

    } catch (error) {
      console.error('[SolanaPolling] Error polling balances via Solana RPC:', error);
    }
  }

  /**
   * Hash balance data to detect changes
   */
  private hashBalance(type: string, balance: any): string {
    return `${type}_${JSON.stringify(balance)}`;
  }
  
  /**
   * Process SOL balance with change detection
   */
  private async processSolBalance(balanceInSOL: number): Promise<void> {
    const userTokensState = get(userTokens);
    const solToken = userTokensState.tokens.find(token => token.chain === 'Solana' && token.symbol === 'SOL');
    
    if (solToken && balanceInSOL != null) {
      const balanceInLamports = Math.floor(balanceInSOL * LAMPORTS_PER_SOL);
      
      // Check if balance actually changed before updating store
      const currentStoreBalance = get(currentUserBalancesStore)[solToken.address];
      const currentBalance = currentStoreBalance?.in_tokens || 0n;
      
      if (BigInt(balanceInLamports) !== currentBalance) {
        currentUserBalancesStore.updateBalance(solToken.address, {
          balance: BigInt(balanceInLamports),
          in_tokens: BigInt(balanceInLamports),
          token: solToken.symbol,
          in_usd: (balanceInSOL * Number(solToken.metrics?.price || 0)).toString()
        });
        
        if (!this.silentMode) {
          console.log(`[SolanaPolling] ✅ SOL balance updated: ${balanceInSOL} SOL (${balanceInLamports} lamports)`);
        }
      }
    }
  }

  /**
   * Process SPL token accounts with change detection
   */
  private async processSplTokenAccounts(tokenAccounts: any[]): Promise<void> {
    if (!Array.isArray(tokenAccounts) || tokenAccounts.length === 0) {
      console.log('[SolanaPolling] No SPL token accounts found');
      return;
    }
    
    const userTokensState = get(userTokens);
    const splTokens = userTokensState.tokens.filter(token => token.chain === 'Solana' && token.symbol !== 'SOL');
    const currentBalances = get(currentUserBalancesStore);
    
    // Parse token accounts to get mint and amount
    const tokenBalances = tokenAccounts.map(account => {
      try {
        // Decode the account data using SPL token layout
        const accountData = AccountLayout.decode(account.account.data);
        const mint = new PublicKey(accountData.mint).toString();
        const amount = accountData.amount; // This is already a bigint
        
        return { mint, amount };
      } catch (error) {
        console.warn('[SolanaPolling] Error parsing token account:', error);
        return null;
      }
    }).filter(Boolean);
    
    // Update all SPL tokens - set zero balance for tokens not found in accounts
    splTokens.forEach(token => {
      const tokenBalance = tokenBalances.find(tb => tb.mint === token.address);
      const amount = tokenBalance?.amount || 0n;
      
      const currentStoreBalance = currentBalances[token.address];
      const currentBalance = currentStoreBalance?.in_tokens || 0n;
      
      if (amount !== currentBalance) {
        const balance = Number(amount) / Math.pow(10, token.decimals);
        
        currentUserBalancesStore.updateBalance(token.address, {
          balance: amount,
          in_tokens: amount,
          token: token.symbol,
          in_usd: (balance * Number(token.metrics?.price || 0)).toString()
        });
        
        if (!this.silentMode) {
          if (amount > 0n) {
            console.log(`[SolanaPolling] ✅ ${token.symbol} balance updated: ${balance}`);
          } else {
            console.log(`[SolanaPolling] ✅ ${token.symbol} balance is zero`);
          }
        }
      }
    });
  }

  /**
   * Subscribe to a new token (compatibility with WebSocket interface)
   */
  async subscribeToToken(token: Kong.Token): Promise<void> {
    // No-op for polling - all enabled tokens are polled automatically
    console.log(`[SolanaPolling] Token enabled: ${token.symbol}`);
  }

  /**
   * Unsubscribe from a token (compatibility with WebSocket interface)
   */
  async unsubscribeFromToken(token: Kong.Token): Promise<void> {
    // No-op for polling - all enabled tokens are polled automatically
    console.log(`[SolanaPolling] Token disabled: ${token.symbol}`);
  }

  /**
   * Clean up polling
   */
  async cleanup(): Promise<void> {
    console.log('[SolanaPolling] Cleaning up');
    
    this.isPolling = false;
    
    if (this.pollingInterval) {
      clearInterval(this.pollingInterval);
      this.pollingInterval = null;
    }
    
    this.userSolanaAddress = null;
    this.lastBalanceUpdate = 0;
    this.lastBalanceHashes.clear();
  }
}

export const solanaPollingService = new SolanaPollingService();
