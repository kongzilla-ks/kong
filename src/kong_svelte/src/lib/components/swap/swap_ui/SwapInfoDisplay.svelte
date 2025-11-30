<script lang="ts">
  import { formatToNonZeroDecimal } from "$lib/utils/numberFormatUtils";
  import { fade } from "svelte/transition";
  import { userTokens } from "$lib/stores/userTokens";
  import { onMount } from "svelte";
  import { app } from "$lib/state/app.state.svelte";

  let {
    payToken = null,
    payAmount = "",
    receiveToken = null,
    receiveAmount = "",
    priceImpact = 0,
    gasFees = [],
    lpFees = [],
    isLoading = false,
  } = $props<{
    payToken: Kong.Token | null;
    payAmount: string;
    receiveToken: Kong.Token | null;
    receiveAmount: string;
    priceImpact: number;
    gasFees: Array<{ amount: string; token: string }>;
    lpFees: Array<{ amount: string; token: string }>;
    isLoading?: boolean;
  }>();

  let isMobile = $derived(app.isMobile);

  // Calculate exchange rate
  const exchangeRate = $derived(() => {
    if (!payAmount || !receiveAmount || !payToken || !receiveToken) return null;
    
    const payValue = parseFloat(payAmount);
    const receiveValue = parseFloat(receiveAmount);
    
    if (payValue && receiveValue) {
      const rate = receiveValue / payValue;
      // Use more compact format for very small numbers
      if (rate < 0.0001) {
        return rate.toExponential(3);
      }
      return formatToNonZeroDecimal(rate);
    }
    return null;
  });

  // Track tokens that need to be fetched and their results
  let tokenFetchPromises = $state<Map<string, Promise<Kong.Token | null>>>(new Map());
  let fetchedTokens = $state<Map<string, Kong.Token>>(new Map());
  
  // Common intermediate tokens that should be pre-loaded
  const COMMON_INTERMEDIATE_TOKENS = [
    "ryjl3-tyaaa-aaaaa-aaaba-cai", // ICP
  ];
  
  // Pre-load common intermediate tokens on mount
  onMount(() => {
    COMMON_INTERMEDIATE_TOKENS.forEach(tokenId => {
      userTokens.getTokenDetails(tokenId).then(token => {
        if (token) {
          fetchedTokens.set(tokenId, token);
          fetchedTokens = new Map(fetchedTokens);
        }
      });
    });
  });

  // Calculate total fees in USD
  const totalFeesUsd = $derived(() => {
    let total = 0;
    
    // Check if we have any fees
    if (!gasFees?.length && !lpFees?.length) {
      return 0;
    }
    
    // Get token data map from the user tokens store for efficient lookup
    const tokenDataMap = $userTokens.tokenData;
    
    // Helper function to find token by address or symbol
    const findToken = (tokenIdentifier: string) => {
      // First check if it matches our pay or receive token by address
      if (payToken?.address === tokenIdentifier || payToken?.canister_id === tokenIdentifier) return payToken;
      if (receiveToken?.address === tokenIdentifier || receiveToken?.canister_id === tokenIdentifier) return receiveToken;
      
      // Then check if it matches by symbol (backward compatibility)
      if (payToken?.symbol === tokenIdentifier) return payToken;
      if (receiveToken?.symbol === tokenIdentifier) return receiveToken;
      
      // Check if we've already fetched this token
      const fetchedToken = fetchedTokens.get(tokenIdentifier);
      if (fetchedToken) return fetchedToken;
      
      // Look up in the token data map by address (which is the canister_id)
      const tokenByAddress = tokenDataMap.get(tokenIdentifier);
      if (tokenByAddress) return tokenByAddress;
      
      // Special case for ICP ledger canister
      if (tokenIdentifier === "ryjl3-tyaaa-aaaaa-aaaba-cai") {
        // Look for ICP token by symbol
        for (const [_, token] of tokenDataMap) {
          if (token.symbol === "ICP") {
            return token;
          }
        }
      }
      
      // If not found by address, search by symbol in all tokens
      // This is a fallback for backward compatibility
      for (const [_, token] of tokenDataMap) {
        if (token.symbol === tokenIdentifier) {
          return token;
        }
      }
      
      return null;
    };
    
    // Helper to fetch missing tokens
    const fetchMissingToken = async (tokenId: string) => {
      if (!tokenFetchPromises.has(tokenId)) {
        const promise = userTokens.getTokenDetails(tokenId);
        tokenFetchPromises.set(tokenId, promise);
      }
      return tokenFetchPromises.get(tokenId);
    };
    
    // Calculate gas fees
    if (gasFees && gasFees.length > 0) {
      gasFees.forEach((fee: { amount: string; token: string }) => {
        if (!fee || !fee.amount || !fee.token) return;
        
        let token = findToken(fee.token);
        
        // If token not found and it looks like a canister ID, try to fetch it
        if (!token && fee.token.length > 10 && !fee.token.includes(' ')) {
          // Trigger async fetch for next render
          fetchMissingToken(fee.token).then(fetchedToken => {
            if (fetchedToken) {
              // Store the fetched token and trigger re-render
              fetchedTokens.set(fee.token, fetchedToken);
              fetchedTokens = new Map(fetchedTokens);
            }
          });
        }
        
        if (token?.metrics?.price && fee.amount) {
          const feeValue = Number(fee.amount) * Number(token.metrics.price);
          if (!isNaN(feeValue)) {
            total += feeValue;
          }
        }
      });
    }
    
    // Calculate LP fees  
    if (lpFees && lpFees.length > 0) {
      lpFees.forEach((fee: { amount: string; token: string }) => {
        if (!fee || !fee.amount || !fee.token) return;
        
        let token = findToken(fee.token);
        
        // If token not found and it looks like a canister ID, try to fetch it
        if (!token && fee.token.length > 10 && !fee.token.includes(' ')) {
          // Trigger async fetch for next render
          fetchMissingToken(fee.token).then(fetchedToken => {
            if (fetchedToken) {
              // Store the fetched token and trigger re-render
              fetchedTokens.set(fee.token, fetchedToken);
              fetchedTokens = new Map(fetchedTokens);
            }
          });
        }
        
        if (token?.metrics?.price && fee.amount) {
          const feeValue = Number(fee.amount) * Number(token.metrics.price);
          if (!isNaN(feeValue)) {
            total += feeValue;
          }
        }
      });
    }
    
    return total;
  });

  // Always show the info display
  const showDisplay = $derived(true);

  // Get warning level for price impact
  const priceImpactLevel = $derived(() => {
    if (priceImpact > 5) return 'critical';
    if (priceImpact > 2) return 'warning';
    return 'normal';
  });
