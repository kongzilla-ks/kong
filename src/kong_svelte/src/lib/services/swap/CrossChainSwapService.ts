// CrossChainSwapService.ts
// Service to handle cross-chain (Solana) swap operations

import { BackendTokenService } from "../tokens/BackendTokenService";
import { auth } from "$lib/stores/auth";
import type { PNP } from "@windoge98/plug-n-play";

// Declare window.solana for TypeScript
declare global {
  interface Window {
    solana?: {
      isPhantom?: boolean;
      isConnected?: boolean;
      publicKey?: any; // Solana PublicKey object
      connect(): Promise<{ publicKey: any }>;
      signMessage(message: Uint8Array, display?: string): Promise<{ signature: Uint8Array | string }>;
      signAndSendTransaction(transaction: any): Promise<{ signature: string }>;
    };
  }
}

export interface CanonicalSwapMessage {
  pay_token: string;
  pay_amount: string;
  pay_address: string;
  receive_token: string;
  receive_amount: string;
  receive_address: string;
  max_slippage: number;
  timestamp: number;
  referred_by: null;
}

/**
 * CrossChainSwapService - Handles cross-chain swap operations, primarily with Solana
 * 
 * This service uses a hybrid approach for Solana wallet integration:
 * 
 * 1. **Native Phantom Wallet (Preferred)**: Direct integration with window.solana API
 *    - Provides full functionality for message signing and transactions
 *    - Bypasses limitations of SIWS (Sign-In With Solana) implementations
 *    - Most reliable for cross-chain swap operations
 * 
 * 2. **PNP SIWS Fallback**: Uses @windoge98/plug-n-play for authentication
 *    - Limited to authentication-focused signing
 *    - May not support arbitrary message signing required for swaps
 *    - Used when native Phantom is not available
 * 
 * The service automatically detects which approach to use and provides clear
 * error messages when cross-chain functionality is not available.
 */
export class CrossChainSwapService {
  /**
   * Check if wallet is connected and reconnect if needed
   */
  static async ensureWalletConnected(): Promise<boolean> {
    // Check if wallet is connected
    if (!auth.pnp || !auth.pnp.provider || !auth.pnp.isAuthenticated()) {
      // Try to get the last connected wallet ID from localStorage
      const lastWalletId = localStorage.getItem('auth:selectedWallet');
      
      if (lastWalletId && lastWalletId.includes('Siws')) {
        try {
          // Try to reconnect the wallet
          await auth.connect(lastWalletId);
          return true;
        } catch (e) {
          console.error("Failed to reconnect wallet:", e);
          return false;
        }
      }
      return false;
    }
    return true;
  }

  /**
   * Get Kong's Solana address for receiving transfers
   */
  static async getKongSolanaAddress(): Promise<string> {
    return BackendTokenService.getKongSolanaAddress();
  }

  /**
   * Create canonical message for signing
   * Matches the format expected by the backend payment verifier
   */
  static createCanonicalMessage(params: {
    payToken: string;
    payAmount: bigint;
    payAddress: string;
    receiveToken: string;
    receiveAmount: bigint;
    receiveAddress: string;
    maxSlippage: number;
    timestamp: bigint;
  }): string {
    // Backend expects exact format from CanonicalSwapMessage struct:
    // - pay_amount and receive_amount as Nat (which serializes to string in JSON)
    // - timestamp as u64 (which serializes to number in JSON)
    // - referred_by as Option<String> (which serializes to null in JSON)
    const message = {
      pay_token: params.payToken,
      pay_amount: params.payAmount.toString(), // Nat serializes to string
      pay_address: params.payAddress,
      receive_token: params.receiveToken,
      receive_amount: params.receiveAmount.toString(), // Nat serializes to string
      receive_address: params.receiveAddress,
      max_slippage: parseFloat(params.maxSlippage.toFixed(1)), // Force float format to avoid 2 vs 2.0 issues
      timestamp: Number(params.timestamp), // u64 serializes to number
      referred_by: null // Option<String> serializes to null
    };
    
    // Use JSON.stringify to match backend's serde_json::to_string
    const jsonMessage = JSON.stringify(message);
    console.log('[CrossChainSwapService] Created canonical message:', jsonMessage);
    console.log('[CrossChainSwapService] Message length:', jsonMessage.length);
    console.log('[CrossChainSwapService] Pay address in message:', params.payAddress);
    console.log('[CrossChainSwapService] Receive address in message:', params.receiveAddress);
    console.log('[CrossChainSwapService] Original slippage:', params.maxSlippage, 'Formatted slippage:', message.max_slippage);
    
    return jsonMessage;
  }

