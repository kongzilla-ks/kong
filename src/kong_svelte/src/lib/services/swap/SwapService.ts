// src/lib/services/swap/SwapService.ts
// Consolidated swap service merging SwapService, SwapLogicService, and SwapMonitor

import { toastStore } from "$lib/stores/toastStore";
import { Principal } from "@dfinity/principal";
import BigNumber from "bignumber.js";
import { IcrcService } from "$lib/services/icrc/IcrcService";
import { swapStatusStore } from "$lib/stores/swapStore";
import { auth, swapActor } from "$lib/stores/auth";
import { requireWalletConnection } from "$lib/stores/auth";
import { fetchTokensByCanisterId } from "$lib/api/tokens";
import { get } from "svelte/store";
import { loadBalances } from "$lib/stores/tokenStore";
import { userTokens } from "$lib/stores/userTokens";
import { trackEvent, AnalyticsEvent } from "$lib/utils/analytics";
import { swapState } from "$lib/stores/swapStateStore";
import { CrossChainSwapService } from "./CrossChainSwapService";
import { solanaTransferModalStore } from "$lib/stores/solanaTransferModal";
import type { SwapAmountsResult, RequestsResult } from "../../../../../declarations/kong_backend/kong_backend.did";
import { LocalActorService } from '../actors/LocalActorService';

interface SwapExecuteParams {
  swapId: string;
  payToken: Kong.Token;
  payAmount: string;
  receiveToken: Kong.Token;
  receiveAmount: string;
  userMaxSlippage: number;
  backendPrincipal: Principal;
  lpFees: any[];
}

// These types match the backend response structure
interface SwapStatus {
  status: string;
  pay_amount: bigint;
  pay_symbol: string;
  receive_amount: bigint;
  receive_symbol: string;
}

// Base BigNumber configuration for internal calculations
// Set this high enough to handle intermediate calculations without loss of precision
BigNumber.config({
  DECIMAL_PLACES: 36, // High enough for internal calculations
  ROUNDING_MODE: BigNumber.ROUND_DOWN,
  EXPONENTIAL_AT: [-50, 50],
});

const BLOCKED_TOKEN_IDS = [];

export class SwapService {
  // Transaction monitoring state
  private static FAST_POLLING_INTERVAL = 100; // 100ms polling interval
  private static MAX_ATTEMPTS = 200; // 30 seconds total monitoring time
  private static pollingInterval: NodeJS.Timeout | null = null;
  private static startTime: number;

  // === Utility Methods ===
  private static isValidNumber(value: string | number): boolean {
    if (typeof value === "number") {
      return !isNaN(value) && isFinite(value);
    }
    if (typeof value === "string") {
      const num = Number(value);
      return !isNaN(num) && isFinite(num);
    }
    return false;
  }
// we need to retry
  public static toBigInt(
    value: string | number | BigNumber,
    decimals?: number,
  ): bigint {
    try {
      // If decimals provided, handle scaling
      if (decimals !== undefined) {
        // Use higher precision for intermediate calculations
        const tempBigNumber = new BigNumber(value);
        
        if (tempBigNumber.isNaN() || !tempBigNumber.isFinite()) {
          console.warn("Invalid numeric value:", value);
          return BigInt(0);
        }

        // For better precision, use string manipulation for simple decimal shifts
        // This avoids floating point errors
        const valueStr = tempBigNumber.toFixed();
        const parts = valueStr.split('.');
        const integerPart = parts[0] || '0';
        const decimalPart = parts[1] || '';
        
        // Pad or trim decimal part to match decimals
        let scaledDecimal = decimalPart.padEnd(decimals, '0').slice(0, decimals);
        
        // Remove leading zeros from integer part (but keep at least one zero)
        let cleanInteger = integerPart.replace(/^0+/, '') || '0';
        
        // Combine integer and decimal parts
        const result = cleanInteger + scaledDecimal;
        
        // Remove any leading zeros from final result (but keep at least one digit)
        const finalResult = result.replace(/^0+/, '') || '0';
        
        // Debug logging for USDC conversion issues
        if (decimals === 6 && (valueStr.includes('0.258109') || finalResult === '258108' || finalResult === '258109')) {
          console.log('[SwapService] toBigInt precision check:', {
            input: value.toString(),
            valueStr,
            integerPart,
            decimalPart,
            scaledDecimal,
            result,
            finalResult,
            decimals
          });
        }
        
        return BigInt(finalResult);
      }

      // Original logic for when no decimals provided
      if (value instanceof BigNumber) {
        return BigInt(value.integerValue(BigNumber.ROUND_DOWN).toString());
      }

      if (!this.isValidNumber(value)) {
        return BigInt(0);
      }

      const bn = new BigNumber(value);
      return BigInt(bn.integerValue(BigNumber.ROUND_DOWN).toString());
    } catch (error) {
      console.error("Error converting to BigInt:", error);
      return BigInt(0);
    }
  }

