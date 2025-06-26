<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { CrossChainSwapService } from '$lib/services/swap/CrossChainSwapService';
  import { toastStore } from '$lib/stores/toastStore';
  import Button from '$lib/components/common/Button.svelte';
  import Modal from '$lib/components/common/Modal.svelte';
  import { Copy, ExternalLink } from 'lucide-svelte';
  import { auth } from '$lib/stores/auth';
  import { Connection, PublicKey, Transaction, SystemProgram, LAMPORTS_PER_SOL } from '@solana/web3.js';
  import { IcrcService } from '$lib/services/icrc/IcrcService';
  import { canisters } from '$lib/config/auth.config';

  // Declare window.solana for TypeScript
  declare global {
    interface Window {
      solana?: {
        isPhantom?: boolean;
        isConnected?: boolean;
        connect(): Promise<{ publicKey: PublicKey }>;
        signAndSendTransaction(transaction: Transaction): Promise<{ signature: string }>;
      };
    }
  }

  export let show = false;
  export let operation: 'add' | 'remove' = 'add'; // add or remove liquidity
  export let token0: Kong.Token;
  export let amount0: string;
  export let token1: Kong.Token;
  export let amount1: string;
  export let lpAmount: string = ''; // for remove liquidity

  const dispatch = createEventDispatcher();
  
  // Add state tracking
  let isMounted = true;

  let kongSolanaAddress = '';
  let userSolanaAddress = '';
  let solTransactionId = '';
  let icrcTransactionId: bigint | undefined = undefined;
  let step: 'icrc' | 'sol' | 'confirm' | 'signing' = 'icrc';
  let loading = false;
  let currentPhase = '';
  let lastError: string | null = null;

  // Determine which token is SOL and which is ICRC
  $: solToken = token0.symbol === 'SOL' ? token0 : token1;
  $: icrcToken = token0.symbol === 'SOL' ? token1 : token0;
  $: solAmount = token0.symbol === 'SOL' ? amount0 : amount1;
  $: icrcAmount = token0.symbol === 'SOL' ? amount1 : amount0;

  $: if (show) {
    initializeFlow();
  }
  
  onDestroy(() => {
    isMounted = false;
    // Clear any ongoing operations
    loading = false;
  });

  async function initializeFlow() {
    try {
      loading = true;
      currentPhase = 'Loading addresses...';
      
      [kongSolanaAddress, userSolanaAddress] = await Promise.all([
        CrossChainSwapService.getKongSolanaAddress(),
        CrossChainSwapService.getSolanaWalletAddress()
      ]);
      
      // Start with ICRC token handling for add liquidity
      if (operation === 'add') {
        await handleIcrcToken();
      } else {
        // For remove liquidity, skip to confirmation
        step = 'confirm';
      }
    } catch (error) {
      console.error('Error initializing flow:', error);
      toastStore.error('Failed to initialize cross-chain flow');
    } finally {
      loading = false;
    }
  }

  async function handleIcrcToken() {
    try {
      loading = true;
      currentPhase = `Processing ${icrcToken.symbol}...`;
      step = 'icrc';
      
      const icrcAmountBigInt = BigInt(Math.floor(parseFloat(icrcAmount) * Math.pow(10, icrcToken.decimals)));
      
      if (icrcToken.standards?.includes("ICRC-2")) {
        console.log("Requesting ICRC-2 approval for", icrcToken.symbol);
        toastStore.info(`Approving ${icrcToken.symbol} spending...`);
        await IcrcService.checkAndRequestIcrc2Allowances(icrcToken, icrcAmountBigInt);
        // For ICRC-2, no transaction ID is returned
        icrcTransactionId = undefined;
      } else {
        // For ICRC-1, transfer the token first
        console.log("Transferring ICRC-1 token", icrcToken.symbol);
        toastStore.info(`Transferring ${icrcToken.symbol}...`);
        const transferResult = await IcrcService.transfer(
          icrcToken,
          canisters.kongBackend.canisterId,
          icrcAmountBigInt
        );
        if (!transferResult?.Ok) {
          throw new Error(`Failed to transfer ${icrcToken.symbol}`);
        }
        icrcTransactionId = transferResult.Ok;
      }
      
      toastStore.success(`${icrcToken.symbol} processed successfully!`);
      
      // Move to SOL transfer step
      await handleSolTransfer();
      
    } catch (error) {
      console.error('ICRC token error:', error);
      if (isMounted) {
        lastError = error instanceof Error ? error.message : `Failed to process ${icrcToken.symbol}`;
        toastStore.error(lastError);
        step = 'icrc'; // Stay on ICRC step for retry
      }
    } finally {
      if (isMounted) {
        loading = false;
      }
    }
  }

  async function handleSolTransfer() {
    try {
      loading = true;
      currentPhase = 'Sending SOL...';
      step = 'sol';
      
      // Get the Phantom provider
      const provider = auth.pnp?.provider;
      if (!provider) {
        throw new Error('Wallet not connected');
      }

      // Create connection
      const connection = new Connection('https://mainnet.solana.validationcloud.io/v1/vEH0znkrmOFeAxXCJAyIGLQFSPVY38NzDe2NkNvNQt4');
      
      // Convert amount to lamports
      const amountInLamports = Math.floor(parseFloat(solAmount) * LAMPORTS_PER_SOL);
      
      // Create transaction
      const transaction = new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: new PublicKey(userSolanaAddress),
          toPubkey: new PublicKey(kongSolanaAddress),
          lamports: amountInLamports,
        })
      );

      // Get recent blockhash
      const { blockhash } = await connection.getLatestBlockhash();
      transaction.recentBlockhash = blockhash;
      transaction.feePayer = new PublicKey(userSolanaAddress);

      toastStore.info('Please approve the SOL transfer in your Phantom wallet...');

      // Try different methods to send the transaction
      let signature = '';
      
      // Method 1: Try using the window.solana object directly (Phantom specific)
      if (window.solana && window.solana.isPhantom) {
        try {
          const phantomProvider = window.solana;
          
          // Request connection if needed
          if (!phantomProvider.isConnected) {
            await phantomProvider.connect();
          }
          
          // Send transaction
          const { signature: txSig } = await phantomProvider.signAndSendTransaction(transaction);
          signature = txSig;
        } catch (e) {
          console.error('Phantom direct method failed:', e);
        }
      }
      
      // Method 2: Try using sendTransaction if available
      if (!signature && (provider as any).sendTransaction) {
        try {
          signature = await (provider as any).sendTransaction(transaction, connection);
          await connection.confirmTransaction(signature, 'confirmed');
        } catch (e) {
          console.error('sendTransaction method failed:', e);
        }
      }

      if (signature) {
        solTransactionId = signature;
        toastStore.success('SOL transfer completed!');
        step = 'confirm';
      } else {
        throw new Error('Failed to send SOL transaction');
      }
      
    } catch (error) {
      console.error('SOL transfer error:', error);
      if (isMounted) {
        lastError = error instanceof Error ? error.message : 'SOL transfer failed';
        toastStore.error(lastError);
        step = 'sol'; // Stay on SOL step for retry
      }
    } finally {
      if (isMounted) {
        loading = false;
      }
    }
  }

  function copyAddress() {
    navigator.clipboard.writeText(kongSolanaAddress);
    toastStore.success('Address copied to clipboard');
  }

  function handleTransferComplete() {
    if (operation === 'add' && !transactionId.trim()) {
      toastStore.error('Please enter the transaction ID');
      return;
    }
    step = 'confirm';
  }

  async function handleConfirmLiquidity() {
    try {
      loading = true;
      step = 'signing';

      // Create timestamp
      const timestamp = BigInt(Date.now());
      
      // Get the IC principal for the user
      const authStore = get(auth);
      const icPrincipal = authStore.account?.owner || '';
      
      let canonicalMessage: string;
      
      if (operation === 'add') {
        // For add liquidity - user sends SOL to Kong
        const amount0BigInt = toBigInt(amount0, token0.decimals);
        const amount1BigInt = toBigInt(amount1, token1.decimals);
        
        canonicalMessage = JSON.stringify({
          token_0: token0.symbol === 'SOL' ? 'SOL' : `IC.${token0.address}`,
          amount_0: amount0BigInt.toString(),
          token_1: token1.symbol === 'SOL' ? 'SOL' : `IC.${token1.address}`,
          amount_1: amount1BigInt.toString(),
          timestamp: timestamp.toString()
        });
      } else {
        // For remove liquidity - user receives SOL from Kong
        const lpAmountBigInt = lpAmount ? toBigInt(lpAmount, 8) : 0n; // LP tokens have 8 decimals
        
        canonicalMessage = JSON.stringify({
          token_0: token0.symbol === 'SOL' ? 'SOL' : `IC.${token0.address}`,
          token_1: token1.symbol === 'SOL' ? 'SOL' : `IC.${token1.address}`,
          remove_lp_token_amount: lpAmountBigInt.toString(),
          payout_address_0: token0.symbol === 'SOL' ? userSolanaAddress : undefined,
          payout_address_1: token1.symbol === 'SOL' ? userSolanaAddress : undefined,
          timestamp: timestamp.toString()
        });
      }

      // Sign the message
      const signature = await CrossChainSwapService.signMessage(canonicalMessage);
      console.log('[SolanaLiquidityModal] Message signed:', canonicalMessage);
      console.log('[SolanaLiquidityModal] Signature:', signature);

      // Dispatch event with liquidity details
      dispatch('confirm', {
        solTransactionId: operation === 'add' ? solTransactionId : undefined,
        icrcTransactionId: operation === 'add' ? icrcTransactionId : undefined,
        signature,
        timestamp,
        canonicalMessage
      });

      handleClose();
    } catch (error) {
      console.error('Error signing message:', error);
      if (isMounted) {
        toastStore.error(error instanceof Error ? error.message : 'Failed to sign message');
        step = 'confirm';
      }
    } finally {
      if (isMounted) {
        loading = false;
      }
    }
  }

  function handleClose() {
    show = false;
    step = 'icrc';
    solTransactionId = '';
    icrcTransactionId = undefined;
    currentPhase = '';
    loading = false;
    lastError = null;
    
    // Notify parent of cancellation
    dispatch('cancel');
    dispatch('close');
  }

  function toBigInt(amount: string, decimals: number): bigint {
    const num = parseFloat(amount);
    return BigInt(Math.floor(num * Math.pow(10, decimals)));
  }