  /**
   * Get connected Solana wallet address (prefers native Phantom)
   */
  static async getSolanaWalletAddress(): Promise<string> {
    // Method 1: Try native Phantom first (most reliable)
    if (window.solana && window.solana.isPhantom) {
      try {
        // Ensure connection
        if (!window.solana.isConnected) {
          await window.solana.connect();
        }
        
        if (window.solana.isConnected && window.solana.publicKey) {
          const address = window.solana.publicKey.toString();
          console.log('[CrossChainSwapService] Got address from native Phantom:', address);
          return address;
        }
      } catch (e) {
        console.error("Error getting address from native Phantom:", e);
      }
    }
    
    // Method 2: Fallback to PNP provider
    if (!auth.pnp || !auth.pnp.provider) {
      throw new Error("No wallet connected");
    }

    const provider = auth.pnp.provider;
    
    // Check if this is a Solana wallet
    const walletId = provider.id || '';
    if (!walletId.includes('Siws') && walletId !== 'walletconnect') {
      throw new Error("Connected wallet does not support Solana");
    }

    // Try to get Solana address from provider
    if (provider.getSolanaAddress) {
      try {
        const address = await provider.getSolanaAddress();
        if (address) {
          console.log('[CrossChainSwapService] Got address from PNP provider:', address);
          return address;
        }
      } catch (e) {
        console.error("Error getting Solana address from provider:", e);
      }
    }

    // For wallets that expose publicKey directly
    if ((provider as any).publicKey) {
      const address = (provider as any).publicKey.toString();
      console.log('[CrossChainSwapService] Got address from provider publicKey:', address);
      return address;
    }

    // Try to get from account if it exists
    if (auth.pnp.account && 'solanaAddress' in auth.pnp.account) {
      const address = (auth.pnp.account as any).solanaAddress;
      console.log('[CrossChainSwapService] Got address from PNP account:', address);
      return address;
    }

    throw new Error("Could not get Solana address from wallet");
  }