  public static fromBigInt(amount: bigint, decimals: number): string {
    try {
      const result = new BigNumber(amount.toString())
        .div(new BigNumber(10).pow(decimals))
        .toString();
      return isNaN(Number(result)) ? "0" : result;
    } catch {
      return "0";
    }
  }

  /**
   * Calculates the maximum transferable amount of a token, considering fees.
   *
   * @param tokenInfo - Information about the token, including fees and canister ID.
   * @param formattedBalance - The user's available balance of the token as a string.
   * @param decimals - Number of decimal places the token supports.
   * @param isIcrc1 - Boolean indicating if the token follows the ICRC1 standard.
   * @returns A BigNumber representing the maximum transferable amount.
   */
  public static calculateMaxAmount(
    tokenInfo: Kong.Token,
    formattedBalance: string,
    decimals: number = 8,
    isIcrc1: boolean = false,
  ): BigNumber {
    const SCALE_FACTOR = new BigNumber(10).pow(decimals);
    const balance = new BigNumber(formattedBalance);

    // Calculate base fee. If fee is undefined, default to 0.
    const baseFee = tokenInfo.fee_fixed
      ? new BigNumber(tokenInfo.fee_fixed.toString()).dividedBy(SCALE_FACTOR)
      : new BigNumber(0);

    // Calculate gas fee based on token standard
    const gasFee = isIcrc1 ? baseFee : baseFee.multipliedBy(2);

    // Ensure that the max amount is not negative
    const maxAmount = balance.minus(gasFee);
    return BigNumber.maximum(maxAmount, new BigNumber(0));
  }

  // === Token ID Formatting ===
  /**
   * Format token ID for backend calls
   * Backend accepts: "SOL", "USDC", "ksUSDT", or with optional chain prefix
   */
  public static formatTokenId(token: Kong.Token): string {
    // For Solana tokens, just use the symbol
    if (token.chain === 'Solana') {
      return token.symbol;
    }
    
    // For IC tokens, use the symbol or IC.address format
    // The backend's get_by_token function handles both
    return token.symbol;
  }

  // === Quote Methods ===
  /**
   * Gets swap quote from backend
   */
  public static async swap_amounts(
    payToken: Kong.Token,
    payAmount: bigint,
    receiveToken: Kong.Token,
  ): Promise<SwapAmountsResult> {
    try {
      if (!payToken?.address || !receiveToken?.address) {
        throw new Error("Invalid tokens provided for swap quote");
      }
      
      const payTokenId = this.formatTokenId(payToken);
      const receiveTokenId = this.formatTokenId(receiveToken);
      
      // Use LocalActorService for local development, PNP for production
      let actor;
      if (process.env.DFX_NETWORK === 'local') {
        actor = await LocalActorService.getKongBackendActor();
      } else {
        actor = swapActor({anon: true, requiresSigning: false});
      }
      
      return await actor.swap_amounts(
        payTokenId,
        payAmount,
        receiveTokenId,
      );
    } catch (error) {
      console.error("Error getting swap amounts:", error);
      throw error;
    }
  }

  /**
   * Gets quote details including price, fees, etc.
   */
  public static async getQuoteDetails(params: {
    payToken: Kong.Token;
    payAmount: bigint;
    receiveToken: Kong.Token;
  }): Promise<{
    receiveAmount: string;
    price: string;
    usdValue: string;
    lpFee: String;
    gasFee: String;
    tokenFee?: String;
    slippage: number;
  }> {
    const quote = await SwapService.swap_amounts(
      params.payToken,
      params.payAmount,
      params.receiveToken,
    );

    if (!("Ok" in quote)) {
      throw new Error(quote.Err);
    }

    const tokens = await fetchTokensByCanisterId([params.payToken.address, params.receiveToken.address]);
    const receiveToken = tokens.find(
      (t) => t.address === params.receiveToken.address,
    );
    const payToken = tokens.find((t) => t.address === params.payToken.address);
    if (!receiveToken) throw new Error("Receive token not found");

    const receiveAmount = SwapService.fromBigInt(
      quote.Ok.receive_amount,
      receiveToken.decimals,
    );

    let lpFee = "0";
    let gasFee = "0";
    let tokenFee = "0";

    if (quote.Ok.txs.length > 0) {
      const tx = quote.Ok.txs[0];
      lpFee = SwapService.fromBigInt(tx.lp_fee, receiveToken.decimals);
      gasFee = SwapService.fromBigInt(tx.gas_fee, receiveToken.decimals);
      tokenFee = payToken.fee_fixed.toString();
    }

    return {
      receiveAmount,
      price: quote.Ok.price.toString(),
      usdValue: new BigNumber(receiveAmount).times(quote.Ok.price).toFormat(2),
      lpFee,
      gasFee,
      tokenFee,
      slippage: quote.Ok.slippage,
    };
  }