</script>

{#if showDisplay}
  <div class="info-row" transition:fade={{ duration: 150 }}>
    <!-- Exchange Rate -->
    {#if !isMobile}
      <div class="info-item">
        <span class="info-label">Rate</span>
        <span class="info-value">
          {#if isLoading}
            <span class="loading-dot"></span>
          {:else if exchangeRate() && payToken && receiveToken}
            1 {payToken.symbol} = {exchangeRate()} {receiveToken.symbol}
          {:else}
            —
          {/if}
        </span>
      </div>
      <span class="divider"></span>
    {/if}

    <!-- Price Impact -->
    <div class="info-item">
      <span class="info-label">Price Impact</span>
      <span class="info-value impact-{priceImpactLevel()}">
        {#if isLoading}
          <span class="loading-dot"></span>
        {:else if payToken && receiveToken && payAmount && receiveAmount && payAmount !== "0" && receiveAmount !== "0"}
          {priceImpact.toFixed(2)}%
        {:else}
          —
        {/if}
      </span>
    </div>
    <span class="divider"></span>

    <!-- Total Fees -->
    <div class="info-item">
      <span class="info-label">Fees</span>
      <span class="info-value">
        {#if isLoading}
          <span class="loading-dot"></span>
        {:else if totalFeesUsd() > 0}
          ${formatToNonZeroDecimal(totalFeesUsd())}
        {:else}
          —
        {/if}
      </span>
    </div>
  </div>
{/if}

<style lang="postcss" scoped>
  .info-row {
    @apply flex items-center justify-center gap-3 py-2 px-3;
    @apply text-xs;
  }

  .info-item {
    @apply flex items-center gap-1.5;
  }

  .info-label {
    @apply text-kong-text-secondary/60 font-normal;
  }

  .info-value {
    @apply text-kong-text-primary/80 font-medium tabular-nums;
  }

  .divider {
    @apply w-px h-3 bg-kong-text-secondary/20;
  }

  /* Price impact color coding */
  .impact-normal {
    @apply text-kong-text-primary/80;
  }

  .impact-warning {
    @apply text-kong-warning;
  }

  .impact-critical {
    @apply text-kong-error;
  }

  /* Loading state */
  .loading-dot {
    @apply inline-block w-1.5 h-1.5 rounded-full bg-kong-text-secondary/40;
    animation: pulse-fade 1s ease-in-out infinite;
  }

  @keyframes pulse-fade {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 0.8; }
  }

  /* Mobile */
  @media (max-width: 640px) {
    .info-row {
      @apply gap-2 text-[11px];
    }

    .info-item {
      @apply gap-1;
    }

    .divider {
      @apply h-2.5;
    }
  }
</style> 