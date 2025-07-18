<script lang="ts">
  import Modal from "$lib/components/common/Modal.svelte";
  import ButtonV2 from "$lib/components/common/ButtonV2.svelte";
  import { liquidityStore } from "$lib/stores/liquidityStore";
  import {
    formatToNonZeroDecimal,
    parseTokenAmount,
  } from "$lib/utils/numberFormatUtils";
  import { onDestroy, createEventDispatcher } from "svelte";
  import { createPool, addLiquidity, pollRequestStatus } from "$lib/api/pools";
  import { toastStore } from "$lib/stores/toastStore";
  import { loadBalance } from "$lib/stores/balancesStore";
  import { currentUserPoolsStore } from "$lib/stores/currentUserPoolsStore";
  import { CrossChainSwapService } from "$lib/services/swap/CrossChainSwapService";
  import { IcrcService } from "$lib/services/icrc/IcrcService";
  import { solanaLiquidityModalStore } from "$lib/stores/solanaLiquidityModal";
  import { auth, swapActor } from "$lib/stores/auth";

  const dispatch = createEventDispatcher();

  export let isCreatingPool: boolean = false;
  export let show: boolean;
  export let onClose: () => void;
  export let modalKey: string = `confirm-liquidity-${Date.now()}`;
  export let target: string = "#portal-target";

  // Get values directly from the store
  $: token0 = $liquidityStore.token0;
  $: token1 = $liquidityStore.token1;
  $: amount0 = $liquidityStore.amount0;
  $: amount1 = $liquidityStore.amount1;

  let isLoading = false;
  let error: string | null = null;
  let mounted = true;
  let pollingController: AbortController | null = null;

  onDestroy(() => {
    mounted = false;
    // Cancel any ongoing polling
    if (pollingController) {
      pollingController.abort();
      pollingController = null;
    }
  });

  // Calculated values
  $: token0Value = token0?.metrics?.price
    ? (Number(amount0) * Number(token0.metrics.price)).toFixed(2)
    : "0";
  $: token1Value = token1?.metrics?.price
    ? (Number(amount1) * Number(token1.metrics.price)).toFixed(2)
    : "0";
  $: totalValue = (Number(token0Value) + Number(token1Value)).toFixed(2);
  $: exchangeRate =
    !isCreatingPool && amount0 && amount1
      ? formatToNonZeroDecimal(Number(amount1) / Number(amount0))
      : $liquidityStore.initialPrice;

  // Helper function to check if token is SOL
  function isSolToken(token: any) {
    return token?.symbol === "SOL" || token?.address === "11111111111111111111111111111111";
  }

  // Helper function to handle cross-chain LP with SOL
  async function handleCrossChainLP(params: any) {
    const isToken0Sol = isSolToken(token0);
    const isToken1Sol = isSolToken(token1);
    
    if (!isToken0Sol && !isToken1Sol) {
      throw new Error("Not a cross-chain LP");
    }

    console.log("Opening Solana liquidity modal for cross-chain LP");
    
    return new Promise((resolve, reject) => {
      let isResolved = false;
      let timeoutId: NodeJS.Timeout;
      
      // Set up timeout to prevent hanging promises
      const TIMEOUT_MS = 5 * 60 * 1000; // 5 minutes
      timeoutId = setTimeout(() => {
        if (!isResolved) {
          isResolved = true;
          console.error("Cross-chain LP operation timed out");
          reject(new Error("Cross-chain operation timed out. Please try again."));
        }
      }, TIMEOUT_MS);
      
      // Store original resolve/reject to handle cleanup
      const safeResolve = (value: any) => {
        if (!isResolved) {
          isResolved = true;
          clearTimeout(timeoutId);
          resolve(value);
        }
      };
      
      const safeReject = (error: any) => {
        if (!isResolved) {
          isResolved = true;
          clearTimeout(timeoutId);
          reject(error);
        }
      };
      
      solanaLiquidityModalStore.show({
        operation: 'add',
        token0: token0,
        amount0: (Number(params.amount_0) / Math.pow(10, token0.decimals)).toString(),
        token1: token1,
        amount1: (Number(params.amount_1) / Math.pow(10, token1.decimals)).toString(),
        lpAmount: '', // not used for add liquidity
        onConfirm: async (modalData) => {
          try {
            const { solTransactionId, icrcTransactionId, pay_signature, timestamp } = modalData;
            
            // Call add_liquidity_async with both transaction details
            const actor = await swapActor({ anon: false, requiresSigning: false });
            
            const addLiquidityArgs = {
              token_0: isToken0Sol ? "SOL" : "IC." + token0.address,
              amount_0: params.amount_0,
              token_1: isToken1Sol ? "SOL" : "IC." + token1.address, 
              amount_1: params.amount_1,
              tx_id_0: isToken0Sol 
                ? (solTransactionId ? [{ TransactionId: solTransactionId }] : [])
                : (icrcTransactionId ? [{ BlockIndex: icrcTransactionId }] : []),
              tx_id_1: isToken1Sol 
                ? (solTransactionId ? [{ TransactionId: solTransactionId }] : [])
                : (icrcTransactionId ? [{ BlockIndex: icrcTransactionId }] : []),
              pay_signature_0: isToken0Sol ? [pay_signature] : [] as [] | [string],
              pay_signature_1: isToken1Sol ? [pay_signature] : [] as [] | [string],
              timestamp: [timestamp] as [] | [bigint],
            };

            console.log("Calling add_liquidity_async with args:", addLiquidityArgs);
            const result = await actor.add_liquidity_async(addLiquidityArgs);
            
            if ("Err" in result) {
              throw new Error(result.Err);
            }
            
            safeResolve(result.Ok);
          } catch (error) {
            console.error("Cross-chain LP error:", error);
            safeReject(error);
          }
        },
        onCancel: () => {
          console.log("Cross-chain LP operation cancelled by user");
          safeReject(new Error("Operation cancelled by user"));
        }
      });
    });
  }

  async function handleConfirm() {
    if (isLoading || !token0 || !token1) return;

    isLoading = true;
    error = null;

    try {
      if (isCreatingPool) {
        // Create pool logic
        const amount0 = parseTokenAmount(
          $liquidityStore.amount0,
          token0.decimals,
        );
        const amount1 = parseTokenAmount(
          $liquidityStore.amount1,
          token1.decimals,
        );

        const params = {
          token_0: token0,
          amount_0: amount0,
          token_1: token1,
          amount_1: amount1,
          initial_price: parseFloat($liquidityStore.initialPrice),
        };

        toastStore.info(
          `Adding liquidity to ${token0.symbol}/${token1.symbol} pool...`,
        );
        const result = await createPool(params);

        if (result) {
          toastStore.success("Pool created successfully!");
          
          // Reload balances and pool list after successful pool creation
          await Promise.all([
            loadBalance(token0.address, true),
            loadBalance(token1.address, true),
            currentUserPoolsStore.initialize(),
          ]);
          
          // Dispatch liquidityAdded event
          dispatch("liquidityAdded");
          
          onClose();
        }
      } else {
        // Add liquidity logic
        const amount0 = parseTokenAmount(
          $liquidityStore.amount0,
          token0.decimals,
        );
        const amount1 = parseTokenAmount(
          $liquidityStore.amount1,
          token1.decimals,
        );

        const params = {
          token_0: token0,
          amount_0: amount0,
          token_1: token1,
          amount_1: amount1,
        };

        // Check if this is a cross-chain LP with SOL
        const isToken0Sol = isSolToken(token0);
        const isToken1Sol = isSolToken(token1);
        
        let addLiquidityResult;
        
        if (isToken0Sol || isToken1Sol) {
          console.log("Detected cross-chain LP with SOL, using signature flow");
          toastStore.info("Initiating cross-chain liquidity addition...");
          addLiquidityResult = await handleCrossChainLP(params);
        } else {
          console.log("Standard ICRC LP, using normal flow");
          toastStore.info(
            `Adding liquidity to ${token0.symbol}/${token1.symbol} pool...`,
          );
          addLiquidityResult = await addLiquidity(params);
        }

        if (addLiquidityResult) {
          // Create new controller for this polling operation
          pollingController = new AbortController();
          
          await pollRequestStatus(
            addLiquidityResult, 
            "Successfully added liquidity",
            "Failed to add liquidity",
            token0?.symbol,
            token1?.symbol,
            pollingController?.signal
          );
          
          // Reload balances and pool list after successful liquidity addition
          await Promise.all([
            loadBalance(token0.address, true),
            loadBalance(token1.address, true),
            currentUserPoolsStore.initialize(),
          ]);
          
          // Dispatch liquidityAdded event
          dispatch("liquidityAdded");
          
          onClose();
        }
      }
    } catch (err) {
      console.error("Error in confirmation:", err);
      if (mounted) {
        error =
          err instanceof Error ? err.message : "Failed to process transaction";
        // Always reset loading state on error
        isLoading = false;
        
        // Show user-friendly error message
        if (err instanceof Error && err.message.includes("cancelled")) {
          toastStore.info("Operation cancelled");
        } else if (err instanceof Error && err.message.includes("timed out")) {
          toastStore.error("Operation timed out. Please try again.");
        } else {
          toastStore.error(error || "Transaction failed");
        }
      }
    } finally {
      // Ensure loading state is always reset
      if (mounted) {
        isLoading = false;
      }
    }
  }

  function handleCancel() {
    // Cancel any ongoing polling
    if (pollingController) {
      pollingController.abort();
      pollingController = null;
    }
    
    // Reset state
    isLoading = false;
    error = null;
    
    show = false;
    onClose();
  }