  /**
   * Fetches the swap quote based on the provided amount and tokens.
   */
  public static async getSwapQuote(
    payToken: Kong.Token,
    receiveToken: Kong.Token,
    payAmount: string,
  ): Promise<{ 
    receiveAmount: string; 
    slippage: number;
    gasFees: Array<{ amount: string; token: string }>;
    lpFees: Array<{ amount: string; token: string }>;
    routingPath: Array<{
      paySymbol: string;
      receiveSymbol: string;
      poolSymbol: string;
      payAmount: string;
      receiveAmount: string;
      price: number;
    }>;
  }> {
    try {
      // Add check for blocked tokens at the start
      if (BLOCKED_TOKEN_IDS.includes(payToken.address) || 
          BLOCKED_TOKEN_IDS.includes(receiveToken.address)) {
        throw new Error("Token temporarily unavailable - BIL is in read-only mode");
      }

      // Validate input amount
      if (!payAmount || isNaN(Number(payAmount))) {
        console.warn("Invalid pay amount:", payAmount);
        return {
          receiveAmount: "0",
          slippage: 0,
          gasFees: [],
          lpFees: [],
          routingPath: [],
        };
      }

      // Convert amount to BigInt with proper decimal handling
      const payAmountBN = new BigNumber(payAmount);
      const payAmountInTokens = this.toBigInt(payAmountBN, payToken.decimals);

      const quote = await this.swap_amounts(
        payToken,
        payAmountInTokens,
        receiveToken,
      );

      if ("Ok" in quote) {
        // Get decimals synchronously from the token object
        const receiveDecimals = receiveToken.decimals;
        const receivedAmount = this.fromBigInt(
          quote.Ok.receive_amount,
          receiveDecimals,
        );

        // Extract fees from the quote
        const gasFees: Array<{ amount: string; token: string }> = [];
        const lpFees: Array<{ amount: string; token: string }> = [];
        const routingPath: Array<{
          paySymbol: string;
          receiveSymbol: string;
          poolSymbol: string;
          payAmount: string;
          receiveAmount: string;
          price: number;
        }> = [];

        if (quote.Ok.txs && quote.Ok.txs.length > 0) {
          for (const tx of quote.Ok.txs) {
            // Extract canister ID from receive_address (remove "IC." prefix if present)
            const feeTokenId = tx.receive_address.startsWith("IC.") 
              ? tx.receive_address.substring(3) 
              : tx.receive_address;
            
            // Determine token decimals for the fee
            // For multi-hop swaps, we need to look up each intermediate token's decimals
            let feeTokenDecimals = 8; // Default to 8 decimals
            let payTokenDecimals = 8; // Default to 8 decimals for pay token in this hop
            
            // Special case for ICP
            if (tx.receive_symbol === "ICP") {
              feeTokenDecimals = 8;
            } else if (feeTokenId === receiveToken.address) {
              // If it's the final receive token, use its decimals
              feeTokenDecimals = receiveDecimals;
            } else if (feeTokenId === payToken.address) {
              // If it's the pay token, use its decimals
              feeTokenDecimals = payToken.decimals;
            } else {
              // For intermediate tokens in multi-hop swaps, we need to look them up
              // For now, we'll use a default of 8 decimals for unknown tokens
              // This could be improved by fetching token details from the store
              feeTokenDecimals = 8;
            }
            
            // Similar logic for pay token decimals
            const payTokenId = tx.pay_address.startsWith("IC.") 
              ? tx.pay_address.substring(3) 
              : tx.pay_address;
            if (tx.pay_symbol === "ICP") {
              payTokenDecimals = 8;
            } else if (payTokenId === payToken.address) {
              payTokenDecimals = payToken.decimals;
            } else if (payTokenId === receiveToken.address) {
              payTokenDecimals = receiveToken.decimals;
            }
            
            // Add to routing path
            routingPath.push({
              paySymbol: tx.pay_symbol,
              receiveSymbol: tx.receive_symbol,
              poolSymbol: tx.pool_symbol,
              payAmount: this.fromBigInt(tx.pay_amount, payTokenDecimals),
              receiveAmount: this.fromBigInt(tx.receive_amount, feeTokenDecimals),
              price: tx.price,
            });
            
            // Add gas fee
            if (tx.gas_fee) {
              gasFees.push({
                amount: this.fromBigInt(tx.gas_fee, feeTokenDecimals),
                token: feeTokenId, // Use canister ID instead of symbol
              });
            }
            
            // Add LP fee
            if (tx.lp_fee) {
              lpFees.push({
                amount: this.fromBigInt(tx.lp_fee, feeTokenDecimals),
                token: feeTokenId, // Use canister ID instead of symbol
              });
            }
          }
        }

        return {
          receiveAmount: receivedAmount,
          slippage: quote.Ok.slippage,
          gasFees,
          lpFees,
          routingPath,
        };
      } else if ("Err" in quote) {
        throw new Error(quote.Err);
      }

      throw new Error("Invalid quote response");
    } catch (err) {
      console.error("Error fetching swap quote:", err);
      throw err;
    }
  }

