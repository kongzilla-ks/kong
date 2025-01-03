<script lang="ts">
    import { onDestroy } from 'svelte';
    import { formatTokenAmount, parseTokenAmount, formatToNonZeroDecimal } from "$lib/utils/numberFormatUtils";
    import { poolStore } from "$lib/services/pools/poolStore";
    import Portal from 'svelte-portal';
    import TokenSelectorDropdown from '$lib/components/swap/swap_ui/TokenSelectorDropdown.svelte';
    import { PoolService } from '$lib/services/pools/PoolService';
    import Panel from '$lib/components/common/Panel.svelte';
    import AddLiquidityConfirmation from './AddLiquidityConfirmation.svelte';
    import { tweened } from "svelte/motion";
    import { cubicOut } from "svelte/easing";
    import { auth } from '$lib/services/auth';
    import { tokenStore } from '$lib/services/tokens/tokenStore';
    import debounce from 'lodash-es/debounce';
    import { toastStore } from "$lib/stores/toastStore";
    import { BigNumber } from 'bignumber.js';
    import { CKUSDT_CANISTER_ID, ICP_CANISTER_ID } from '$lib/constants/canisterConstants';
    import { validateTokenCombination, formatDisplayValue, isValidNumber, calculateMaxAmount } from '$lib/utils/liquidityUtils';
    import { 
        formatWithCommas,
        hasInsufficientBalance as checkInsufficientBalance,
        getButtonText,
        calculatePoolRatio,
        calculateUsdRatio,
        formatLargeNumber
    } from '$lib/utils/liquidityUtils';

    export let token0: FE.Token | null = null;
    export let token1: FE.Token | null = null;
    export let amount0: string = null;
    export let amount1: string = null;
    export let loading: boolean = false;
    export let error: string | null = null;
    export let token0Balance: string;
    export let token1Balance: string;
    export let onTokenSelect: (index: 0 | 1) => void;
    export let onInput: (index: 0 | 1, value: string) => void;
    export let onSubmit: () => Promise<void>;
    export let pool: BE.Pool | null = null;
    export let showConfirmation = false;

    let showToken0Selector = false;
    let showToken1Selector = false;
    let loadingState = '';

    const ALLOWED_TOKEN_SYMBOLS = ['ICP', 'ckUSDT'];
    const DEFAULT_TOKEN = 'ICP';
    const SECONDARY_TOKEN_IDS = [
        ICP_CANISTER_ID,
        CKUSDT_CANISTER_ID
    ];

    let userIsTyping = false;
    let typingTimeout: ReturnType<typeof setTimeout> | undefined;

    let finalLoading = false;
    let finalLoadingState = "";
    $: {
        if (userIsTyping) {
            finalLoading = false;
            finalLoadingState = "";
        } else {
            finalLoading = loading;
            finalLoadingState = loadingState;
        }
    }

    $: token0Balance = $tokenStore.balances[token0?.canister_id]?.in_tokens?.toString() || "0";
    $: token1Balance = $tokenStore.balances[token1?.canister_id]?.in_tokens?.toString() || "0";

    function handleTokenSelect(index: 0 | 1, token: FE.Token) {
        const otherToken = index === 0 ? token1 : token0;
        
        if (otherToken && !validateTokenCombination(
            index === 0 ? token.symbol : token0.symbol,
            index === 0 ? token1.symbol : token.symbol,
            ALLOWED_TOKEN_SYMBOLS
        )) {
            toastStore.error(
                'Token 2 must be ICP or ckUSDT',
                undefined,
                'Invalid Token Pair'
            );
            // Reset to default token
            const defaultToken = $tokenStore.tokens.find(t => t.symbol === DEFAULT_TOKEN);
            if (defaultToken) {
                if (index === 0) {
                    token0 = defaultToken;
                } else {
                    token1 = defaultToken;
                }
            }
            return;
        }

        if (index === 0) {
            token0 = token;
            showToken0Selector = false;
        } else {
            token1 = token;
            showToken1Selector = false;
        }
        onTokenSelect(index);
    }

    // Add reactive statement to handle token changes
    $: {
        if (token0 && token1 && (amount0 !== "0" || amount1 !== "0")) {
            // Only trigger calculation if the amount being watched isn't from user input
            const nonZeroAmount = amount0 !== "0" ? amount0 : amount1;
            const index = amount0 !== "0" ? 0 : 1;
            const currentToken = index === 0 ? token0 : token1;
            const otherToken = index === 0 ? token1 : token0;
            
            // Add this check to prevent recalculation when user is inputting
            if (!input0Element?.matches(':focus') && !input1Element?.matches(':focus')) {
                if (nonZeroAmount && currentToken && otherToken) {
                    debouncedHandleInput(index, nonZeroAmount, currentToken, otherToken);
                }
            }
        }
    }

    // Constants for formatting and animations
    const DEFAULT_DECIMALS = 8;
    const ANIMATION_BASE_DURATION = 200;

    // Input state management
    let input0Element: HTMLInputElement | null = null;
    let input1Element: HTMLInputElement | null = null;
    let input0Focused = false;
    let input1Focused = false;

    // Animated values for smooth transitions
    const animatedUsdValue0 = tweened(0, {
        duration: ANIMATION_BASE_DURATION,
        easing: cubicOut,
    });

    const animatedUsdValue1 = tweened(0, {
        duration: ANIMATION_BASE_DURATION,
        easing: cubicOut,
    });

    // Create debounced version of the input handler with shorter delay
    const debouncedHandleInput = debounce(async (index: 0 | 1, value: string, currentToken: FE.Token, otherToken: FE.Token) => {
        try {
            // loading = true;
            // loadingState = `Calculating required ${otherToken.symbol} amount...`;
            
            // Clean the value by removing underscores before parsing
            const cleanValue = value.replace(/_/g, '');
            let inputAmount = parseTokenAmount(cleanValue, currentToken.decimals);

            const requiredAmount = await PoolService.addLiquidityAmounts(
                index === 0 ? token0.symbol : token1.symbol,
                inputAmount,
                index === 0 ? token1.symbol : token0.symbol,
            );

            if (!requiredAmount.Ok) {
                throw new Error("Failed to calculate required amount");
            }

            // Only update the non-active input
            if (index === 0) {
                // User modified top input, only update bottom input
                amount1 = formatTokenAmount(requiredAmount.Ok.amount_1, token1.decimals).toString();
                if (input1Element) input1Element.value = amount1;
            } else {
                // User modified bottom input, only update top input
                amount0 = formatTokenAmount(requiredAmount.Ok.amount_0, token0.decimals).toString();
                if (input0Element) input0Element.value = amount0;
            }

            // Call the parent's onInput handler with the user's input value
            onInput(index, value);
        } catch (err) {
            console.error("Error in debouncedHandleInput:", err);
            error = err.message;
        } finally {
            // loading = false;
            // loadingState = '';
        }
    }, 500);

    // Enhanced input handling
    async function handleInput(index: 0 | 1, event: Event) {
        if (!poolExists) {
            event.preventDefault();
            return;
        }
        const inputElement = event.target as HTMLInputElement;
        let value = inputElement.value.replace(/[,_]/g, ''); // Remove commas and underscores
        
        if (!isValidNumber(value)) {
            inputElement.value = index === 0 ? amount0 : amount1;
            return;
        }

        // Handle decimal point
        if (value.includes('.')) {
            const [whole, decimal] = value.split('.');
            const currentToken = index === 0 ? token0 : token1;
            const maxDecimals = currentToken?.decimals || DEFAULT_DECIMALS;
            value = `${whole}.${decimal.slice(0, maxDecimals)}`;
        }

        // Remove leading zeros unless it's "0." or just "0"
        if (value.length > 1 && value.startsWith('0') && value[1] !== '.') {
            value = value.replace(/^0+/, '');
        }

        // If empty or invalid after processing, set to "0"
        if (!value || value === '.') {
            value = "0";
        }

        const currentToken = index === 0 ? token0 : token1;
        const otherToken = index === 0 ? token1 : token0;

        if (!currentToken || !otherToken) {
            error = "Please select both tokens.";
            return;
        }

        // Update the input value
        inputElement.value = value;

        // Call debounced handler for API request
        debouncedHandleInput(index, value, currentToken, otherToken);
    }

    // Remove the blur handler as we don't need formatting anymore
    function handleBlur(index: 0 | 1) {
        if (index === 0) {
            input0Focused = false;
        } else {
            input1Focused = false;
        }
    }

    // Update max button handler
    async function handleMaxClick(index: 0 | 1) {
        if (!poolExists) return;
        const currentToken = index === 0 ? token0 : token1;
        const currentBalance = index === 0 ? token0Balance : token1Balance;
        const otherToken = index === 0 ? token1 : token0;
        
        if (!currentToken || !otherToken) return;

        try {
            const feeMultiplier = currentToken.icrc2 ? 2 : 1;
            const value = await calculateMaxAmount(currentToken, currentBalance, feeMultiplier);
            
            // Update the input display
            if (index === 0) {
                if (input0Element) {
                    input0Element.value = value;
                }
                amount0 = value;
            } else {
                if (input1Element) {
                    input1Element.value = value;
                }
                amount1 = value;
            }

            // Call the debounced handler to calculate the other amount
            await debouncedHandleInput(index, value, currentToken, otherToken);
        } catch (err) {
            console.error("Error in handleMaxClick:", err);
            error = err.message;
            toastStore.error(err.message);
        }
    }

    // Reactive declarations for USD values
    $: {
        if (token0?.metrics.price && amount0) {
            const cleanAmount = amount0.toString().replace(/[,_]/g, '');
            const value = new BigNumber(cleanAmount)
            .times(new BigNumber(token0.metrics.price))
            .toNumber();
            animatedUsdValue0.set(value);
        }
        if (token1?.metrics.price && amount1) {
            const cleanAmount = amount1.toString().replace(/[,_]/g, '');
            const value = new BigNumber(cleanAmount)
            .times(new BigNumber(token1.metrics.price))
            .toNumber();
            animatedUsdValue1.set(value);
        }
    }

    // Calculate and display pool ratio
    $: poolRatio = calculatePoolRatio(token0, token1, amount0, amount1);

    // Calculate USD ratio to show price relationship
    $: usdRatio = calculateUsdRatio(token0, token1);

    // Get pool when both tokens are selected
    $: if (token0 && token1) {
        tokenStore.loadBalancesForTokens([token0, token1], auth.pnp.account?.owner?.toString());
        pool = $poolStore.pools.find(p => 
            (p.address_0 === token0.canister_id && p.address_1 === token1.canister_id) ||
            (p.address_0 === token1.canister_id && p.address_1 === token0.canister_id)
        ) || null;
    } else {
        pool = null;
    }

    function openTokenSelector(index: 0 | 1) {
        if (index === 0) {
            showToken0Selector = true;
            showToken1Selector = false;
        } else {
            showToken1Selector = true;
            showToken0Selector = false;
        }
    }

    $: hasInsufficientBalance = () => checkInsufficientBalance(
        amount0,
        amount1,
        token0,
        token1
    );

    $: buttonText = userIsTyping
        ? "Entering Amounts..."
        : getButtonText(
            token0,
            token1,
            poolExists,
            hasInsufficientBalance(),
            amount0,
            amount1,
            finalLoading,
            finalLoadingState
        );

    $: isValid = token0 && token1 && 
        parseFloat(amount0.replace(/[,_]/g, '')) > 0 && 
        parseFloat(amount1.replace(/[,_]/g, '')) > 0 && 
        !error && !hasInsufficientBalance() && 
        pool !== null;

    async function handleSubmit() {
        if (!isValid || loading) return;
        showConfirmation = true;
    }

    // Cleanup debounced function
    onDestroy(() => {
        debouncedHandleInput.cancel();
    });

    // Add this computed property to check if pool exists
    $: poolExists = pool !== null;

    // Change the initial display values to empty strings
    let displayValue0 = "";
    let displayValue1 = "";

    function handleFormattedInput(index: 0 | 1, event: Event) {
        const input = event.target as HTMLInputElement;
        const cursorPosition = input.selectionStart || 0;
        
        // Get the raw value and clean it
        let rawValue = input.value.replace(/,/g, '');
        if (rawValue === '') {
            return;
        }
        
        if (!isValidNumber(rawValue)) {
            input.value = index === 0 ? displayValue0 : displayValue1;
            return;
        }

        // Handle empty values
        if (!rawValue || rawValue === '.') {
            if (index === 0) {
                amount0 = "0";
                displayValue0 = "";
            } else {
                amount1 = "0";
                displayValue1 = "";
            }
            input.value = "";
            
            const newEvent = new Event('input', {
                bubbles: true,
                cancelable: true
            });
            Object.defineProperty(newEvent, 'target', { value: input });
            handleInput(index, newEvent);
            return;
        }

        // Update the actual amounts and display values
        const formattedValue = formatWithCommas(rawValue);
        
        // Count actual commas before cursor in formatted value
        const beforeCursor = formattedValue.slice(0, cursorPosition);
        const commasBeforeCursor = (beforeCursor.match(/,/g) || []).length;
        const commasInOriginal = (input.value.slice(0, cursorPosition).match(/,/g) || []).length;
        const commaDiff = commasBeforeCursor - commasInOriginal;

        if (index === 0) {
            amount0 = rawValue;
            displayValue0 = formattedValue;
        } else {
            amount1 = rawValue;
            displayValue1 = formattedValue;
        }
        input.value = formattedValue;

        // Create a new event with the raw value
        const newEvent = new Event('input', {
            bubbles: true,
            cancelable: true
        });
        Object.defineProperty(newEvent, 'target', { value: input });

        // Handle the main input logic with raw value
        handleInput(index, newEvent);

        // Set new cursor position accounting for commas
        const newPosition = cursorPosition + commaDiff;
        requestAnimationFrame(() => {
            input.setSelectionRange(newPosition, newPosition);
        });

        // Mark user as typing and start/reset a brief timer
        userIsTyping = true;
        if (typingTimeout) clearTimeout(typingTimeout);
        typingTimeout = setTimeout(() => {
            userIsTyping = false;
        }, 300);
    }