</script>

<Modal
  title={isCreatingPool ? "Create Pool" : "Add Liquidity"}
  onClose={handleCancel}
  isOpen={show}
  variant="solid"
  width="460px"
  height="auto"
  {modalKey}
  {target}
>
  <div class="flex flex-col min-h-[400px] px-4 pb-4">
    {#if error}
      <div class="mb-4 text-kong-text-accent-red text-center p-4 bg-red-400/20 rounded-xl">
        {error}
      </div>
    {/if}

    <div class="flex-1">
      <div class="text-kong-text-primary/90 mb-1">You will provide</div>

      <div class="bg-white/5 rounded-xl p-4 space-y-4">
        <div class="flex justify-between items-center">
          <div class="flex items-center gap-2">
            <img
              src={token0?.logo_url}
              alt={token0?.symbol}
              class="w-8 h-8 rounded-full bg-white"
            />
            <div class="flex flex-col">
              <span class="text-kong-text-primary text-lg">{token0?.symbol}</span>
            </div>
          </div>
          <div class="flex flex-col items-end">
            <span class="text-kong-text-primary text-xl">{amount0}</span>
            <span class="text-kong-text-secondary text-sm">${token0Value}</span>
          </div>
        </div>

        <div class="text-kong-text-secondary text-xl text-center">+</div>

        <div class="flex justify-between items-center">
          <div class="flex items-center gap-2">
            <img
              src={token1?.logo_url}
              alt={token1?.symbol}
              class="w-8 h-8 rounded-full bg-white"
            />
            <div class="flex flex-col">
              <span class="text-kong-text-primary text-lg">{token1?.symbol}</span>
            </div>
          </div>
          <div class="flex flex-col items-end">
            <span class="text-kong-text-primary text-xl">{amount1}</span>
            <span class="text-kong-text-secondary text-sm">${token1Value}</span>
          </div>
        </div>
      </div>

      <div class="mt-4 bg-white/5 rounded-xl p-4 space-y-3">
        <div class="flex justify-between text-kong-text-primary/80 text-sm">
          <span>Total Value:</span>
          <span>${totalValue}</span>
        </div>
        <div class="flex justify-between text-kong-text-primary/80 text-sm">
          <span>Exchange Rate:</span>
          <span>1 {token0?.symbol} = {exchangeRate} {token1?.symbol}</span>
        </div>
      </div>

      {#if isCreatingPool}
        <div
          class="mt-4 p-3 bg-yellow-500/10 border border-yellow-500/20 rounded-lg"
        >
          <div class="text-kong-text-accent-yellow text-sm">
            You are the first liquidity provider.
            <br />
            The ratio of tokens you add will set the price of this pool.
          </div>
        </div>
      {/if}
    </div>

    <div class="mt-6 flex gap-4">
      <ButtonV2
        variant="outline"
        theme="accent-green"
        size="lg"
        fullWidth={true}
        onclick={handleCancel}
        isDisabled={isLoading}
      >
        Cancel
      </ButtonV2>
      <ButtonV2
        variant="solid"
        theme="accent-green"
        size="lg"
        fullWidth={true}
        onclick={handleConfirm}
        isDisabled={isLoading}
      >
        <div class="flex items-center justify-center gap-2">
          <span>{isLoading ? "Confirming..." : "Confirm"}</span>
          {#if isLoading}
            <div
              class="w-4 h-4 border-2 border-black/20 border-t-black rounded-full animate-spin"
            />
          {/if}
        </div>
      </ButtonV2>
    </div>
  </div>
</Modal>

<style>
  img {
    background: white;
  }
</style>