  /**
   * Sign message with connected Solana wallet
   * Uses native Phantom wallet API when available, bypassing PNP limitations
   */
  static async signMessage(message: string): Promise<string> {
    console.log('[CrossChainSwapService] Attempting to sign message');
    console.log('[CrossChainSwapService] Message to sign:', message);
    console.log('[CrossChainSwapService] Message length:', message.length);
    
    // Method 1: Try native Phantom wallet first (best option for cross-chain swaps)
    if (window.solana && window.solana.isPhantom) {
      try {
        console.log('[CrossChainSwapService] Using native Phantom wallet');
        
        // Ensure Phantom is connected
        if (!window.solana.isConnected) {
          console.log('[CrossChainSwapService] Connecting to Phantom...');
          await window.solana.connect();
        }
        
        // Get the current wallet address for verification
        const walletAddress = window.solana.publicKey?.toString();
        console.log('[CrossChainSwapService] Phantom wallet address:', walletAddress);
        
        // Encode the message properly
        const encodedMessage = new TextEncoder().encode(message);
        console.log('[CrossChainSwapService] Encoded message length:', encodedMessage.length);
        console.log('[CrossChainSwapService] Encoded message bytes (first 50):', Array.from(encodedMessage.slice(0, 50)));
        
        // Sign with native Phantom API
        const signedMessage = await window.solana.signMessage(encodedMessage, "utf8");
        console.log('[CrossChainSwapService] Phantom signing successful');
        
        if (signedMessage.signature) {
          // Handle different signature formats
          if (typeof signedMessage.signature === 'string') {
            console.log('[CrossChainSwapService] Got string signature from Phantom:', signedMessage.signature);
            return signedMessage.signature;
          } else if (signedMessage.signature instanceof Uint8Array) {
            console.log('[CrossChainSwapService] Got Uint8Array signature, length:', signedMessage.signature.length);
            const base58Signature = await this.encodeToBase58(signedMessage.signature);
            console.log('[CrossChainSwapService] Converted to base58:', base58Signature);
            return base58Signature;
          } else {
            console.log('[CrossChainSwapService] Unknown signature format:', typeof signedMessage.signature);
          }
        }
        
        throw new Error("No signature returned from Phantom");
      } catch (e: any) {
        console.error("Native Phantom signing failed:", e);
        
        // If it's a user rejection, throw a user-friendly error
        if (e.message?.includes('User rejected') || e.code === 4001) {
          throw new Error("Transaction signing was cancelled by user");
        }
        
        // For other errors, try fallback methods
        console.log('[CrossChainSwapService] Trying fallback methods...');
      }
    }
    
    // Method 2: Try to get Solana address from PNP and ensure native Phantom connection
    if (auth.pnp && auth.pnp.provider) {
      const provider = auth.pnp.provider;
      console.log('[CrossChainSwapService] PNP Provider ID:', provider.id);
      
      // If using a Solana-based PNP provider, try to get the address and connect native Phantom
      if (provider.id && provider.id.includes('Siws')) {
        try {
          // Get the Solana address from PNP
          const solanaAddress = await this.getSolanaWalletAddress();
          console.log('[CrossChainSwapService] Got Solana address from PNP:', solanaAddress);
          
          // Now try to connect to native Phantom with the same address
          if (window.solana && window.solana.isPhantom) {
            try {
              const phantomConnection = await window.solana.connect();
              const phantomAddress = phantomConnection.publicKey.toString();
              
              // Verify addresses match (security check)
              if (phantomAddress === solanaAddress) {
                console.log('[CrossChainSwapService] Address verification passed, using native Phantom');
                
                const encodedMessage = new TextEncoder().encode(message);
                const signedMessage = await window.solana.signMessage(encodedMessage, "utf8");
                
                if (signedMessage.signature) {
                  if (typeof signedMessage.signature === 'string') {
                    return signedMessage.signature;
                                     } else if (signedMessage.signature instanceof Uint8Array) {
                     return await this.encodeToBase58(signedMessage.signature);
                  }
                }
              } else {
                console.warn('[CrossChainSwapService] Address mismatch - PNP:', solanaAddress, 'Phantom:', phantomAddress);
              }
            } catch (phantomError) {
              console.error("Failed to connect native Phantom:", phantomError);
            }
          }
        } catch (e) {
          console.error("Failed to get Solana address from PNP:", e);
        }
      }
    }
    
    // Method 3: Last resort - try PNP provider methods (likely to fail for SIWS)
    if (auth.pnp && auth.pnp.provider && auth.pnp.provider.signMessage) {
      try {
        console.log('[CrossChainSwapService] Trying PNP provider as last resort');
        const encodedMessage = new TextEncoder().encode(message);
        const signature = await auth.pnp.provider.signMessage(encodedMessage);
        
                 if (signature instanceof Uint8Array) {
           return await this.encodeToBase58(signature);
        } else if (typeof signature === 'string') {
          return signature;
        }
      } catch (e) {
        console.error("PNP provider signing failed:", e);
      }
    }
    
    // If all methods fail, provide helpful guidance
    const walletType = auth.pnp?.provider?.id || 'unknown';
    const hasNativePhantom = !!(window.solana && window.solana.isPhantom);
    
    if (!hasNativePhantom && walletType.includes('Siws')) {
      throw new Error(
        "Cross-chain swaps require native Phantom wallet access. Please install the Phantom browser extension and ensure it's unlocked, then refresh the page."
      );
    } else if (hasNativePhantom) {
      throw new Error(
        "Failed to sign message with Phantom wallet. Please ensure Phantom is unlocked and try again."
      );
    } else {
      throw new Error(
        `Unable to sign cross-chain swap message with ${walletType} wallet. Native Phantom wallet support is required for cross-chain operations.`
      );
    }
  }

  /**
   * Check if native Phantom wallet is available and connected
   */
  static async isNativePhantomAvailable(): Promise<boolean> {
    try {
      if (!window.solana || !window.solana.isPhantom) {
        return false;
      }
      
      // Check if already connected
      if (window.solana.isConnected) {
        return true;
      }
      
      // Try to connect (this will prompt user if needed)
      try {
        await window.solana.connect();
        return window.solana.isConnected || false;
      } catch (e) {
        // User might have rejected connection
        console.log('[CrossChainSwapService] Phantom connection rejected or failed:', e);
        return false;
      }
    } catch (e) {
      console.error('[CrossChainSwapService] Error checking native Phantom:', e);
      return false;
    }
  }

