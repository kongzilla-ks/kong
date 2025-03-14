<script lang="ts">
  import { formatUsdValue } from "$lib/utils/tokenFormatters";
  import { formatToNonZeroDecimal } from "$lib/utils/numberFormatUtils";
  import Panel from "$lib/components/common/Panel.svelte";
  import { InfoIcon } from "lucide-svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { fetchTokensByCanisterId } from "$lib/api/tokens";
  import { cubicOut } from 'svelte/easing';
  import { tweened } from 'svelte/motion';

  // Props using $props() for Svelte 5 runes mode
  const { token, marketCapRank = null } = $props<{
    token: FE.Token;
    marketCapRank: number | null;
  }>();

  // State variables
  let previousPrice: number | null = $state(null);
  let priceFlash: 'up' | 'down' | null = $state(null);
  let priceFlashTimeout: NodeJS.Timeout;
  
  // Live token data (replacing writable store)
  let liveToken: FE.Token | null = $state(null);
  
  // Motion values (replacing tweened stores)
  let marketCapValue = $state(0);
  let volume24hValue = $state(0);
  let totalSupplyValue = $state(0);
  let circulatingSupplyValue = $state(0);
  
  // Create tweened motions
  const marketCapMotion = tweened(0, { duration: 500, easing: cubicOut });
  const volume24hMotion = tweened(0, { duration: 500, easing: cubicOut });
  const totalSupplyMotion = tweened(0, { duration: 500, easing: cubicOut });
  const circulatingSupplyMotion = tweened(0, { duration: 500, easing: cubicOut });
  
  // Derived values from motions
  const marketCap = $derived($marketCapMotion);
  const volume24h = $derived($volume24hMotion);
  const totalSupplyTweened = $derived($totalSupplyMotion);
  const circulatingSupplyTweened = $derived($circulatingSupplyMotion);

  // Function to update token data
  async function updateTokenData() {
    try {
      const result = await fetchTokensByCanisterId([token.canister_id]);
      if (result && result[0]) {
        // Update state variable
        liveToken = { ...result[0] };
      }
    } catch (error) {
      console.error("Error fetching token data:", error);
    }
  }

  // Initial fetch
  updateTokenData();

  // Derived active token (replacing reactive statement)
  const activeToken = $derived(liveToken || token);

  // Track price changes with effect
  $effect(() => {
    const currentPrice = Number(activeToken?.metrics?.price || 0);
    if (previousPrice !== null && currentPrice !== previousPrice) {
      if (priceFlashTimeout) {
        clearTimeout(priceFlashTimeout);
      }
      priceFlash = currentPrice > previousPrice ? 'up' : 'down';
      priceFlashTimeout = setTimeout(() => priceFlash = null, 1000);
    }
    previousPrice = currentPrice;
  });

  // Update motion values with effect
  $effect(() => {
    if (activeToken?.metrics) {
      marketCapValue = Number(activeToken.metrics.market_cap || 0);
      volume24hValue = Number(activeToken.metrics.volume_24h || 0);
      totalSupplyValue = Number(activeToken.metrics.total_supply || 0) / 10 ** activeToken.decimals;
      circulatingSupplyValue = Number(activeToken.metrics.total_supply || 0) / 10 ** activeToken.decimals;
      
      // Update motion values
      marketCapMotion.set(marketCapValue);
      volume24hMotion.set(volume24hValue);
      totalSupplyMotion.set(totalSupplyValue);
      circulatingSupplyMotion.set(circulatingSupplyValue);
    }
  });

  function calculateVolumePercentage(volume: number, marketCap: number): string {
    if (!marketCap) return "0.00%";
    return ((volume / marketCap) * 100).toFixed(2) + "%";
  }

  // Lifecycle with cleanup
  $effect.root(() => {
    const pollInterval = setInterval(async () => {
      try {
        await updateTokenData();
      } catch (error) {
        console.error("Error polling token data:", error);
      }
    }, 1000 * 10); // 10 seconds

    return () => {
      clearInterval(pollInterval);
      if (priceFlashTimeout) {
        clearTimeout(priceFlashTimeout);
      }
      marketCapMotion.set(0);
      volume24hMotion.set(0);
      totalSupplyMotion.set(0);
      circulatingSupplyMotion.set(0);
    };
  });

  // Derived formatted values
  const formattedPrice = $derived(formatToNonZeroDecimal(activeToken?.metrics?.price));
  const formattedPriceChange24h = $derived(Number(activeToken?.metrics?.price_change_24h) || 0);
