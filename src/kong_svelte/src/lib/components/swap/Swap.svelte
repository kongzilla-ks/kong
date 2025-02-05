<script lang="ts">
  import SwapPanel from "./swap_ui/SwapPanel.svelte";
  import TokenSelectorDropdown from "./swap_ui/TokenSelectorDropdown.svelte";
  import SwapConfirmation from "./swap_ui/SwapConfirmation.svelte";
  import Portal from "svelte-portal";
  import { Principal } from "@dfinity/principal";
  import { fade } from "svelte/transition";
  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import { replaceState, afterNavigate } from "$app/navigation";
  import { page } from "$app/stores";
  import { SwapLogicService } from "$lib/services/swap/SwapLogicService";
  import { swapState } from "$lib/services/swap/SwapStateService";
  import { SwapService } from "$lib/services/swap/SwapService";
  import { auth } from "$lib/services/auth";
  import {
    getTokenDecimals,
    loadBalances,
  } from "$lib/services/tokens/tokenStore";
  import { settingsStore } from "$lib/services/settings/settingsStore";
  import { toastStore } from "$lib/stores/toastStore";
  import { swapStatusStore } from "$lib/services/swap/swapStore";
  import { sidebarStore } from "$lib/stores/sidebarStore";
  import {
    KONG_BACKEND_CANISTER_ID,
  } from "$lib/constants/canisterConstants";
  import { livePools } from "$lib/services/pools/poolStore";
  import Settings from "$lib/components/settings/Settings.svelte";
  import Modal from "$lib/components/common/Modal.svelte";
    import SwapSuccessModal from "./swap_ui/SwapSuccessModal.svelte";
    import { userTokens } from "$lib/stores/userTokens";
  import { browser } from '$app/environment';

  // Types
  type PanelType = "pay" | "receive";
  interface PanelConfig {
    id: string;
    type: PanelType;
    title: string;
  }

  // Props
  export let initialFromToken: FE.Token | null = null;
  export let initialToToken: FE.Token | null = null;
  export let currentMode: "normal" | "pro";

  const PANELS: PanelConfig[] = [
    { id: "pay", type: "pay", title: "You Pay" },
    { id: "receive", type: "receive", title: "You Receive" },
  ];

  // Constants for dropdown positioning
  const DROPDOWN_WIDTH = 360; // Width of dropdown
  const MARGIN = 16; // Margin from edges
  const SEARCH_HEADER_HEIGHT = 56; // Height of search header

  // State
  let isProcessing = false;
  let isInitialized = false;
  let currentSwapId: string | null = null;
  let previousMode = currentMode;
  let rotationCount = 0;
  let isQuoteLoading = false;
  let showSettings = false;

  // Add these variables to track URL params
  let currentToken0Id: string | null = null;
  let currentToken1Id: string | null = null;

  // Add these near the other state variables at the top
  let token0Id: string | null = null;
  let token1Id: string | null = null;

  // Function to calculate optimal dropdown position
  function getDropdownPosition(
    pos: { x: number; y: number; windowWidth: number } | null,
  ) {
    if (!pos) return { top: 0, left: 0 };

    // Position dropdown to the right of the button
    let left = pos.x;

    // If it would overflow right edge, position to the left of the button instead
    if (left + DROPDOWN_WIDTH > pos.windowWidth - MARGIN) {
      left = Math.max(MARGIN, pos.x - DROPDOWN_WIDTH - 8);
    }

    // Align the first token item with the button by offsetting the search header height
    const top = pos.y - SEARCH_HEADER_HEIGHT - 8; // 8px for the padding of first token

    return { top, left };
  }

  // Reactive statements

  $: insufficientFunds =
    $swapState.payToken &&
    $swapState.payAmount &&
    Number($swapState.payAmount) >
      Number(loadBalances($auth.account?.owner, {
        tokens: [$swapState.payToken],
        forceRefresh: true,
      }));

  $: buttonText = (() => {
    if (!$swapState.payToken || !$swapState.receiveToken)
      return "Select Tokens";
    if ($swapState.isProcessing) return "Processing...";
    if (isQuoteLoading) return "Fetching Quote...";
    if ($swapState.error) return $swapState.error;
    if (insufficientFunds) return "Insufficient Funds";
    if ($swapState.swapSlippage > $settingsStore.max_slippage)
      return `High Slippage (${$swapState.swapSlippage.toFixed(2)}% > ${$settingsStore.max_slippage}%) - Click to Adjust`;
    if (!$auth?.account?.owner) return "Click to Connect Wallet";
    if (!$swapState.payAmount) return "Enter Amount";
    return "SWAP";
  })();

  // Initialize tokens when they become available
  $: if (!isInitialized && initialFromToken !== undefined && initialToToken !== undefined) {
    isInitialized = true;
    swapState.update((state) => ({
      ...state,
      payToken: initialFromToken,
      receiveToken: initialToToken,
      payAmount: "",
      receiveAmount: "",
    }));
  }

  // Add this function to handle initial URL params
  function initializeFromUrl() {
    if (!browser || !$userTokens.tokens.length) return;

    const token0Id = $page.url.searchParams.get("from") || $page.url.searchParams.get("token0");
    const token1Id = $page.url.searchParams.get("to") || $page.url.searchParams.get("token1");

    if (!token0Id && !token1Id) return;

    const token0 = token0Id ? $userTokens.tokens.find((t) => t.canister_id === token0Id) : null;
    const token1 = token1Id ? $userTokens.tokens.find((t) => t.canister_id === token1Id) : null;

    if (token0 || token1) {
      swapState.update((state) => ({
        ...state,
        payToken: token0 || state.payToken,
        receiveToken: token1 || state.receiveToken,
        payAmount: "",
        receiveAmount: "",
        error: null,
      }));
    }
  }

  // Update onMount to handle URL parameters
  onMount(() => {
    if (browser) {
      initializeFromUrl();
      
      // Set up page store subscription for URL changes
      const unsubscribe = page.subscribe(() => {
        if (browser) {
          initializeFromUrl();
        }
      });

      return () => {
        unsubscribe();
      };
    }
  });

  // Modify the poolExists function to add more debugging
  function poolExists(
    payToken: FE.Token | null,
    receiveToken: FE.Token | null,
  ): boolean {
    if (!payToken || !receiveToken) {
      return false;
    }

    if ($livePools.length === 0) {
      return true; // Return true when pools aren't loaded yet
    }

    const exists = $livePools.some(
      (pool) =>
        (pool.symbol_0 === payToken.symbol &&
          pool.symbol_1 === receiveToken.symbol) ||
        (pool.symbol_0 === receiveToken.symbol &&
          pool.symbol_1 === payToken.symbol),
    );

    return exists;
  }

  async function handleSwapClick(): Promise<void> {
    if (!$auth.isConnected) {
      sidebarStore.toggleExpand();
      return;
    }

    if (!$swapState.payToken || !$swapState.receiveToken) return;

    swapState.update((state) => ({
      ...state,
      showConfirmation: true,
      isProcessing: false,
      error: null,
      showSuccessModal: false,
    }));
  }

  async function handleSwap(): Promise<boolean> {
    if (
      !$swapState.payToken ||
      !$swapState.receiveToken ||
      !$swapState.payAmount ||
      $swapState.isProcessing
    ) {
      return false;
    }

    try {
      swapState.update((state) => ({
        ...state,
        isProcessing: true,
        error: null,
      }));

      currentSwapId = swapStatusStore.addSwap({
        expectedReceiveAmount: $swapState.receiveAmount,
        lastPayAmount: $swapState.payAmount,
        payToken: $swapState.payToken,
        receiveToken: $swapState.receiveToken,
        payDecimals: Number(
          getTokenDecimals($swapState.payToken.canister_id).toString(),
        ),
      });

      const result = await SwapService.executeSwap({
        swapId: currentSwapId,
        payToken: $swapState.payToken,
        payAmount: $swapState.payAmount,
        receiveToken: $swapState.receiveToken,
        receiveAmount: $swapState.receiveAmount,
        userMaxSlippage: $settingsStore.max_slippage,
        backendPrincipal: Principal.fromText(KONG_BACKEND_CANISTER_ID),
        lpFees: $swapState.lpFees,
      });

      if (typeof result !== 'bigint') {
        swapState.update((state) => ({
          ...state,
          isProcessing: false,
          error: "Swap failed",
        }));
        return false;
      }

      // Store the successful swap details and clear input amounts
      swapState.update((state) => ({
        ...state,
        successDetails: {
          payAmount: state.payAmount,
          payToken: state.payToken,
          receiveAmount: state.receiveAmount,
          receiveToken: state.receiveToken,
          principalId: $auth.account?.owner?.toString() || "",
        },
        // Clear input amounts but keep tokens selected
        payAmount: "",
        receiveAmount: "",
        isProcessing: false,
        showConfirmation: false,
      }));

      return true;
    } catch (error) {
      console.error("Swap execution failed:", error);
      swapState.update((state) => ({
        ...state,
        isProcessing: false,
        error: error.message || "Swap failed",
      }));
      return false;
    } finally {
      // Always reset processing state if the swap fails
      if (!$swapStatusStore[currentSwapId]?.details) {
        swapState.update((state) => ({
          ...state,
          isProcessing: false,
        }));
      }
    }
  }

  async function handleButtonAction(): Promise<void> {
    if (!$auth.isConnected) {
      sidebarStore.open();
      return;
    }

    if ($swapState.swapSlippage > $settingsStore.max_slippage) {
      showSettings = true;
      return;
    }

    if (insufficientFunds) {
      toastStore.error("Insufficient funds for this swap");
      return;
    }

    if (!insufficientFunds && $swapState.payAmount) {
      await handleSwapClick();
    }
  }

  // Initialization functions
  async function initializeComponent(): Promise<void> {
    try {
      // Only initialize default tokens if no URL parameters are present
      const token0Id = get(page).url.searchParams.get("token0");
      const token1Id = get(page).url.searchParams.get("token1");

      if (!token0Id && !token1Id && !isInitialized && $userTokens.tokens.length > 0) {
        isInitialized = true;
        swapState.initializeTokens(initialFromToken, initialToToken);
      }
    } catch (error) {
      console.error("Error initializing component:", error);
    }
  }

  async function handleAmountChange(event: CustomEvent) {
    const { value, panelType } = event.detail;

    if (panelType === "pay") {
      swapState.setPayAmount(value);
      await updateSwapQuote();
    } else {
      swapState.setReceiveAmount(value);
    }
  }

  // Update the handleTokenSelect function to be simpler
  function handleTokenSelect(panelType: PanelType) {
    if (panelType === "pay") {
      swapState.update((s) => ({
        ...s,
        showPayTokenSelector: true,
        error: null,
      }));
    } else {
      swapState.update((s) => ({
        ...s,
        showReceiveTokenSelector: true,
        error: null,
      }));
    }
  }

  async function handleReverseTokens() {
    if ($swapState.isProcessing) return;

    rotationCount++;
    
    const tempPayToken = $swapState.payToken;
    const tempPayAmount = $swapState.payAmount;
    const tempReceiveAmount = $swapState.receiveAmount;

    // Update tokens and reset error state
    swapState.update((s) => ({
      ...s,
      payToken: s.receiveToken,
      receiveToken: tempPayToken,
      error: null,
    }));

    // Load both balances at once after reversing
    if ($swapState.payToken && $swapState.receiveToken) {
      await loadBalances(auth?.pnp?.account?.owner?.toString(), { 
        tokens: [$swapState.payToken, $swapState.receiveToken], 
        forceRefresh: true 
      });
    }

    // Set the new pay amount
    if (tempReceiveAmount && tempReceiveAmount !== "0") {
      swapState.setPayAmount(tempReceiveAmount);
    } else if (tempPayAmount) {
      swapState.setPayAmount(tempPayAmount);
    }

    // Update quote with reversed tokens
    await updateSwapQuote();

    // Update URL params
    if (
      $swapState.payToken?.canister_id &&
      $swapState.receiveToken?.canister_id
    ) {
      updateTokenInURL("from", $swapState.payToken.canister_id);
      updateTokenInURL("to", $swapState.receiveToken.canister_id);
    }
  }

  // Update the updateTokenInURL function to use the correct parameter names
  function updateTokenInURL(param: "from" | "to", tokenId: string) {
    if (!browser) return;
    
    const url = new URL(window.location.href);
    url.searchParams.set(param, tokenId);
    replaceState(url.toString(), {});
  }

  // Add a new state for quote loading
  let quoteUpdateTimeout: NodeJS.Timeout;

  async function updateSwapQuote() {
    const state = get(swapState);

    if (
      !state.payToken ||
      !state.receiveToken ||
      !hasValidPool ||
      !state.payAmount ||
      state.payAmount === "0"
    ) {
      swapState.update((s) => ({
        ...s,
        receiveAmount: "0",
        swapSlippage: 0,
      }));
      return;
    }

    // Clear any pending timeout
    if (quoteUpdateTimeout) {
      clearTimeout(quoteUpdateTimeout);
    }

    // Set loading state immediately
    isQuoteLoading = true;

    // Debounce the quote update
    quoteUpdateTimeout = setTimeout(async () => {
      try {
        const quote = await SwapService.getSwapQuote(
          state.payToken,
          state.receiveToken,
          state.payAmount,
        );

        swapState.update((s) => ({
          ...s,
          receiveAmount: quote.receiveAmount,
          swapSlippage: quote.slippage,
        }));
      } catch (error) {
        console.error("Error getting quote:", error);
        swapState.update((s) => ({
          ...s,
          receiveAmount: "0",
          swapSlippage: 0,
          error: "Failed to get quote",
        }));
      } finally {
        isQuoteLoading = false;
      }
    }, 600); // 600ms debounce
  }

  let previousPayAmount = "";
  let previousPayToken = null;
  let previousReceiveToken = null;

  $: {
    // Only update quote if relevant values have actually changed
    if (
      $swapState.payToken &&
      $swapState.receiveToken &&
      $swapState.payAmount &&
      ($swapState.payAmount !== previousPayAmount ||
        $swapState.payToken?.canister_id !== previousPayToken?.canister_id ||
        $swapState.receiveToken?.canister_id !==
          previousReceiveToken?.canister_id)
    ) {
      previousPayAmount = $swapState.payAmount;
      previousPayToken = $swapState.payToken;
      previousReceiveToken = $swapState.receiveToken;

      updateSwapQuote();
    }
  }

  // Add this to the reactive statements section
  $: if (currentMode !== previousMode) {
    resetSwapState();
  }

  // Add this function to handle resetting state
  function resetSwapState() {
    // Cancel any pending quote updates
    if (quoteUpdateTimeout) {
      clearTimeout(quoteUpdateTimeout);
    }

    // Reset quote loading state
    isQuoteLoading = false;

    // Immediately reset all relevant state
    swapState.update((state) => ({
      ...state,
      payAmount: "",
      receiveAmount: "",
      error: null,
      isProcessing: false,
      showConfirmation: false,
      showBananaRain: false,
      swapSlippage: 0, // Reset slippage
      lpFees: null, // Reset fees
      routingPath: null, // Reset routing
    }));

    // Reset previous values to prevent unnecessary quote updates
    previousPayAmount = "";
    previousPayToken = null;
    previousReceiveToken = null;
  }

  // Add a reactive statement to check pool existence
  $: hasValidPool = poolExists($swapState.payToken, $swapState.receiveToken);

  // Add a reactive statement to update button state when tokens change
  $: {
    if ($swapState.payToken || $swapState.receiveToken) {
      loadBalances(auth?.pnp?.account?.owner?.toString(), { 
        tokens: [$swapState.payToken, $swapState.receiveToken], 
        forceRefresh: true 
      });
      swapState.update((s) => ({
        ...s,
        error: null, // Reset error state when tokens change
      }));
    }
  }

  // Add reactive statement to load balances when auth becomes available
  $: if ($auth.isConnected && $swapState.payToken && $swapState.receiveToken) {
    loadBalances($auth.account?.owner?.toString(), {
      tokens: [$swapState.payToken, $swapState.receiveToken],
      forceRefresh: true
    });
  }

  // Add this near your other lifecycle hooks
  onDestroy(() => {
    resetSwapState();
  });