  // === Token Selection Logic (from SwapLogicService) ===
  static handleSelectToken(type: "pay" | "receive", token: Kong.Token) {
    const state = get(swapState);
    
    if (
      (type === "pay" && token?.address === state.receiveToken?.address) ||
      (type === "receive" && token?.address === state.payToken?.address)
    ) {
      toastStore.error("Cannot select the same token for both sides");
      return;
    }

    const updates: Partial<typeof state> = {
      manuallySelectedTokens: {
        ...state.manuallySelectedTokens,
        [type]: true
      }
    };

    if (type === "pay") {
      updates.payToken = token;
      updates.showPayTokenSelector = false;
    } else {
      updates.receiveToken = token;
      updates.showReceiveTokenSelector = false;
    }

    // Debug log to check token chain
    console.log(`[SwapService] Selected ${type} token:`, {
      symbol: token.symbol,
      chain: token.chain,
      address: token.address,
      token_type: token.token_type
    });

    swapState.update(state => ({ ...state, ...updates }));
  }

  static async handleSwapSuccess(event: CustomEvent) {
    const tokens = get(userTokens).tokens;
    if (!tokens?.length) {
      console.warn('TokenStore not initialized or empty');
      return;
    }

    const payToken = tokens.find((t: any) => t.symbol === event.detail.payToken);
    const receiveToken = tokens.find((t: any) => t.symbol === event.detail.receiveToken);

    if (!payToken || !receiveToken) {
      console.error('Could not find pay or receive token', {
        paySymbol: event.detail.payToken,
        receiveSymbol: event.detail.receiveToken,
        availableTokens: tokens.map(t => t.symbol)
      });
      return;
    }

    // Reset the swap state
    swapState.update((state) => ({
      ...state,
      payAmount: "",
      receiveAmount: "",
      error: null,
      isProcessing: false,
    }));
  }

  // === Swap Execution Methods ===
  /**
   * Executes swap asynchronously with retry logic for TRANSACTION_NOT_READY errors
   */
  public static async swap_async(
    params: {
      pay_token: string;
      pay_amount: bigint;
      receive_token: string;
      receive_amount: [] | [bigint];
      max_slippage: [] | [number];
      receive_address: [] | [string];
      referred_by: [] | [string];
      pay_tx_id: [] | [{ BlockIndex: bigint }] | [{ TransactionId: string }];
      pay_signature: [] | [string];
      timestamp: [] | [bigint];
    },
    onRetryProgress?: (attempt: number, maxAttempts: number) => void
  ): Promise<BE.SwapAsyncResponse> {
    const MAX_RETRY_ATTEMPTS = 10;
    const RETRY_DELAY_MS = 1000; // 1 second
    
    // Helper function to check for TRANSACTION_NOT_READY in various formats
    const isTransactionNotReadyError = (error: any): boolean => {
      if (!error) return false;
      
      // Check error message for various formats
      const errorMessage = error?.message || error?.toString?.() || '';
      const errorString = typeof error === 'string' ? error : JSON.stringify(error);
      
      return errorMessage.includes('TRANSACTION_NOT_READY') ||
             errorString.includes('TRANSACTION_NOT_READY') ||
             errorString.toLowerCase().includes('transaction_not_ready') ||
             errorString.toLowerCase().includes('transaction not ready') ||
             (errorMessage.includes('ic0.trap') && errorMessage.includes('TRANSACTION_NOT_READY'));
    };
    
    for (let attempt = 1; attempt <= MAX_RETRY_ATTEMPTS; attempt++) {
      try {
        // For authenticated calls, we need to use PNP even in local dev
        // because LocalActorService doesn't handle authentication
        const actor = swapActor({anon: false, requiresSigning: auth.pnp.adapter.id === "plug"});
        const result: any = await actor.swap_async(params);
        
        // Check if we got an error response that indicates transaction not ready
        if (result.Err && isTransactionNotReadyError(result.Err)) {
          
          console.log(`[SwapService] Attempt ${attempt}/${MAX_RETRY_ATTEMPTS}: Transaction not ready, retrying in ${RETRY_DELAY_MS}ms...`);
          
          // Notify caller about retry progress
          if (onRetryProgress) {
            onRetryProgress(attempt, MAX_RETRY_ATTEMPTS);
          }
          
          // If this is not the last attempt, wait and retry
          if (attempt < MAX_RETRY_ATTEMPTS) {
            await new Promise(resolve => setTimeout(resolve, RETRY_DELAY_MS));
            continue;
          } else {
            // Final attempt failed
            console.error(`[SwapService] All ${MAX_RETRY_ATTEMPTS} attempts failed - transaction still not ready`);
            throw new Error(`Transaction verification failed after ${MAX_RETRY_ATTEMPTS} attempts. The Solana transaction may need more time to be confirmed.`);
          }
        }
        
        // Success or other error (non-retryable)
        return result;
        
      } catch (error) {
        console.error(`[SwapService] Attempt ${attempt}/${MAX_RETRY_ATTEMPTS} failed:`, error);
        
        // Check if this is a TRANSACTION_NOT_READY error thrown as exception
        if (isTransactionNotReadyError(error)) {
          console.log(`[SwapService] Attempt ${attempt}/${MAX_RETRY_ATTEMPTS}: Transaction not ready (exception), retrying in ${RETRY_DELAY_MS}ms...`);
          
          // Notify caller about retry progress
          if (onRetryProgress) {
            onRetryProgress(attempt, MAX_RETRY_ATTEMPTS);
          }
          
          // If this is not the last attempt, wait and retry
          if (attempt < MAX_RETRY_ATTEMPTS) {
            await new Promise(resolve => setTimeout(resolve, RETRY_DELAY_MS));
            continue;
          } else {
            // Final attempt failed
            console.error(`[SwapService] All ${MAX_RETRY_ATTEMPTS} attempts failed - transaction still not ready`);
            throw new Error(`Transaction verification failed after ${MAX_RETRY_ATTEMPTS} attempts. The Solana transaction may need more time to be confirmed.`);
          }
        }
        
        // If it's a network/connection error and not the last attempt, retry
        if (attempt < MAX_RETRY_ATTEMPTS && 
            (error instanceof Error && 
             (error.message.includes('network') || 
              error.message.includes('timeout') || 
              error.message.includes('fetch')))) {
          
          console.log(`[SwapService] Network error on attempt ${attempt}, retrying in ${RETRY_DELAY_MS}ms...`);
          
          if (onRetryProgress) {
            onRetryProgress(attempt, MAX_RETRY_ATTEMPTS);
          }
          
          await new Promise(resolve => setTimeout(resolve, RETRY_DELAY_MS));
          continue;
        }
        
        // Non-retryable error or final attempt
        throw error;
      }
    }
    
    // This should never be reached, but just in case
    throw new Error("Maximum retry attempts exceeded");
  }