  /**
   * Check if current wallet supports Solana (enhanced to include native Phantom)
   */
  static async isWalletSolanaCompatible(): Promise<boolean> {
    // First check if native Phantom is available (best option)
    const hasNativePhantom = await CrossChainSwapService.isNativePhantomAvailable();
    if (hasNativePhantom) {
      console.log('[CrossChainSwapService] Native Phantom wallet detected and available');
      return true;
    }
    
    // Then check PNP wallet compatibility
    const isConnected = await CrossChainSwapService.ensureWalletConnected();
    if (!isConnected) return false;
    
    // Check if PNP is connected and provider exists
    if (!auth.pnp || !auth.pnp.provider) return false;
    
    // Get the current adapter from the provider
    const provider = auth.pnp.provider;
    
    // Check if it's a Solana-compatible wallet
    // Phantom, Solflare, Backpack all have 'Siws' in their ID
    const walletId = provider.id || '';
    const isPnpSolanaWallet = walletId.includes('Siws') || walletId === 'walletconnect';
    
    if (isPnpSolanaWallet) {
      console.log('[CrossChainSwapService] PNP Solana wallet detected:', walletId);
      return true;
    }
    
    return false;
  }

  /**
   * Wait for transaction confirmation
   * In production, this would poll the Solana RPC or wait for kong_rpc
   */
  static async waitForTransactionConfirmation(txId: string): Promise<void> {
    // Wait 15 seconds for kong_rpc to process the transaction
    // In production, you might want to poll the transaction status
    console.log(`Waiting for transaction ${txId} to be processed by kong_rpc...`);
    await new Promise(resolve => setTimeout(resolve, 15000));
  }

  /**
   * Format Solana amount for display
   */
  static formatSolanaAmount(amount: bigint, decimals: number): string {
    const divisor = BigInt(10 ** decimals);
    const whole = amount / divisor;
    const remainder = amount % divisor;
    
    if (remainder === 0n) {
      return whole.toString();
    }
    
    // Convert remainder to decimal string with proper padding
    const decimalStr = remainder.toString().padStart(decimals, '0');
    // Remove trailing zeros
    const trimmed = decimalStr.replace(/0+$/, '');
    
    return `${whole}.${trimmed}`;
  }

  /**
   * Utility function to encode Uint8Array to base58 string
   */
  private static async encodeToBase58(data: Uint8Array): Promise<string> {
    const bs58 = await import('bs58');
    return bs58.default.encode(data);
  }

  /**
   * Debug method to test wallet signing capabilities
   */
  static async debugWalletCapabilities(): Promise<void> {
    console.log('[CrossChainSwapService] === Wallet Debug Info ===');
    
    // Check native Phantom
    const hasNativePhantom = !!(window.solana && window.solana.isPhantom);
    console.log('[CrossChainSwapService] Native Phantom available:', hasNativePhantom);
    
    if (hasNativePhantom) {
      console.log('[CrossChainSwapService] Phantom isConnected:', window.solana?.isConnected);
      console.log('[CrossChainSwapService] Phantom publicKey:', window.solana?.publicKey?.toString());
    }
    
    // Check PNP provider
    console.log('[CrossChainSwapService] PNP available:', !!(auth.pnp && auth.pnp.provider));
    if (auth.pnp && auth.pnp.provider) {
      console.log('[CrossChainSwapService] PNP provider ID:', auth.pnp.provider.id);
      console.log('[CrossChainSwapService] PNP has signMessage:', !!auth.pnp.provider.signMessage);
    }
    
    // Test compatibility
    try {
      const isCompatible = await this.isWalletSolanaCompatible();
      console.log('[CrossChainSwapService] Wallet is Solana compatible:', isCompatible);
      
      if (isCompatible) {
        const address = await this.getSolanaWalletAddress();
        console.log('[CrossChainSwapService] Solana address:', address);
      }
    } catch (e) {
      console.error('[CrossChainSwapService] Error testing compatibility:', e);
    }
    
    console.log('[CrossChainSwapService] === End Debug Info ===');
  }
}