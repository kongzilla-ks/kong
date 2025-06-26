import { Connection, PublicKey } from '@solana/web3.js';
import { get } from 'svelte/store';
import { SOLANA_RPC_ENDPOINT, SOLANA_WS_ENDPOINT, WS_CONFIG, COMMITMENT_CONFIG } from '$lib/config/solana.config';
import { currentUserBalancesStore } from '$lib/stores/balancesStore';
import { userTokens } from '$lib/stores/userTokens';
import { toastStore } from '$lib/stores/toastStore';
import { formatBalance } from '$lib/utils/numberFormatUtils';
import type { AccountInfo, Context } from '@solana/web3.js';

interface SubscriptionInfo {
  id: number;
  address: string;
  type: 'native' | 'token';
  tokenSymbol?: string;
}

class SolanaWebSocketService {
  private connection: Connection | null = null;
  private subscriptions: Map<string, SubscriptionInfo> = new Map();
  private isConnected: boolean = false;
  private reconnectAttempts: number = 0;
  private reconnectTimer: NodeJS.Timeout | null = null;
  private heartbeatTimer: NodeJS.Timeout | null = null;
  private userPublicKey: PublicKey | null = null;

  constructor() {
    // Initialize connection with both HTTP and WebSocket endpoints
    this.connection = new Connection(SOLANA_RPC_ENDPOINT, {
      commitment: COMMITMENT_CONFIG.balance,
      wsEndpoint: SOLANA_WS_ENDPOINT,
    });
  }

  /**
   * Initialize WebSocket subscriptions for a user's wallet
   */
  async initialize(walletAddress: string): Promise<void> {
    console.log('[SolanaWebSocket] Initializing for wallet:', walletAddress);
    
    try {
      this.userPublicKey = new PublicKey(walletAddress);
      
      // Clean up any existing subscriptions
      await this.cleanup();
      
      // Subscribe to native SOL balance
      await this.subscribeToNativeBalance();
      
      // Subscribe to SPL token balances
      await this.subscribeToTokenBalances();
      
      // Start heartbeat to ensure connection stays alive
      this.startHeartbeat();
      
      this.isConnected = true;
      this.reconnectAttempts = 0;
      
      console.log('[SolanaWebSocket] Successfully initialized');
    } catch (error) {
      console.error('[SolanaWebSocket] Initialization error:', error);
      this.scheduleReconnect();
    }
  }

  /**
   * Subscribe to native SOL balance changes
   */
  private async subscribeToNativeBalance(): Promise<void> {
    if (!this.connection || !this.userPublicKey) return;

    try {
      const subscriptionId = this.connection.onAccountChange(
        this.userPublicKey,
        (accountInfo: AccountInfo<Buffer>, context: Context) => {
          this.handleNativeBalanceUpdate(accountInfo, context);
        },
        COMMITMENT_CONFIG.balance
      );

      this.subscriptions.set(this.userPublicKey.toBase58(), {
        id: subscriptionId,
        address: this.userPublicKey.toBase58(),
        type: 'native',
      });

      console.log('[SolanaWebSocket] Subscribed to native SOL balance');
    } catch (error) {
      console.error('[SolanaWebSocket] Failed to subscribe to native balance:', error);
    }
  }