  /**
   * Gets request status
   */
  public static async requests(requestIds: bigint[]): Promise<RequestsResult> {
    try {
      // Use LocalActorService for local development, PNP for production
      let actor;
      if (process.env.DFX_NETWORK === 'local') {
        actor = await LocalActorService.getKongBackendActor();
      } else {
        actor = swapActor({anon: true, requiresSigning: false});
      }
      
      // Ensure we only pass a single-element array or empty array
      const result = await actor.requests([requestIds[0]]);
      return result;
    } catch (error) {
      console.error("Error getting request status:", error);
      throw error;
    }
  }

  /**
   * Executes complete swap flow
   */
  public static async executeSwap(
    params: SwapExecuteParams,
  ): Promise<bigint | false> {
    const swapId = params.swapId;
    try {
      requireWalletConnection();
      
      // Log all input parameters
      console.log("[SwapService] executeSwap called with params:", {
        swapId: params.swapId,
        payToken: {
          symbol: params.payToken.symbol,
          address: params.payToken.address,
          chain: params.payToken.chain,
          decimals: params.payToken.decimals,
          standards: params.payToken.standards,
          fee_fixed: params.payToken.fee_fixed
        },
        payAmount: params.payAmount,
        receiveToken: {
          symbol: params.receiveToken.symbol,
          address: params.receiveToken.address,
          chain: params.receiveToken.chain,
          decimals: params.receiveToken.decimals
        },
        receiveAmount: params.receiveAmount,
        userMaxSlippage: params.userMaxSlippage,
        backendPrincipal: params.backendPrincipal.toString()
      });
      
      // Add check for blocked tokens at the start
      if (BLOCKED_TOKEN_IDS.includes(params.payToken.address) || 
          BLOCKED_TOKEN_IDS.includes(params.receiveToken.address)) {
        toastStore.warning(
          "BIL token is currently in read-only mode. Trading will resume when the ledger is stable.",
          {
            title: "Token Temporarily Unavailable",
            duration: 8000
          }
        );
        swapStatusStore.updateSwap(swapId, {
          status: "Failed",
          isProcessing: false,
          error: "Token temporarily unavailable"
        });
        return false;
      }
      const payToken = params.payToken

      if (!payToken) {
        throw new Error(`Pay token ${params.payToken.symbol} not found`);
      }

      const payAmount = SwapService.toBigInt(
        params.payAmount,
        payToken.decimals,
      );
      
      console.log("[SwapService] Calculated payAmount (bigint):", payAmount.toString());

      const receiveToken = params.receiveToken

      if (!receiveToken) {
        throw new Error(`Receive token ${params.receiveToken.symbol} not found`);
      }

      // Check if this is a cross-chain swap (paying with Solana token)
      if (payToken.chain === 'Solana') {
        console.log("[SwapService] Detected Solana token, executing cross-chain swap");
        return await this.executeCrossChainSwap(params);
      }

      // Check if we need to get Solana address for receiving SOL tokens
      let solanaReceiveAddress: string | undefined;
      if (receiveToken.chain === 'Solana') {
        try {
          solanaReceiveAddress = await CrossChainSwapService.getSolanaWalletAddress();
          console.log("[SwapService] Got Solana receive address:", solanaReceiveAddress);
        } catch (error) {
          console.error("[SwapService] Failed to get Solana address:", error);
          throw new Error("Please connect a Solana wallet to receive SOL tokens");
        }
      }

      // Original IC token swap logic
      let txId: bigint | false = false;
      let approvalId: bigint | false = false;
      const toastId = toastStore.info(
        `Swapping ${params.payAmount} ${params.payToken.symbol} to ${params.receiveAmount} ${params.receiveToken.symbol}...`,
        { duration: 15000 }, // 15 seconds
      );

      console.log("[SwapService] Token standards:", payToken.standards);

      if (payToken.standards.includes("ICRC-2")) {
        console.log("[SwapService] Token supports ICRC-2, requesting allowance");
        const requiredAllowance = payAmount;
        console.log("[SwapService] Required allowance:", requiredAllowance.toString());
        console.log("[SwapService] Backend principal for approval:", params.backendPrincipal.toString());
        approvalId = await IcrcService.checkAndRequestIcrc2Allowances(
          payToken,
          requiredAllowance,
          params.backendPrincipal.toString(),
        );
        console.log("[SwapService] Approval result:", approvalId);
      } else if (payToken.standards.includes("ICRC-1")) {
        console.log("[SwapService] Token supports ICRC-1, doing direct transfer");
        const result = await IcrcService.transfer(
          payToken,
          params.backendPrincipal,
          payAmount,
          { fee: BigInt(payToken.fee_fixed) },
        );
        console.log("[SwapService] Transfer result:", result);

        if (result?.Ok) {
          txId = result.Ok;
        } else {
          txId = false;
        }
      } else {
        throw new Error(
          `Token ${payToken.symbol} does not support ICRC1 or ICRC2`,
        );
      }

      if (txId === false && approvalId === false) {
        console.error("[SwapService] Both txId and approvalId are false");
        swapStatusStore.updateSwap(swapId, {
          status: "Failed",
          isProcessing: false,
          error: "Transaction failed during transfer/approval",
        });
        toastStore.error("Transaction failed during transfer/approval");
        return false;
      }

      // IMPORTANT: The backend routing logic:
      // - If pay_tx_id is None (not provided at all), it uses swap_transfer_from (ICRC-2 flow)
      // - If pay_tx_id is Some (provided, even if empty), it uses swap_transfer (ICRC-1 flow)
      // So for ICRC-2 tokens, we must NOT provide pay_tx_id field at all
      // For ICRC-1 tokens, we MUST provide pay_tx_id with the block index
      
      // Build base params with explicit pay_tx_id field
      const baseSwapParams = {
        pay_token: this.formatTokenId(params.payToken),
        pay_amount: BigInt(payAmount),
        receive_token: this.formatTokenId(params.receiveToken),
        receive_amount: [] as [] | [bigint],
        max_slippage: [params.userMaxSlippage] as [] | [number],
        receive_address: solanaReceiveAddress ? [solanaReceiveAddress] as [] | [string] : [] as [] | [string],
        referred_by: [] as [] | [string],
        pay_tx_id: [] as [] | [{ BlockIndex: bigint }] | [{ TransactionId: string }],
        pay_signature: [] as [] | [string],
        timestamp: [] as [] | [bigint],
      };
      
      // Set pay_tx_id based on token type
      // For ICRC-1 tokens (when we did a transfer), set the block index
      // For ICRC-2 tokens (when we did an approval), leave it as empty array
      const swapParams = txId 
        ? { ...baseSwapParams, pay_tx_id: [{ BlockIndex: BigInt(txId) }] as [] | [{ BlockIndex: bigint }] }
        : baseSwapParams;
      
      console.log("[SwapService] Final swap_async params:", {
        pay_token: swapParams.pay_token,
        pay_amount: swapParams.pay_amount.toString(),
        receive_token: swapParams.receive_token,
        receive_amount: swapParams.receive_amount,
        max_slippage: swapParams.max_slippage,
        receive_address: swapParams.receive_address,
        referred_by: swapParams.referred_by,
        pay_tx_id: swapParams.pay_tx_id,
        pay_signature: swapParams.pay_signature,
        timestamp: swapParams.timestamp
      });

      const result = await SwapService.swap_async(swapParams, (attempt, maxAttempts) => {
        // Show retry progress to user for IC token swaps
        if (attempt > 1) {
          toastStore.info(
            `Verifying transaction... (attempt ${attempt}/${maxAttempts})`,
            { 
              duration: 2000,
              title: "Transaction Verification" 
            }
          );
        }
      });
      
      console.log("[SwapService] swap_async result:", result);

      if (result.Ok) {
        // Start monitoring the transaction
        this.monitorTransaction(result?.Ok, swapId, toastId);
      } else {
        console.error("Swap error:", result.Err);
        return false;
      }
      return result.Ok;
    } catch (error) {
      swapStatusStore.updateSwap(swapId, {
        status: "Failed",
        isProcessing: false,
        error: error instanceof Error ? error.message : "Swap failed",
      });
      console.error("Swap execution failed:", error);
      toastStore.error(error instanceof Error ? error.message : "Swap failed");
      return false;
    }
  }