</script>

<Panel variant="green" width="auto" className="liquidity-panel w-full max-w-[690px]">
    <div class="flex flex-col min-h-[165px] box-border relative rounded-lg">
        <header>
            <div class="flex items-center justify-between gap-4 min-h-[2.5rem] mb-5">
                <h2 class="text-[clamp(1.5rem,4vw,2rem)] font-semibold text-white m-0 tracking-tight leading-none">
                    Add Liquidity
                </h2>
            </div>
        </header>

        <div class="relative">
            <div class="token-input-container">
                <div class="relative flex-grow mb-2">
                    <div class="flex items-center gap-4">
                        <div class="relative flex-1">
                            <input
                                bind:this={input0Element}
                                type="text"
                                inputmode="decimal"
                                pattern="[0-9]*"
                                placeholder="0"
                                class="amount-input {!poolExists ? 'cursor-not-allowed' : ''}"
                                value={displayValue0}
                                on:input={(e) => handleFormattedInput(0, e)}
                                on:focus={() => (input0Focused = true)}
                                on:blur={() => handleBlur(0)}
                                disabled={!poolExists}
                            />
                        </div>
                        <div class="token-selector-wrapper">
                            <button 
                                class="token-selector-button" 
                                on:click={() => openTokenSelector(0)}
                            >
                                {#if token0}
                                    <div class="token-info">
                                        <img src={token0.logo_url} alt={token0.symbol} class="token-logo" />
                                        <span class="token-symbol">{token0.symbol}</span>
                                    </div>
                                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="chevron">
                                        <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clip-rule="evenodd" />
                                    </svg>
                                {:else}
                                    <span class="select-token-text">Select Token</span>
                                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="chevron">
                                        <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clip-rule="evenodd" />
                                    </svg>
                                {/if}
                            </button>
                        </div>
                    </div>
                </div>
                <div class="balance-info">
                    <div class="flex items-center gap-2">
                        <span class="text-white/50 font-normal tracking-wide">Value</span>
                        <span class="pl-1 text-white/50 font-medium tracking-wide">
                            ${formatToNonZeroDecimal($animatedUsdValue0)}
                        </span>
                    </div>
                    <button 
                        class="available-balance"
                        on:click={() => handleMaxClick(0)}
                    >
                        Available: {token0 ? formatTokenAmount(token0Balance, token0.decimals) : '0.00'} {token0?.symbol || ''}
                    </button>
                </div>
            </div>

            <div class="token-input-container mt-12">
                <div class="relative flex-grow mb-2">
                    <div class="flex items-center gap-4">
                        <div class="relative flex-1">
                            <input
                                bind:this={input1Element}
                                type="text"
                                inputmode="decimal"
                                pattern="[0-9]*"
                                placeholder="0"
                                class="amount-input {!poolExists ? 'cursor-not-allowed' : ''}"
                                value={displayValue1}
                                on:input={(e) => handleFormattedInput(1, e)}
                                on:focus={() => (input1Focused = true)}
                                on:blur={() => handleBlur(1)}
                                disabled={!poolExists}
                            />
                        </div>
                        <div class="token-selector-wrapper">
                            <button 
                                class="token-selector-button" 
                                on:click={() => openTokenSelector(1)}
                            >
                                {#if token1}
                                    <div class="token-info">
                                        <img src={token1.logo_url} alt={token1.symbol} class="token-logo" />
                                        <span class="token-symbol">{token1.symbol}</span>
                                    </div>
                                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="chevron">
                                        <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clip-rule="evenodd" />
                                    </svg>
                                {:else}
                                    <span class="select-token-text">Select Token</span>
                                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="chevron">
                                        <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clip-rule="evenodd" />
                                    </svg>
                                {/if}
                            </button>
                        </div>
                    </div>
                </div>
                <div class="balance-info">
                    <div class="flex items-center gap-2">
                        <span class="text-white/50 font-normal tracking-wide">Value</span>
                        <span class="pl-1 text-white/50 font-medium tracking-wide">
                            ${formatToNonZeroDecimal($animatedUsdValue1)}
                        </span>
                    </div>
                    <button 
                        class="available-balance"
                        on:click={() => handleMaxClick(1)}
                    >
                        Available: {token1 ? formatTokenAmount(token1Balance, token1.decimals) : '0.00'} {token1?.symbol || ''}
                    </button>
                </div>
            </div>
        </div>

        <div class="mt-4">
            <button
                class="submit-button"
                disabled={!isValid || loading}
                on:click={handleSubmit}
            >
                {#if loading}
                    <div class="loading-state">
                        <span class="loading-spinner"></span>
                        <span>{loadingState}</span>
                    </div>
                {:else}
                    {buttonText}
                {/if}
            </button>
        </div>

        {#if token0 && token1}
            {#if pool}
                <div class="pool-info mt-4">
                    <div class="pool-stats-grid">
                        <div class="pool-stat">
                            <span class="stat-value">${formatLargeNumber(pool.tvl)}</span>
                            <span class="stat-label">TVL</span>
                        </div>
                        <div class="pool-stat">
                            <span class="stat-value">${formatLargeNumber(pool.rolling_24h_volume)}</span>
                            <span class="stat-label">24h Vol</span>
                        </div>
                        <div class="pool-stat">
                            <span class="stat-value">{formatToNonZeroDecimal(pool.rolling_24h_apy)}%</span>
                            <span class="stat-label">APY</span>
                        </div>
                    </div>
                    {#if poolRatio}
                    <div class="flex flex-col gap-1 mt-4 text-sm text-gray-500">
                        <div class="flex items-center justify-between">
                            <span>Pool Ratio:</span>
                            <span class="font-medium">{poolRatio}</span>
                        </div>
                        <div class="flex items-center justify-between">
                            <span>Price:</span>
                            <span class="font-medium">{usdRatio}</span>
                        </div>
                    </div>
                {/if}
                </div>
            {:else}
                <div class="pool-info mt-4">
                    <div class="no-pool-message">
                        <div class="flex flex-col items-center gap-2 py-3">
                            <span class="text-yellow-500 font-medium">This pool doesn't exist yet</span>
                            <span class="text-white/60 text-sm text-center">
                                You cannot add liquidity until the pool is created
                            </span>
                        </div>
                    </div>
                </div>
            {/if}
        {/if}
    </div>
</Panel>

<!-- Token Selectors -->
{#if showToken0Selector}
    <Portal target="body">
        <TokenSelectorDropdown
            show={true}
            currentToken={token0}
            otherPanelToken={token1}
            onSelect={(token) => handleTokenSelect(0, token)}
            onClose={() => showToken0Selector = false}
        />
    </Portal>
{/if}

{#if showToken1Selector}
    <Portal target="body">
        <TokenSelectorDropdown
            show={true}
            currentToken={token1}
            otherPanelToken={token0}
            onSelect={(token) => handleTokenSelect(1, token)}
            onClose={() => showToken1Selector = false}
            allowedCanisterIds={SECONDARY_TOKEN_IDS}
        />
    </Portal>
{/if}

{#if showConfirmation}
    <Portal target="body">
        <AddLiquidityConfirmation
            {token0}
            {token1}
            {amount0}
            {amount1}
            {pool}
            isOpen={showConfirmation}
            onClose={() => {
                showConfirmation = false;
                // Reset form state on close
                amount0 = "0";
                amount1 = "0";
                if (input0Element) input0Element.value = "0";
                if (input1Element) input1Element.value = "0";
            }}
            onConfirm={async () => {
                showConfirmation = false;
                await onSubmit();
            }}
        />
    </Portal>
{/if}

<style lang="postcss">
    .token-selector-wrapper {
        @apply min-w-[180px];
    }

    .token-selector-button {
        @apply w-full flex items-center justify-between;
        @apply bg-white/5 hover:bg-white/10;
        @apply rounded-xl px-4 py-3;
        @apply border border-white/10;
        @apply transition-colors duration-150;
    }

    .token-info {
        @apply flex items-center gap-2 min-w-[140px];
    }

    .token-logo {
        @apply w-8 h-8 rounded-full bg-white/5 object-contain;
    }

    .token-symbol {
        @apply text-[15px] text-white font-medium min-w-[80px];
    }

    .select-token-text {
        @apply text-[15px] text-white/70 min-w-[120px] text-left;
    }

    .amount-input {
        @apply flex-1 min-w-0 bg-transparent border-none;
        @apply text-white text-[2.5rem] font-medium tracking-tight;
        @apply w-full relative z-10 p-0;
        @apply opacity-85 focus:outline-none focus:text-white;
        @apply disabled:text-white/65 placeholder:text-white/65;
    }

    .balance-info {
        @apply flex justify-between mt-2;
        @apply text-[clamp(0.8rem,2vw,0.875rem)] text-white/50;
    }

    .submit-button {
        @apply w-full px-6 py-3 rounded-xl;
        @apply bg-blue-600 text-white font-medium;
        @apply disabled:opacity-50 disabled:cursor-not-allowed;
        @apply hover:bg-blue-700 transition-colors duration-200;
        @apply mt-4;
    }

    .loading-state {
        @apply flex items-center justify-center gap-2;
    }

    .loading-spinner {
        @apply w-4 h-4 border-2 border-white/20 border-t-white rounded-full animate-spin;
    }

    .chevron {
        @apply w-5 h-5 text-white/50;
    }

    @media (max-width: 420px) {
        .amount-input {
            @apply text-2xl mt-[-0.15rem];
        }
    }

    .pool-info {
        @apply border-t border-white/10 pt-4;
    }

    .pool-stats-grid {
        @apply grid grid-cols-3 gap-2 bg-white/5 rounded-lg p-3;
    }

    .pool-stat {
        @apply flex flex-col items-center;
    }

    .stat-value {
        @apply text-base font-medium text-white;
    }

    .stat-label {
        @apply text-xs text-white/60;
    }

    @media (min-width: 640px) {
        .stat-value {
            @apply text-lg;
        }
        
        .stat-label {
            @apply text-sm;
        }
    }

    .no-pool-message {
        @apply flex items-center justify-center p-4 rounded-lg;
        @apply bg-white/5 border border-yellow-500/20;
    }

    .available-balance {
        @apply text-white/70 hover:text-yellow-500 transition-colors duration-150;
    }

    .amount-input:disabled {
        @apply opacity-50 cursor-not-allowed;
    }
</style>