</script>

<div class="swap-container" in:fade={{ duration: 420 }}>
  <div class="panels-container">
    <div class="panels-wrapper">
      <div class="panel">
        <SwapPanel
          title={PANELS[0].title}
          token={$swapState.payToken}
          amount={$swapState.payAmount}
          onAmountChange={handleAmountChange}
          onTokenSelect={() => handleTokenSelect("pay")}
          showPrice={false}
          slippage={$swapState.swapSlippage}
          disabled={false}
          panelType="pay"
          otherToken={$swapState.receiveToken}
        />
      </div>

      <button
        class="switch-button"
        class:disabled={isProcessing}
        style="--rotation-count: {rotationCount}"
        on:click={handleReverseTokens}
        disabled={isProcessing}
        aria-label="Switch tokens position"
      >
        <div class="switch-button-inner">
          <svg
            class="switch-icon"
            viewBox="0 0 24 24"
            width="24"
            height="24"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M7.5 3.5L4.5 6.5L7.5 9.5M4.5 6.5H16.5C18.71 6.5 20.5 8.29 20.5 10.5C20.5 11.48 20.14 12.37 19.55 13.05M16.5 20.5L19.5 17.5L16.5 14.5M19.5 17.5H7.5C5.29 17.5 3.5 15.71 3.5 13.5C3.5 12.52 3.86 11.63 4.45 10.95"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </div>
      </button>

      <div class="panel">
        <SwapPanel
          title={PANELS[1].title}
          token={$swapState.receiveToken}
          amount={$swapState.receiveAmount}
          onAmountChange={handleAmountChange}
          onTokenSelect={() => handleTokenSelect("receive")}
          showPrice={true}
          slippage={$swapState.swapSlippage}
          disabled={false}
          panelType="receive"
          otherToken={$swapState.payToken}
          isLoading={isQuoteLoading}
        />
      </div>
    </div>

    <div class="swap-footer">
      <button
        class="swap-button"
        class:error={$swapState.error ||
          $swapState.swapSlippage > $settingsStore.max_slippage ||
          insufficientFunds
          }
        class:processing={$swapState.isProcessing || isQuoteLoading}
        class:ready={!$swapState.error &&
          $swapState.swapSlippage <= $settingsStore.max_slippage &&
          !insufficientFunds &&
          !isQuoteLoading
        }
        class:shine-animation={buttonText === "SWAP"}
        on:click={handleButtonAction}
        disabled={$swapState.isProcessing || insufficientFunds || isQuoteLoading}
      >
        <div class="button-content">
          {#if $swapState.isProcessing || isQuoteLoading}
            <div class="loading-spinner" />
          {/if}
          <span class="swap-button-text">{buttonText}</span>
        </div>
        <div class="button-glow" />
        <div class="shine-effect" />
        <div class="ready-glow" />
      </button>
    </div>
  </div>
</div>

{#if $swapState.tokenSelectorOpen}
  <Portal target="main">
    {#if $swapState.tokenSelectorPosition}
      {@const position = getDropdownPosition($swapState.tokenSelectorPosition)}
      <div
        class="fixed z-50 origin-left"
        style="
          left: {position.left}px;
          top: {position.top}px;
        "
      >
        <TokenSelectorDropdown
          show={true}
          onSelect={(selectedToken) => {
            // First handle the token selection
            SwapLogicService.handleSelectToken(
              $swapState.tokenSelectorOpen,
              selectedToken,
            );
            
            // Then update the URL parameter based on which panel was selected
            if (browser) {
              if ($swapState.tokenSelectorOpen === "pay") {
                updateTokenInURL("from", selectedToken.canister_id);
              } else {
                updateTokenInURL("to", selectedToken.canister_id);
              }
            }
            
            swapState.closeTokenSelector();
          }}
          onClose={() => swapState.closeTokenSelector()}
          currentToken={$swapState.tokenSelectorOpen === "pay"
            ? $swapState.payToken
            : $swapState.receiveToken}
          otherPanelToken={$swapState.tokenSelectorOpen === "pay"
            ? $swapState.receiveToken
            : $swapState.payToken}
        />
      </div>
    {/if}
  </Portal>
{/if}

{#if $swapState.showConfirmation}
  <Portal target="body">
    <SwapConfirmation
      payToken={$swapState.payToken}
      payAmount={$swapState.payAmount}
      receiveToken={$swapState.receiveToken}
      receiveAmount={$swapState.receiveAmount}
      userMaxSlippage={$settingsStore.max_slippage}
      routingPath={$swapState.routingPath}
      onConfirm={handleSwap}
      onClose={() => {
        swapState.setShowConfirmation(false);
      }}
      on:quoteUpdate={({ detail }) => {
        swapState.update((state) => ({
          ...state,
          receiveAmount: detail.receiveAmount,
        }));
      }}
    />
  </Portal>
{/if}

{#if $swapState.showSuccessModal}
<SwapSuccessModal
  show={$swapState.showSuccessModal}
  payAmount={$swapState.successDetails?.payAmount || $swapState.payAmount}
  payToken={$swapState.successDetails?.payToken || $swapState.payToken}
  receiveAmount={$swapState.successDetails?.receiveAmount || $swapState.receiveAmount}
  receiveToken={$swapState.successDetails?.receiveToken || $swapState.receiveToken}
  onClose={() => {
    swapState.setShowSuccessModal(false);
    resetSwapState();
  }}
  />
{/if}

{#if showSettings}
  <Modal 
    isOpen={true}
    title="Settings"
    height="auto"
    variant="transparent"
    on:close={() => showSettings = false}
  >
    <Settings on:close={() => showSettings = false} />
  </Modal>
{/if}

<style scoped lang="postcss">
  .swap-container {
    position: relative;
    display: flex;
    flex-direction: column;
  }

  .button-content {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .swap-button {
    @apply relative overflow-hidden;
    @apply w-full py-4 px-6 rounded-lg;
    @apply transition-all duration-200 ease-out;
    @apply disabled:opacity-50 disabled:cursor-not-allowed;
    margin-top: 4px;
    background: linear-gradient(
      135deg,
      rgba(55, 114, 255, 0.95) 0%,
      rgba(111, 66, 193, 0.95) 100%
    );
    border: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow: 0 2px 6px rgba(55, 114, 255, 0.2);
    transform: translateY(0);
    min-height: 64px;
  }

  .swap-button:hover:not(:disabled) {
    background: linear-gradient(
      135deg,
      rgba(75, 124, 255, 1) 0%,
      rgba(131, 86, 213, 1) 100%
    );
    border-color: rgba(255, 255, 255, 0.2);
    transform: translateY(-1px);
    box-shadow:
      0 4px 12px rgba(55, 114, 255, 0.3),
      0 0 0 1px rgba(255, 255, 255, 0.1);
  }

  .swap-button:active:not(:disabled) {
    transform: translateY(0);
    background: linear-gradient(
      135deg,
      rgba(45, 104, 255, 1) 0%,
      rgba(91, 46, 173, 1) 100%
    );
    box-shadow: 0 2px 4px rgba(55, 114, 255, 0.2);
    transition-duration: 0.1s;
  }

  .switch-icon {
    transition: all 0.2s ease;
    opacity: 0.9;
    width: 24px;
    height: 24px;
    color: currentColor;
  }

  .panels-container {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 8px;
  }

  .panels-wrapper {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 240px;
  }

  .panel {
    position: relative;
    z-index: 1;
  }

  /* Add subtle bounce animation for the emoji */
  @keyframes subtle-bounce {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-2px);
    }
  }

  .swap-button-text {
    @apply text-white font-semibold;
    font-size: 24px !important; /* Using !important to ensure it takes precedence */
    letter-spacing: 0.01em;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 140px;
    text-align: center;
    text-shadow: 0 1px 1px rgba(0, 0, 0, 0.1);
  }

  .swap-button.error {
    background: linear-gradient(
      135deg,
      rgba(239, 68, 68, 0.9) 0%,
      rgba(239, 68, 68, 0.8) 100%
    );
    box-shadow: none;
  }

  .swap-button.error:hover:not(:disabled) {
    background: linear-gradient(
      135deg,
      rgba(239, 68, 68, 1) 0%,
      rgba(239, 68, 68, 0.9) 100%
    );
    box-shadow: none;
  }

  .swap-button.processing {
    background: linear-gradient(135deg, #3772ff 0%, #4580ff 100%);
    cursor: wait;
    opacity: 0.8;
  }

  .button-content {
    @apply relative z-10 flex items-center justify-center gap-2;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
  }

  .loading-spinner {
    width: 22px;
    height: 22px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .button-glow {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: radial-gradient(
      circle at var(--x, 50%) var(--y, 50%),
      rgba(255, 255, 255, 0.2),
      rgba(255, 255, 255, 0) 70%
    );
    opacity: 0;
    transition: opacity 0.3s ease;
  }

  .swap-button:hover .button-glow {
    opacity: 1;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* Add a subtle pulse animation for processing state */
  @keyframes pulse {
    0% {
      opacity: 0.8;
    }
    50% {
      opacity: 0.6;
    }
    100% {
      opacity: 0.8;
    }
  }

  .swap-button.processing {
    animation: pulse 2s infinite ease-in-out;
  }

  .switch-button {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    z-index: 10;
    cursor: pointer;
    padding: 0;
    margin: 0;
    width: 44px;
    height: 44px;
    border: none;
    border-radius: 50%;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    background: #1c2333;
    color: white;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .switch-button:hover:not(.disabled) {
    background: #252b3d;
    transform: translate(-50%, -50%) scale(1.1);
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.3);
    border-color: rgba(255, 255, 255, 0.15);
  }

  .switch-button:active:not(.disabled) {
    transform: translate(-50%, -50%) scale(0.95);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
  }

  .switch-button.disabled {
    opacity: 0.5;
    cursor: not-allowed;
    background: #1c2333;
  }
  .switch-button-inner {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    transform: rotate(calc(-180deg * var(--rotation-count)));
  }

  .switch-icon {
    transition: all 0.2s ease;
    opacity: 0.9;
    width: 24px;
    height: 24px;
    color: currentColor;
  }

  .switch-button:hover:not(.disabled) .switch-icon {
    transform: scale(1.1);
    opacity: 1;
  }

  .panels-container {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 8px;
  }

  .panels-wrapper {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 240px;
  }

  .panel {
    position: relative;
    z-index: 1;
  }

  /* Add subtle bounce animation for the emoji */
  @keyframes subtle-bounce {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-2px);
    }
  }

  .swap-button-text {
    @apply text-white font-semibold;
    font-size: 24px !important; /* Using !important to ensure it takes precedence */
    letter-spacing: 0.01em;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 140px;
    text-align: center;
    text-shadow: 0 1px 1px rgba(0, 0, 0, 0.1);
  }

  .swap-button.error {
    background: linear-gradient(
      135deg,
      rgba(239, 68, 68, 0.9) 0%,
      rgba(239, 68, 68, 0.8) 100%
    );
    box-shadow: none;
  }

  .shine-effect {
    position: absolute;
    top: 0;
    left: -100%;
    width: 50%;
    height: 100%;
    background: linear-gradient(
      90deg,
      transparent,
      rgba(255, 255, 255, 0.2),
      transparent
    );
    transform: skewX(-20deg);
    pointer-events: none;
  }

  .shine-animation .shine-effect {
    animation: shine 3s infinite;
  }

  @keyframes shine {
    0%,
    100% {
      left: -100%;
    }
    35%,
    65% {
      left: 200%;
    }
  }

  .swap-button:hover:not(:disabled) {
    background: linear-gradient(
      135deg,
      rgba(75, 124, 255, 1) 0%,
      rgba(131, 86, 213, 1) 100%
    );
    border-color: rgba(255, 255, 255, 0.2);
    transform: translateY(-1px);
    box-shadow:
      0 4px 12px rgba(55, 114, 255, 0.3),
      0 0 0 1px rgba(255, 255, 255, 0.1);
  }

  .ready-glow {
    position: absolute;
    inset: -2px;
    border-radius: 18px;
    background: linear-gradient(
      135deg,
      rgba(55, 114, 255, 0.5),
      rgba(111, 66, 193, 0.5)
    );
    opacity: 0;
    filter: blur(8px);
    transition: opacity 0.3s ease;
  }

  .shine-animation .ready-glow {
    animation: pulse-glow 2s ease-in-out infinite;
  }

  @keyframes float-arrow {
    0%,
    100% {
      transform: translateX(0);
    }
    50% {
      transform: translateX(3px);
    }
  }

  @keyframes pulse-glow {
    0%,
    100% {
      opacity: 0;
      transform: scale(1);
    }
    50% {
      opacity: 0.5;
      transform: scale(1.02);
    }
  }
</style>