  /**
   * Subscribe to SPL token balance changes
   */
  private async subscribeToTokenBalances(): Promise<void> {
    if (!this.connection || !this.userPublicKey) return;

    const tokens = get(userTokens);
    const solanaTokens = Array.from(tokens.tokenData.values()).filter(
      token => token.chain === 'Solana' && tokens.enabledTokens.has(token.address)
    );

    for (const token of solanaTokens) {
      try {
        // Get the associated token account address
        const { getAssociatedTokenAddress } = await import('@solana/spl-token');
        const tokenMint = new PublicKey(token.address);
        const associatedTokenAccount = await getAssociatedTokenAddress(
          tokenMint,
          this.userPublicKey
        );

        // Subscribe to token account changes
        const subscriptionId = this.connection.onAccountChange(
          associatedTokenAccount,
          (accountInfo: AccountInfo<Buffer>, context: Context) => {
            this.handleTokenBalanceUpdate(token, accountInfo, context);
          },
          COMMITMENT_CONFIG.balance
        );

        this.subscriptions.set(associatedTokenAccount.toBase58(), {
          id: subscriptionId,
          address: associatedTokenAccount.toBase58(),
          type: 'token',
          tokenSymbol: token.symbol,
        });

        console.log(`[SolanaWebSocket] Subscribed to ${token.symbol} balance`);
      } catch (error) {
        console.error(`[SolanaWebSocket] Failed to subscribe to ${token.symbol}:`, error);
      }
    }
  }

  /**
   * Handle native SOL balance update
   */
  private handleNativeBalanceUpdate(accountInfo: AccountInfo<Buffer>, context: Context): void {
    const lamports = accountInfo.lamports;
    const solBalance = lamports / 1e9; // Convert lamports to SOL

    console.log(`[SolanaWebSocket] SOL balance updated: ${solBalance} SOL`);

    // Find SOL token in the store
    const tokens = get(userTokens);
    const solToken = Array.from(tokens.tokenData.values()).find(
      token => token.chain === 'Solana' && token.symbol === 'SOL'
    );

    if (solToken) {
      // Update the balance in the store
      currentUserBalancesStore.updateBalance(solToken.address, solBalance.toString());
      
      // Show a subtle notification
      toastStore.success(`SOL balance updated: ${formatBalance(solBalance.toString(), 9)} SOL`);
    }
  }

  /**
   * Handle SPL token balance update
   */
  private async handleTokenBalanceUpdate(
    token: Kong.Token, 
    accountInfo: AccountInfo<Buffer>, 
    context: Context
  ): Promise<void> {
    try {
      // Parse token account data
      const { getAccount, TOKEN_PROGRAM_ID } = await import('@solana/spl-token');
      
      if (!accountInfo.owner.equals(TOKEN_PROGRAM_ID)) {
        console.warn(`[SolanaWebSocket] Account ${token.symbol} is not a token account`);
        return;
      }

      // Decode the token account data
      const data = accountInfo.data;
      if (data.length >= 72) {
        // Read amount from the account data (bytes 64-72)
        const amount = data.readBigUInt64LE(64);
        const decimals = token.decimals || 9;
        const balance = Number(amount) / Math.pow(10, decimals);

        console.log(`[SolanaWebSocket] ${token.symbol} balance updated: ${balance}`);

        // Update the balance in the store
        currentUserBalancesStore.updateBalance(token.address, balance.toString());
        
        // Show a subtle notification
        toastStore.success(`${token.symbol} balance updated: ${formatBalance(balance.toString(), decimals)} ${token.symbol}`);
      }
    } catch (error) {
      console.error(`[SolanaWebSocket] Error parsing ${token.symbol} balance:`, error);
    }
  }

  /**
   * Subscribe to a new token (called when a token is enabled)
   */
  async subscribeToToken(token: Kong.Token): Promise<void> {
    if (!this.connection || !this.userPublicKey || token.chain !== 'Solana') return;

    try {
      const { getAssociatedTokenAddress } = await import('@solana/spl-token');
      const tokenMint = new PublicKey(token.address);
      const associatedTokenAccount = await getAssociatedTokenAddress(
        tokenMint,
        this.userPublicKey
      );

      // Check if already subscribed
      if (this.subscriptions.has(associatedTokenAccount.toBase58())) {
        return;
      }

      const subscriptionId = this.connection.onAccountChange(
        associatedTokenAccount,
        (accountInfo: AccountInfo<Buffer>, context: Context) => {
          this.handleTokenBalanceUpdate(token, accountInfo, context);
        },
        COMMITMENT_CONFIG.balance
      );

      this.subscriptions.set(associatedTokenAccount.toBase58(), {
        id: subscriptionId,
        address: associatedTokenAccount.toBase58(),
        type: 'token',
        tokenSymbol: token.symbol,
      });

      console.log(`[SolanaWebSocket] Subscribed to new token: ${token.symbol}`);
    } catch (error) {
      console.error(`[SolanaWebSocket] Failed to subscribe to ${token.symbol}:`, error);
    }
  }

