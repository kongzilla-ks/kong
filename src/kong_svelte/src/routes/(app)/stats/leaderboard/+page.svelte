<script lang="ts">
  import { browser } from '$app/environment';
  import Panel from '$lib/components/common/Panel.svelte';
  import { Trophy, BarChart3, Users, TrendingUp, Activity } from 'lucide-svelte';
  import DemoMessage from '$lib/components/common/DemoMessage.svelte';
  
  // Import the components
  import LeaderboardTraderCard from '$lib/components/stats/LeaderboardTraderCard.svelte';
  import LoadingIndicator from '$lib/components/common/LoadingIndicator.svelte';
  import ErrorState from '$lib/components/common/ErrorState.svelte';
  import EmptyState from '$lib/components/common/EmptyState.svelte';
  import PageHeader from '$lib/components/common/PageHeader.svelte';
  
  // Import utility functions
  import { formatVolume, formatNumberWithCommas } from '$lib/utils/numberFormatUtils';
  
  // Import the store
  import { 
    leaderboardStore, 
    isLoading, 
    error, 
    leaderboardData, 
    totalVolume, 
    totalTraders 
  } from '$lib/stores/leaderboardStore';
  import type { Period } from '$lib/types';
  
  // State variables using runes
  let selectedPeriod = $state<Period>('day');
  let expandedRowIndex = $state<number | null>(null);
  let tradedTokens = $state<Record<number, any>>({});
  let loadingTokens = $state<Record<number, boolean>>({});
  let tokenErrors = $state<Record<number, string | null>>({});
  let userDetails = $state<Record<number, any>>({});
  let loadingUserDetails = $state<Record<number, boolean>>({});
  
  // Derived values from stores
  const isLoadingValue = $derived($isLoading);
  const errorValue = $derived($error);
  const leaderboardDataValue = $derived($leaderboardData);
  const totalVolumeValue = $derived($totalVolume);
  const totalTradersValue = $derived($totalTraders);
  
  // Initialize state from store
  $effect(() => {
    const state = $leaderboardStore;
    selectedPeriod = state.selectedPeriod;
    expandedRowIndex = state.expandedRowIndex;
    tradedTokens = state.tradedTokens;
    loadingTokens = state.loadingTokens;
    tokenErrors = state.tokenErrors;
    userDetails = state.userDetails;
    loadingUserDetails = state.loadingUserDetails;
  });
  
  // Handle period change
  function handlePeriodChange(period: Period) {
    leaderboardStore.setPeriod(period);
  }
  
  // Toggle row expansion
  function toggleRowExpansion(index: number) {
    leaderboardStore.toggleRowExpansion(index);
  }
  
  // Load data on mount
  $effect(() => {
    if (browser) {
      leaderboardStore.loadLeaderboard(selectedPeriod);
    }
  });
</script>

<svelte:head>
  <title>Trading Leaderboard - KongSwap</title>
</svelte:head>

<DemoMessage 
  feature="Trading Leaderboard" 
/>

<style>
  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(10px); }
    to { opacity: 1; transform: translateY(0); }
  }
  
  /* Add glow effect for top traders */
  :global(.border-yellow-400) {
    box-shadow: 0 0 15px rgba(250, 204, 21, 0.3);
  }
  
  :global(.border-amber-600) {
    box-shadow: 0 0 12px rgba(217, 119, 6, 0.25);
  }
  
  :global(.border-gray-300) {
    box-shadow: 0 0 12px rgba(209, 213, 219, 0.25);
  }
</style>
