<script lang="ts">
  // UI Components
  import SwapPanel from "./swap_ui/SwapPanel.svelte";
  import TokenSelectorDropdown from "./swap_ui/TokenSelectorDropdown.svelte";
  import SwapConfirmation from "./swap_ui/SwapConfirmation.svelte";
  import SwapSuccessModal from "./swap_ui/SwapSuccessModal.svelte";
  import BananaRain from "$lib/components/common/BananaRain.svelte";
  import Modal from "$lib/components/common/Modal.svelte";
  import Settings from "$lib/components/settings/Settings.svelte";
  import Portal from 'svelte-portal';
  import WalletProvider from "$lib/components/sidebar/WalletProvider.svelte";

  // Svelte imports
  import { fade } from "svelte/transition";
  import { onMount, createEventDispatcher } from "svelte";
  import { get } from "svelte/store";
  import { replaceState } from "$app/navigation";

  // Services and stores
  import { SwapLogicService } from "$lib/services/swap/SwapLogicService";
  import { swapState } from "$lib/services/swap/SwapStateService";
  import { SwapService } from "$lib/services/swap/SwapService";
  import { auth, selectedWalletId } from "$lib/services/auth";
  import {
    tokenStore,
    getTokenDecimals,
  } from "$lib/services/tokens/tokenStore";
  import { settingsStore } from "$lib/services/settings/settingsStore";
  import { toastStore } from "$lib/stores/toastStore";
  import { swapStatusStore } from "$lib/services/swap/swapStore";
  import { sidebarStore } from "$lib/stores/sidebarStore";

  // Utils
  import { getKongBackendPrincipal } from "$lib/utils/canisterIds";

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

  // Constants
  const KONG_BACKEND_PRINCIPAL = getKongBackendPrincipal();
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
  let isTransitioning = false;
  let isRotating = false;
  let rotationCount = 0;
  let isSettingsModalOpen = false;
  let isQuoteLoading = false;
  let showSuccessModal = false;
  let successDetails = null;
  let showWalletModal = false;

  // Subscribe to swap status changes
  $: {
    if (currentSwapId && $swapStatusStore[currentSwapId]) {
      const status = $swapStatusStore[currentSwapId];
      if (status.status === "Success" && status.details) {
        showSuccessModal = true;
        successDetails = {
          payAmount: status.details.payAmount,
          payToken: status.details.payToken,
          receiveAmount: status.details.receiveAmount,
          receiveToken: status.details.receiveToken,
          principalId: selectedWalletId,
        };
      }
    }
  }

  // Function to handle success modal close
  function handleSuccessModalClose() {
    showSuccessModal = false;
    successDetails = null;
    // Reset swap state
    swapState.update((state) => ({
      ...state,
      payAmount: "",
      receiveAmount: "",
      error: null,
      isProcessing: false,
    }));
  }

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

  // Event dispatcher
  const dispatch = createEventDispatcher<{
    modeChange: { mode: "normal" | "pro" };
    tokenChange: { fromToken: FE.Token | null; toToken: FE.Token | null };
  }>();

  // Settings modal functions
  function toggleSettingsModal() {
    isSettingsModalOpen = !isSettingsModalOpen;
  }

  // Reactive statements
  $: userMaxSlippage = $settingsStore.max_slippage;

  $: insufficientFunds =
    $swapState.payToken &&
    $swapState.payAmount &&
    Number($swapState.payAmount) >
      Number(getTokenBalance($swapState.payToken.canister_id));

  $: buttonText = getButtonText(
    showSuccessModal,
    $swapState.isProcessing,
    $swapState.error,
    insufficientFunds,
    $swapState.swapSlippage > userMaxSlippage,
    $auth?.account?.owner,
    $swapState.payAmount,
  );

  // Initialize tokens when they become available
  $: if ($tokenStore.tokens.length > 0 && !isInitialized) {
    isInitialized = true;
    swapState.initializeTokens(initialFromToken, initialToToken);
  }

  // Initialize on mount
  onMount(() => {
    initializeComponent();
    setupEventListeners();
    resetSwapState();

    return () => {
      window.removeEventListener("swapSuccess", handleSwapSuccess);
    };
  });

  // Helper functions
  function getTokenBalance(tokenId: string): string {
    if (!tokenId) return "0";
    const balance = $tokenStore.balances[tokenId]?.in_tokens ?? BigInt(0);
    const token = $tokenStore.tokens.find((t) => t.canister_id === tokenId);
    return token
      ? (Number(balance) / Math.pow(10, token.decimals)).toString()
      : "0";
  }

  function getButtonText(
    showSuccessModal: boolean,
    isProcessing: boolean,
    error: string | null,
    insufficientFunds: boolean,
    highSlippage: boolean,
    isWalletConnected: boolean | undefined,
    payAmount: string | null,
  ): string {
    if (showSuccessModal) return "Swap";
    if (isProcessing) return "Processing...";
    if (error) return error;
    if (insufficientFunds) return "Insufficient Funds";
    if (highSlippage)
      return `High Slippage (${$swapState.swapSlippage.toFixed(2)}% > ${userMaxSlippage}%) - Click to Adjust`;
    if (!isWalletConnected) return "Click to Connect Wallet";
    if (!payAmount) return "Enter Amount";
    return "SWAP";
  }

  function getButtonTooltip(
    owner: boolean | undefined,
    slippageTooHigh: boolean,
    error: string | null,
  ): string {
    if (!owner) return "Connect to trade";
    if (insufficientFunds) {
      const balance = getTokenBalance($swapState.payToken?.canister_id);
      return `Balance: ${balance} ${$swapState.payToken?.symbol}`;
    }
    if (slippageTooHigh)
      return `Slippage: ${$swapState.swapSlippage}% > ${userMaxSlippage}%`;
    if (error) return error;
    return "Execute swap";
  }

  // Event handlers
  function handleModeChange(mode: "normal" | "pro"): void {
    if (mode === currentMode || isTransitioning) return;
    isTransitioning = true;
    previousMode = currentMode;
    setTimeout(() => {
      dispatch("modeChange", { mode });
      setTimeout(() => {
        isTransitioning = false;
      }, 300);
    }, 150);
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
        userMaxSlippage,
        backendPrincipal: KONG_BACKEND_PRINCIPAL,
        lpFees: $swapState.lpFees,
      });

      return typeof result === "bigint";
    } catch (error) {
      console.error("Swap execution failed:", error);
      swapState.update((state) => ({
        ...state,
        isProcessing: false,
        error: error.message || "Swap failed",
      }));
      return false;
    }
  }

  async function handleButtonAction(): Promise<void> {
    if (!$auth.isConnected) {
      showWalletModal = true;
      return;
    }

    if ($swapState.swapSlippage > userMaxSlippage) {
      toggleSettingsModal();
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
    const tokens = get(tokenStore);
    if (!tokens.tokens.length) {
      await tokenStore.loadTokens();
    }
  }

  function setupEventListeners(): void {
    window.addEventListener("swapSuccess", handleSwapSuccess);
  }

  function handleSwapSuccess(event: CustomEvent): void {
    SwapLogicService.handleSwapSuccess(event);
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

  function handleTokenSelect(panelType: PanelType) {
    if (panelType === "pay") {
      swapState.update((s) => ({ ...s, showPayTokenSelector: true }));
    } else {
      swapState.update((s) => ({ ...s, showReceiveTokenSelector: true }));
    }
  }

  async function handleReverseTokens() {
    if ($swapState.isProcessing) return;

    isRotating = true;
    rotationCount++;

    const tempPayToken = $swapState.payToken;
    const tempPayAmount = $swapState.payAmount;
    const tempReceiveAmount = $swapState.receiveAmount;

    // Update tokens
    swapState.setPayToken($swapState.receiveToken);
    swapState.setReceiveToken(tempPayToken);

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

    setTimeout(() => {
      isRotating = false;
    }, 300);
  }

  function updateTokenInURL(param: "from" | "to", tokenId: string) {
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

    // Debounce the quote update
    quoteUpdateTimeout = setTimeout(async () => {
      try {
        isQuoteLoading = true;

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

  // Add reactive variables for token changes
  $: fromToken = $swapState.payToken;
  $: toToken = $swapState.receiveToken;

  // Watch for token changes and dispatch event
  $: {
    if ($swapState.payToken || $swapState.receiveToken) {
      dispatch("tokenChange", {
        fromToken: $swapState.payToken,
        toToken: $swapState.receiveToken,
      });
    }
  }

  // Handle wallet connection success
  function handleWalletLogin() {
    showWalletModal = false;
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
    swapState.update(state => ({
      ...state,
      payAmount: '',
      receiveAmount: '',
      error: null,
      isProcessing: false,
      showConfirmation: false,
      showBananaRain: false,
      swapSlippage: 0,  // Reset slippage
      lpFees: null,     // Reset fees
      routingPath: null // Reset routing
    }));

    // Reset previous values to prevent unnecessary quote updates
    previousPayAmount = '';
    previousPayToken = null;
    previousReceiveToken = null;
  }
</script>

<!-- Template content -->
<div class="swap-container">
  <div class="swap-wrapper">
    <div class="swap-container" in:fade={{ duration: 420 }}>
      <div class="mode-selector">
        <div
          class="mode-selector-background"
          style="transform: translateX({currentMode === 'pro' ? '100%' : '0'})"
        ></div>
        <button
          class="mode-button"
          class:selected={currentMode === "normal"}
          class:transitioning={isTransitioning && previousMode === "pro"}
          on:click={() => handleModeChange("normal")}
        >
          <span class="mode-text">Normal</span>
        </button>
        <button
          class="mode-button"
          class:selected={currentMode === "pro"}
          class:transitioning={isTransitioning && previousMode === "normal"}
          on:click={() => handleModeChange("pro")}
        >
          <span class="mode-text">Pro</span>
        </button>
      </div>

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
            class:rotating={isRotating}
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
            />
          </div>
        </div>

        <div class="swap-footer">
          <button
            class="swap-button"
            class:error={$swapState.error ||
              $swapState.swapSlippage > userMaxSlippage ||
              insufficientFunds}
            class:processing={$swapState.isProcessing}
            class:ready={!$swapState.error &&
              $swapState.swapSlippage <= userMaxSlippage &&
              !insufficientFunds}
            class:shine-animation={buttonText === "SWAP"}
            on:click={handleButtonAction}
            title={getButtonTooltip(
              $auth?.account?.owner,
              $swapState.swapSlippage > userMaxSlippage,
              $swapState.error,
            )}
          >
            <div class="button-content">
              {#if $swapState.isProcessing}
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
  </div>
</div>

{#if $swapState.tokenSelectorOpen}
  <Portal target="body">
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
            SwapLogicService.handleSelectToken(
              $swapState.tokenSelectorOpen,
              selectedToken,
            );
            swapState.closeTokenSelector();
          }}
          onClose={() => swapState.closeTokenSelector()}
          currentToken={$swapState.tokenSelectorOpen === "pay"
            ? $swapState.payToken
            : $swapState.receiveToken}
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
      gasFees={$swapState.gasFees}
      lpFees={$swapState.lpFees}
      {userMaxSlippage}
      routingPath={$swapState.routingPath}
      onConfirm={handleSwap}
      onClose={() => {
        swapState.setShowConfirmation(false);
      }}
      on:quoteUpdate={({ detail }) => {
        swapState.update(state => ({
          ...state,
          receiveAmount: detail.receiveAmount
        }));
      }}
    />
  </Portal>
{/if}

{#if $swapState.showBananaRain}
  <BananaRain />
{/if}

<SwapSuccessModal
  show={showSuccessModal}
  {...successDetails}
  onClose={handleSuccessModalClose}
/>

{#if isSettingsModalOpen}
  <Portal target="body">
    <Modal
      isOpen={isSettingsModalOpen}
      onClose={() => (isSettingsModalOpen = false)}
      title="Slippage Settings"
    >
      <Settings />
    </Modal>
  </Portal>
{/if}

{#if showWalletModal}
  <Portal target="body">
    <Modal
      isOpen={showWalletModal}
      onClose={() => (showWalletModal = false)}
      title="Connect Wallet"
    >
      <WalletProvider on:login={handleWalletLogin} />
    </Modal>
  </Portal>
{/if}

<style lang="postcss">
  .swap-container {
    position: relative;
    display: flex;
    flex-direction: column;
  }

  .mode-selector {
    position: relative;
    display: flex;
    gap: 1px;
    margin-bottom: 12px;
    padding: 8px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
  }

  .mode-selector-background {
    position: absolute;
    top: 2px;
    left: 2px;
    width: calc(50% - 1px);
    height: calc(100% - 4px);
    background: linear-gradient(
      135deg,
      rgba(55, 114, 255, 0.15),
      rgba(55, 114, 255, 0.2)
    );
    border-radius: 12px;
    transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    z-index: 0;
  }

  .mode-button {
    position: relative;
    z-index: 1;
    flex: 1;
    padding: 6px 12px;
    border: none;
    border-radius: 16px;
    font-size: 0.875rem;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.7);
    background: transparent;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .mode-text {
    @apply text-lg;
    position: relative;
    z-index: 2;
    transition:
      transform 0.2s ease,
      color 0.2s ease;
  }

  .mode-button:hover:not(.selected) .mode-text {
    color: rgba(255, 255, 255, 0.9);
  }

  .mode-button.selected .mode-text {
    color: rgba(255, 255, 255, 1);
    font-weight: 600;
  }

  .mode-button.transitioning .mode-text {
    transform: scale(0.95);
  }

  .button-content {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .swap-button {
    @apply relative overflow-hidden;
    @apply w-full py-4 px-6;
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
    border-radius: 16px;
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

  .swap-button.disabled {
    opacity: 0.5;
    cursor: not-allowed;
    background: #1c2333;
  }

  .swap-button-inner {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.4s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .swap-button.rotating .swap-button-inner {
    transform: rotate(180deg);
  }

  .switch-icon {
    transition: all 0.2s ease;
    opacity: 0.9;
    width: 24px;
    height: 24px;
    color: currentColor;
  }

  .swap-button:hover:not(.disabled) .switch-icon {
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

  /* Add these new styles for enhanced button text */
  .button-text {
    @apply text-white font-semibold text-lg;
    letter-spacing: 0.01em;
    display: flex;
    align-items: center;
    gap: 6px;

    /* Add subtle text shadow for better contrast */
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
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

  .button-text :first-child {
    animation: subtle-bounce 2s infinite ease-in-out;
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

  .button-text {
    @apply text-white font-semibold text-lg;
    font-size: 1.125rem;
    letter-spacing: 0.01em;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 140px;
    text-align: center;
    text-shadow: 0 1px 1px rgba(0, 0, 0, 0.1);
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
    transition: transform 0.4s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .switch-button.rotating .switch-button-inner {
    transform: rotate(180deg);
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

  .button-text {
    @apply text-white font-semibold text-lg;
    letter-spacing: 0.01em;
    display: flex;
    align-items: center;
    gap: 6px;

    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
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