  /**
   * Unsubscribe from a token (called when a token is disabled)
   */
  async unsubscribeFromToken(token: Kong.Token): Promise<void> {
    if (!this.connection || !this.userPublicKey || token.chain !== 'Solana') return;

    try {
      const { getAssociatedTokenAddress } = await import('@solana/spl-token');
      const tokenMint = new PublicKey(token.address);
      const associatedTokenAccount = await getAssociatedTokenAddress(
        tokenMint,
        this.userPublicKey
      );

      const subscription = this.subscriptions.get(associatedTokenAccount.toBase58());
      if (subscription) {
        await this.connection.removeAccountChangeListener(subscription.id);
        this.subscriptions.delete(associatedTokenAccount.toBase58());
        console.log(`[SolanaWebSocket] Unsubscribed from token: ${token.symbol}`);
      }
    } catch (error) {
      console.error(`[SolanaWebSocket] Failed to unsubscribe from ${token.symbol}:`, error);
    }
  }

  /**
   * Start heartbeat to keep connection alive
   */
  private startHeartbeat(): void {
    this.stopHeartbeat();
    
    this.heartbeatTimer = setInterval(() => {
      if (this.connection && this.isConnected) {
        // Send a lightweight request to keep the connection alive
        this.connection.getSlot()
          .catch(error => {
            console.error('[SolanaWebSocket] Heartbeat failed:', error);
            this.handleDisconnect();
          });
      }
    }, WS_CONFIG.heartbeatInterval);
  }

  /**
   * Stop heartbeat
   */
  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  /**
   * Handle WebSocket disconnect
   */
  private handleDisconnect(): void {
    this.isConnected = false;
    this.stopHeartbeat();
    this.scheduleReconnect();
  }

  /**
   * Schedule reconnection with exponential backoff
   */
  private scheduleReconnect(): void {
    if (this.reconnectTimer || this.reconnectAttempts >= WS_CONFIG.maxReconnectAttempts) {
      return;
    }

    const delay = Math.min(
      WS_CONFIG.reconnectDelay * Math.pow(WS_CONFIG.reconnectBackoffMultiplier, this.reconnectAttempts),
      WS_CONFIG.maxReconnectDelay
    );

    console.log(`[SolanaWebSocket] Scheduling reconnect in ${delay}ms (attempt ${this.reconnectAttempts + 1})`);

    this.reconnectTimer = setTimeout(async () => {
      this.reconnectTimer = null;
      this.reconnectAttempts++;
      
      if (this.userPublicKey) {
        await this.initialize(this.userPublicKey.toBase58());
      }
    }, delay);
  }

  /**
   * Clean up all subscriptions and timers
   */
  async cleanup(): Promise<void> {
    console.log('[SolanaWebSocket] Cleaning up subscriptions');

    // Remove all subscriptions
    if (this.connection) {
      for (const [address, subscription] of this.subscriptions) {
        try {
          await this.connection.removeAccountChangeListener(subscription.id);
        } catch (error) {
          console.error(`[SolanaWebSocket] Error removing subscription for ${address}:`, error);
        }
      }
    }

    this.subscriptions.clear();
    
    // Clear timers
    this.stopHeartbeat();
    
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    this.isConnected = false;
    this.reconnectAttempts = 0;
  }

  /**
   * Get connection status
   */
  getStatus(): { isConnected: boolean; subscriptionCount: number } {
    return {
      isConnected: this.isConnected,
      subscriptionCount: this.subscriptions.size,
    };
  }
}

// Export singleton instance
export const solanaWebSocketService = new SolanaWebSocketService();