</script>

<Modal {show} onClose={handleClose} title="Cross-Chain Liquidity {operation === 'add' ? 'Addition' : 'Removal'}" size="md">
  <div class="space-y-6">
    {#if loading}
      <div class="text-center space-y-4">
        <div class="animate-spin w-12 h-12 border-4 border-blue-200 border-t-blue-600 rounded-full mx-auto"></div>
        <div>
          <h3 class="font-medium mb-2">{currentPhase}</h3>
          <p class="text-sm text-gray-600">
            Please approve the transaction in your wallet...
          </p>
        </div>
      </div>
    {:else if step === 'icrc'}
      <div class="space-y-4">
        {#if lastError}
          <div class="bg-red-50 border border-red-200 rounded-lg p-4">
            <h4 class="font-medium text-red-900 mb-2">Error</h4>
            <p class="text-sm text-red-700 mb-3">{lastError}</p>
          </div>
        {/if}
        
        <div class="bg-green-50 border border-green-200 rounded-lg p-4">
          <h3 class="font-medium text-green-900 mb-2">
            Step 1: Process {icrcToken.symbol}
          </h3>
          <p class="text-sm text-green-700 mb-3">
            First, we need to {icrcToken.standards?.includes("ICRC-2") ? 'approve' : 'transfer'} {icrcAmount} {icrcToken.symbol}.
          </p>
          
          <div class="bg-white rounded border p-3 mb-3">
            <div class="text-sm font-medium mb-1">Amount:</div>
            <div class="font-mono text-lg font-bold">{icrcAmount} {icrcToken.symbol}</div>
          </div>
        </div>
      </div>
      
      <div class="flex gap-2">
        <Button variant="secondary" fullWidth on:click={handleClose}>
          Cancel
        </Button>
        <Button 
          variant="primary" 
          fullWidth 
          on:click={() => {
            lastError = null;
            handleIcrcToken();
          }}
          disabled={loading}
        >
          {lastError ? 'Retry' : 'Process'} {icrcToken.symbol}
        </Button>
      </div>

    {:else if step === 'sol'}
      <div class="space-y-4">
        {#if lastError}
          <div class="bg-red-50 border border-red-200 rounded-lg p-4">
            <h4 class="font-medium text-red-900 mb-2">Error</h4>
            <p class="text-sm text-red-700 mb-3">{lastError}</p>
          </div>
        {/if}
        
        <div class="bg-purple-50 border border-purple-200 rounded-lg p-4">
          <h3 class="font-medium text-purple-900 mb-2">
            Step 2: Send SOL
          </h3>
          <p class="text-sm text-purple-700 mb-3">
            Now we need to send {solAmount} SOL to Kong's address.
          </p>
          
          <div class="bg-white rounded border p-3 mb-3">
            <div class="text-sm font-medium mb-1">Amount:</div>
            <div class="font-mono text-lg font-bold">{solAmount} SOL</div>
          </div>
        </div>
      </div>
      
      <div class="flex gap-2">
        <Button variant="secondary" fullWidth on:click={handleClose}>
          Cancel
        </Button>
        <Button 
          variant="primary" 
          fullWidth 
          on:click={() => {
            lastError = null;
            handleSolTransfer();
          }}
          disabled={loading}
        >
          {lastError ? 'Retry Send' : 'Send'} SOL
        </Button>
      </div>

    
    {:else if step === 'confirm'}
      <div class="space-y-4">
        <div class="bg-gray-50 rounded-lg p-4">
          <h3 class="font-medium mb-3">Confirm Liquidity {operation === 'add' ? 'Addition' : 'Removal'}</h3>
          
          <div class="grid grid-cols-2 gap-4 text-sm">
            <div>
              <div class="font-medium">{token0.symbol}</div>
              <div class="text-gray-600">{amount0}</div>
            </div>
            <div>
              <div class="font-medium">{token1.symbol}</div>
              <div class="text-gray-600">{amount1}</div>
            </div>
          </div>
          
          {#if operation === 'remove' && lpAmount}
            <div class="mt-3 pt-3 border-t">
              <div class="text-sm">
                <div class="font-medium">LP Tokens to Remove</div>
                <div class="text-gray-600">{lpAmount}</div>
              </div>
            </div>
          {/if}
          
          {#if operation === 'add'}
            <div class="mt-3 pt-3 border-t space-y-2">
              {#if solTransactionId}
                <div class="text-sm">
                  <div class="font-medium">SOL Transaction</div>
                  <div class="font-mono text-xs break-all text-gray-600">{solTransactionId}</div>
                </div>
              {/if}
              {#if icrcTransactionId}
                <div class="text-sm">
                  <div class="font-medium">{icrcToken.symbol} Transaction</div>
                  <div class="font-mono text-xs break-all text-gray-600">{icrcTransactionId}</div>
                </div>
              {:else if icrcToken.standards?.includes("ICRC-2")}
                <div class="text-sm">
                  <div class="font-medium">{icrcToken.symbol} Status</div>
                  <div class="text-gray-600">Approved via ICRC-2</div>
                </div>
              {/if}
            </div>
          {/if}
        </div>
        
        <div class="bg-yellow-50 border border-yellow-200 rounded-lg p-3">
          <p class="text-sm text-yellow-800">
            Next, you'll need to sign a message with your Solana wallet to verify the {operation === 'add' ? 'liquidity addition' : 'withdrawal authorization'}.
          </p>
        </div>
      </div>
      
      <div class="flex gap-2">
        <Button variant="secondary" fullWidth on:click={() => step = 'sol'}>
          Back
        </Button>
        <Button 
          variant="primary" 
          fullWidth 
          on:click={handleConfirmLiquidity}
          disabled={loading}
        >
          {loading ? 'Signing...' : 'Sign Message'}
        </Button>
      </div>
    
    {:else if step === 'signing'}
      <div class="text-center space-y-4">
        <div class="animate-spin w-12 h-12 border-4 border-blue-200 border-t-blue-600 rounded-full mx-auto"></div>
        <div>
          <h3 class="font-medium mb-2">Please sign the message</h3>
          <p class="text-sm text-gray-600">
            Check your Solana wallet for a signing request to complete the {operation === 'add' ? 'liquidity addition' : 'withdrawal'}.
          </p>
        </div>
      </div>
    {/if}
  </div>
</Modal>

<style>
  /* Add any custom styles here */
</style>