  /**
   * Execute cross-chain swap (Solana -> IC tokens)
   */
  private static async executeCrossChainSwap(
    params: SwapExecuteParams
  ): Promise<bigint | false> {
    const swapId = params.swapId;
    try {
      // Check if wallet supports Solana
      const isSolanaCompatible = await CrossChainSwapService.isWalletSolanaCompatible();
      if (!isSolanaCompatible) {
        throw new Error("Connected wallet does not support Solana. Please connect a Solana wallet.");
      }

      // Get Kong's Solana address
      const kongSolanaAddress = await CrossChainSwapService.getKongSolanaAddress();
      const userSolanaAddress = await CrossChainSwapService.getSolanaWalletAddress();
      
      const payAmount = SwapService.toBigInt(
        params.payAmount,
        params.payToken.decimals,
      );

      // Show transfer modal and wait for user to complete
      return new Promise<bigint | false>((resolve) => {
        solanaTransferModalStore.show({
          payToken: params.payToken,
          payAmount: params.payAmount,
          receiveToken: params.receiveToken,
          receiveAmount: params.receiveAmount,
          maxSlippage: params.userMaxSlippage,
          onConfirm: async (modalData) => {
            try {
              const { transactionId, pay_signature: signature, timestamp } = modalData;
              
              console.log('[SwapService] Cross-chain swap modalData:', {
                transactionId,
                signature,
                timestamp,
                modalData
              });
              
              // Execute swap with signature
              // IMPORTANT: These values must match exactly what was signed in the canonical message
              const receiveAmountBigInt = SwapService.toBigInt(params.receiveAmount, params.receiveToken.decimals);
              const authStore = get(auth);
              const icPrincipal = authStore.account?.owner || '';
              const receiveAddress = (params.receiveToken.chain === 'ICP') ? icPrincipal : userSolanaAddress;
              
              const swapParams = {
                pay_token: this.formatTokenId(params.payToken),
                pay_amount: payAmount,
                receive_token: this.formatTokenId(params.receiveToken),
                receive_amount: [receiveAmountBigInt] as [] | [bigint], // Must match signed message
                max_slippage: [params.userMaxSlippage] as [] | [number],
                receive_address: [receiveAddress] as [] | [string], // Must match signed message
                referred_by: [] as [] | [string],
                pay_tx_id: [{ TransactionId: transactionId }] as [] | [{ TransactionId: string }],
                pay_signature: [signature] as [] | [string],
                timestamp: [timestamp] as [] | [bigint],
              };

              console.log('[SwapService] Final cross-chain swap_async params:', {
                pay_token: swapParams.pay_token,
                pay_amount: swapParams.pay_amount.toString(),
                receive_token: swapParams.receive_token,
                receive_amount: swapParams.receive_amount,
                max_slippage: swapParams.max_slippage,
                receive_address: swapParams.receive_address,
                referred_by: swapParams.referred_by,
                pay_tx_id: swapParams.pay_tx_id,
                pay_signature: swapParams.pay_signature,
                timestamp: swapParams.timestamp
              });

              const result = await SwapService.swap_async(swapParams, (attempt, maxAttempts) => {
                // Show retry progress to user
                if (attempt === 1) {
                  // Silent - no need for initiation toast
                } else {
                  toastStore.info(
                    `Verifying Solana transaction... (attempt ${attempt}/${maxAttempts})`,
                    { 
                      duration: 2000,
                      title: "Transaction Verification" 
                    }
                  );
                }
              });

              if (result.Ok) {
                // Silent - will show success message when complete
                // Start monitoring the transaction
                this.monitorTransaction(result.Ok, swapId, "");
                resolve(result.Ok);
              } else {
                console.error("Cross-chain swap error:", result.Err);
                throw new Error(result.Err || "Cross-chain swap failed");
              }
            } catch (error) {
              swapStatusStore.updateSwap(swapId, {
                status: "Failed",
                isProcessing: false,
                error: error instanceof Error ? error.message : "Cross-chain swap failed",
              });
              toastStore.error(error instanceof Error ? error.message : "Cross-chain swap failed");
              resolve(false);
            }
          },
        });
      });
    } catch (error) {
      swapStatusStore.updateSwap(swapId, {
        status: "Failed",
        isProcessing: false,
        error: error instanceof Error ? error.message : "Cross-chain swap failed",
      });
      console.error("Cross-chain swap execution failed:", error);
      toastStore.error(error instanceof Error ? error.message : "Cross-chain swap failed");
      return false;
    }
  }

