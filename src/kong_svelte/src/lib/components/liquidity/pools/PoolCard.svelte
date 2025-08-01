<script lang="ts">
  import Panel from "$lib/components/common/Panel.svelte";
  import TokenImages from "$lib/components/common/TokenImages.svelte";
  import ChainBadge from "$lib/components/common/ChainBadge.svelte";
  import { formatToNonZeroDecimal } from "$lib/utils/numberFormatUtils";
  import { formatUsdValue } from "$lib/utils/tokenFormatters";
  import { getPoolPriceUsd } from "$lib/utils/statsUtils";
  import { calculatePoolTVL } from "$lib/utils/liquidityUtils";

  interface PoolCardProps {
    pool: BE.Pool;
    tokenMap: Map<string, any>;
    userPoolData?: any;
    isHighlighted?: boolean;
    isMobile?: boolean;
    isConnected?: boolean;
    onClick: () => void;
  }

  let { 
    pool, 
    tokenMap, 
    userPoolData = null, 
    isHighlighted = false, 
    isMobile = false,
    isConnected = false,
    onClick 
  }: PoolCardProps = $props();

  // Calculate TVL using token prices
  const calculatedTVL = $derived(() => {
    const token0 = tokenMap.get(pool.address_0) || pool.token0;
    const token1 = tokenMap.get(pool.address_1) || pool.token1;
    
    // Get token prices, defaulting to pool price for stablecoins
    const token0Price = token0?.metrics?.price || 
      (pool.symbol_0 === 'ckUSDT' || pool.symbol_0 === 'USDC' ? 1 : 
       pool.symbol_0 === 'SOL' ? 144.38 : 
       pool.symbol_0 === 'ICP' ? 3.18 : 
       pool.symbol_0 === 'KONG' ? 0.0287 : 0);
       
    const token1Price = token1?.metrics?.price || 
      (pool.symbol_1 === 'ckUSDT' || pool.symbol_1 === 'USDC' ? 1 : 0);
    
    return calculatePoolTVL(pool, token0Price, token1Price);
  });

  const stats = $derived([
    { 
      label: "APR", 
      value: pool.apr ? `${pool.apr}%` : Number(pool.rolling_24h_apy || 0) > 0 ? `${Number(pool.rolling_24h_apy).toFixed(2)}%` : "Early Access",
      color: Number(pool.rolling_24h_apy || 0) > 0 ? 'text-kong-primary' : 'text-kong-text-secondary',
      isEarlyAccess: !pool.apr && Number(pool.rolling_24h_apy || 0) === 0
    },
    { 
      label: "Price", 
      value: pool.price ? `${formatToNonZeroDecimal(parseFloat(pool.price))} ${pool.symbol_1}` : pool.tvl !== undefined ? `${(pool.symbol_1 || pool.token1?.symbol) === "ckUSDT" ? "$" : ""}${formatToNonZeroDecimal(getPoolPriceUsd(pool))}${(pool.symbol_1 || pool.token1?.symbol) === "ckUSDT" ? "" : " " + (pool.symbol_1 || pool.token1?.symbol)}` : "--"
    },
    { 
      label: "TVL", 
      value: formatUsdValue(pool.tvl || calculatedTVL()) 
    },
    { 
      label: "Vol 24h", 
      value: Number(pool.rolling_24h_volume || 0) > 0 ? formatUsdValue(Number(pool.rolling_24h_volume)) : "Early Access",
      color: Number(pool.rolling_24h_volume || 0) > 0 ? 'text-kong-text-primary' : 'text-kong-text-secondary',
      isEarlyAccess: Number(pool.rolling_24h_volume || 0) === 0
    },
    ...(isConnected ? [{
      label: "Your Position",
      value: userPoolData ? `$${userPoolData.usdValue || 0}` : "$0",
      color: userPoolData ? 'text-kong-success' : 'text-kong-text-secondary'
    }] : [])
  ]);

  const mobileStats = $derived(stats.slice(0, 4));
</script>

<Panel
  interactive={true}
  onclick={onClick}
  className={`cursor-pointer transition-all ${isMobile
    ? "active:scale-[0.99]"
    : "hover:scale-[1.02] active:scale-[0.98]"} ${isHighlighted
    ? 'bg-gradient-to-br from-[rgba(0,149,235,0.05)] to-[rgba(0,149,235,0.02)] shadow-[inset_0_1px_1px_rgba(0,149,235,0.1)]'
    : ''}`}
  unpadded={true}
>
  <div class="p-4 h-full flex flex-col">
    <!-- Pool Info Row -->
    <div class="flex items-center gap-{isMobile ? '2.5' : '3'} mb-4">
      <TokenImages
        tokens={[
          tokenMap.get(pool.address_0) || pool.token0,
          tokenMap.get(pool.address_1) || pool.token1,
        ]}
        size={isMobile ? 28 : 36}
        overlap={!isMobile}
      />
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2">
          <div
            class="text-base font-{isMobile ? 'medium' : 'semibold'} text-kong-text-primary truncate"
          >
            {pool.symbol_0 || pool.token0?.symbol}/{pool.symbol_1 || pool.token1?.symbol}
          </div>
          {#if !isMobile && pool.chain_0 && pool.chain_1}
            {#if pool.chain_0 !== pool.chain_1}
              <div class="flex items-center gap-1">
                <ChainBadge chain={pool.chain_0} size="small" variant="minimal" />
                <span class="text-kong-text-secondary text-xs">·</span>
                <ChainBadge chain={pool.chain_1} size="small" variant="minimal" />
              </div>
            {:else}
              <ChainBadge chain={pool.chain_0} size="small" variant="minimal" />
            {/if}
          {/if}
        </div>
        {#if !isMobile}
          <div class="text-xs text-kong-text-secondary truncate">
            {pool.name || `${pool.symbol_0 || pool.token0?.symbol}/${pool.symbol_1 || pool.token1?.symbol} Pool`}
          </div>
        {/if}
        {#if userPoolData}
          <div class="flex items-center gap-2">
            <div class="text-xs text-kong-accent-blue">
              {userPoolData.sharePercentage}% of pool
            </div>
          </div>
        {/if}
      </div>
    </div>

    <!-- Stats Grid -->
    <div class="{!userPoolData ? 'flex-1 flex flex-col justify-end' : ''}">
      {#if isMobile}
        <div class="grid grid-cols-2 gap-3">
          {#each mobileStats as stat}
            <div class="bg-black/10 rounded-lg p-2.5">
              <div class="text-xs text-kong-text-secondary mb-1">
                {stat.label}
              </div>
              <div class="text-sm font-medium {stat.color || 'text-kong-text-primary'} {stat.isEarlyAccess ? 'opacity-50 italic text-xs' : ''}">
                {stat.value}
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="space-y-3">
          {#each stats as stat}
            <div class="flex justify-between items-center">
              <span class="text-sm text-kong-text-secondary">{stat.label}</span>
              <span class="text-sm font-medium {stat.color || 'text-kong-text-primary'} {stat.isEarlyAccess ? 'opacity-50 italic text-xs' : ''}">{stat.value}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</Panel>