</script>

<Panel variant="transparent" type="main" className="p-6">
  <div class="flex flex-col gap-8">
    <!-- Price Section -->
    <div>
      <div class="text-sm text-kong-text-primary/50 uppercase tracking-wider mb-2"
      >
      <span class="flex gap-x-2 items-center">
        Current Price       <span use:tooltip={{
          text: "This is a weighted average price of all pools",
          direction: "bottom",
        }}><InfoIcon size={16} /> 
      </span>
      </div>
      <div class="flex flex-col gap-2">
        <div 
          class="text-[32px] font-medium text-kong-text-primary"
          class:flash-green-text={priceFlash === 'up'}
          class:flash-red-text={priceFlash === 'down'}
        >
          ${formattedPrice}
        </div>
        {#if formattedPriceChange24h}
          <div class="text-sm">
            <span
              class={formattedPriceChange24h > 0 ? "text-green-400" : "text-red-400"}
            >
              {formattedPriceChange24h > 0 ? "+" : ""}{formattedPriceChange24h.toFixed(2)}%
            </span>
            <span class="text-kong-text-primary/40 ml-1">24h</span>
          </div>
        {/if}
      </div>
    </div>
    
    <!-- Market Stats -->
    <div class="grid grid-cols-2 gap-6">
      <!-- Market Cap -->
      <div>
        <div class="text-sm text-kong-text-primary/50 uppercase tracking-wider mb-2">Market Cap</div>
        <div class="text-xl font-medium text-kong-text-primary">
          {formatUsdValue(marketCap)}
        </div>
        <div class="text-sm text-kong-text-primary/40 mt-1">
          Rank #{marketCapRank !== null ? marketCapRank : "N/A"}
        </div>
      </div>
      
      <!-- 24h Volume -->
      <div>
        <div class="text-sm text-kong-text-primary/50 uppercase tracking-wider mb-2">24h Volume</div>
        <div class="text-xl font-medium text-kong-text-primary">
          {formatUsdValue(volume24h)}
        </div>
        <div class="text-sm text-kong-text-primary/40 mt-1">
          {activeToken.metrics.volume_24h
            ? `${calculateVolumePercentage(Number(activeToken.metrics.volume_24h), Number(activeToken.metrics.market_cap))} of mcap`
            : "No volume data"}
        </div>
      </div>
      
      <!-- Total Supply -->
      <div>
        <div class="text-sm text-kong-text-primary/50 uppercase tracking-wider mb-2">Total Supply</div>
        <div class="text-xl font-medium text-kong-text-primary">
          {formatToNonZeroDecimal(totalSupplyTweened)}
        </div>
        <div class="text-sm text-kong-text-primary/40 mt-1">
          {activeToken?.symbol || ""} tokens
        </div>
      </div>

      <!-- Circl Supply -->
      <div>
        <div class="text-sm text-kong-text-primary/50 uppercase tracking-wider mb-2">Circulating Supply</div>
        <div class="text-xl font-medium text-kong-text-primary">
          {formatToNonZeroDecimal(circulatingSupplyTweened)}
        </div>
        <div class="text-sm text-kong-text-primary/40 mt-1">
          {activeToken?.symbol || ""} tokens
        </div>
      </div>
    </div>
  </div>
</Panel>

<style scoped>
  .flash-green-text {
    animation: flashGreen 1s ease-out;
  }
  
  .flash-red-text {
    animation: flashRed 1s ease-out;
  }
  
  @keyframes flashGreen {
    0% { color: rgb(34, 197, 94); }
    100% { color: inherit; }
  }
  
  @keyframes flashRed {
    0% { color: rgb(239, 68, 68); }
    100% { color: inherit; }
  }
</style> 