  // === Transaction Monitoring (from SwapMonitor) ===
  private static async monitorTransaction(requestId: bigint, swapId: string, toastId: string) {
    this.stopPolling();
    this.startTime = Date.now();
    let attempts = 0;
    let shownStatuses = new Set<string>();

    const poll = async () => {
      if (attempts >= this.MAX_ATTEMPTS) {
        this.stopPolling();
        swapStatusStore.updateSwap(swapId, {
          status: "Timeout",
          isProcessing: false,
          error: "Swap timed out",
        });
        toastStore.error("Swap timed out");
        return;
      }

      try {
        const status = await SwapService.requests([requestId]);

        if ("Ok" in status) {
          const res = status.Ok[0];

          // Only show toast for new status updates
          if (res.statuses && res.statuses.length > 0) {            
            for (const status of res.statuses) {
              if (!shownStatuses.has(status)) {
                shownStatuses.add(status);
                
                if (status.toLowerCase() === "swap success") {
                  toastStore.dismiss(toastId);
                  // Silent - will show detailed message below
                  swapState.setShowSuccessModal(true);
                } else if (status === "Success") {
                  // Silent - balances update automatically
                } else if (status.toLowerCase().includes("failed")) {
                  toastStore.dismiss(toastId);
                  toastStore.error(`${status}`);
                }
              }
            }
          }

          if (res.statuses.find((s) => s.includes("Failed"))) {
            this.stopPolling();
            swapStatusStore.updateSwap(swapId, {
              status: "Error",
              isProcessing: false,
              error: res.statuses.find((s) => s.includes("Failed")),
            });
            toastStore.dismiss(toastId);
            toastStore.error(res.statuses.find((s) => s.includes("Failed")));
            return;
          }

          if ("Swap" in res.reply) {
            const swapStatus = res.reply.Swap as SwapStatus;
            swapStatusStore.updateSwap(swapId, {
              status: swapStatus.status,
              isProcessing: true,
              error: null,
            });

            if (swapStatus.status === "Success") {
              this.stopPolling();
              const token0 = get(userTokens).tokens.find(
                (t) => t.symbol === swapStatus.pay_symbol,
              );
              const token1 = get(userTokens).tokens.find(
                (t) => t.symbol === swapStatus.receive_symbol,
              );

              const formattedPayAmount = SwapService.fromBigInt(
                swapStatus.pay_amount,
                token0?.decimals || 0,
              );
              const formattedReceiveAmount = SwapService.fromBigInt(
                swapStatus.receive_amount,
                token1?.decimals || 0,
              );

              // Show detailed toast with actual amounts
              toastStore.success(`Swap of ${formattedPayAmount} ${swapStatus.pay_symbol} for ${formattedReceiveAmount} ${swapStatus.receive_symbol} completed successfully`);

              // Track successful swap event
              trackEvent(AnalyticsEvent.SwapCompleted, {
                pay_token: token0?.symbol,
                pay_amount: formattedPayAmount,
                receive_token: token1?.symbol,
                receive_amount: formattedReceiveAmount,
                duration_ms: Date.now() - this.startTime
              });

              swapStatusStore.updateSwap(swapId, {
                status: "Success",
                isProcessing: false,
                shouldRefreshQuote: true,
                lastQuote: null,
                details: {
                  payAmount: formattedPayAmount,
                  payToken: token0,
                  receiveAmount: formattedReceiveAmount,
                  receiveToken: token1,
                },
              });

              // Load updated balances
              const tokens = get(userTokens).tokens;
              const payToken = tokens.find(
                (t) => t.symbol === swapStatus.pay_symbol,
              );
              const receiveToken = tokens.find(
                (t) => t.symbol === swapStatus.receive_symbol,
              );
              const walletId = auth?.pnp?.account?.owner;

              if (!payToken || !receiveToken || !walletId) {
                console.error("Missing token or wallet info for balance update");
                return;
              }

              try {
                await loadBalances(
                  [payToken, receiveToken],
                  walletId,
                  true
                );
              } catch (error) {
                console.error("Error updating balances:", error);
              }

              return;
            } else if (swapStatus.status === "Failed") {
              this.stopPolling();
              swapStatusStore.updateSwap(swapId, {
                status: "Failed",
                isProcessing: false,
                error: "Swap failed",
              });
              toastStore.error("Swap failed");
              
              // Track failed swap event
              trackEvent(AnalyticsEvent.SwapFailed, {
                pay_token: swapStatus.pay_symbol,
                receive_token: swapStatus.receive_symbol,
                error: "Swap failed",
                duration_ms: Date.now() - this.startTime
              });
              
              return;
            }
          }
        }

        attempts++;
        this.pollingInterval = setTimeout(poll, this.FAST_POLLING_INTERVAL);
      } catch (error) {
        console.error("Error monitoring swap:", error);
        this.stopPolling();
        swapStatusStore.updateSwap(swapId, {
          status: "Error",
          isProcessing: false,
          error: "Failed to monitor swap status",
        });
        toastStore.error("Failed to monitor swap status");
        return;
      }
    };

    // Start polling
    poll();
  }

  private static stopPolling() {
    if (this.pollingInterval) {
      clearTimeout(this.pollingInterval);
      this.pollingInterval = null;
    }
  }

  // Clean up method to be called when component unmounts
  public static cleanup() {
    this.stopPolling();
  }
}