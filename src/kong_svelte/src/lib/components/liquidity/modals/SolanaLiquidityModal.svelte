<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { get } from 'svelte/store';
  import { CrossChainSwapService } from '$lib/services/swap/CrossChainSwapService';
  import { toastStore } from '$lib/stores/toastStore';
  import Modal from '$lib/components/common/Modal.svelte';
  import { auth } from '$lib/stores/auth';
  import { Connection, PublicKey, Transaction, SystemProgram, LAMPORTS_PER_SOL } from '@solana/web3.js';
  import { IcrcService } from '$lib/services/icrc/IcrcService';
  import { canisters } from '$lib/config/auth.config';
  import { SOLANA_RPC_ENDPOINT } from '$lib/config/solana.config';

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
  export let operation: 'add' | 'remove' = 'add';
  export let token0: Kong.Token;
  export let amount0: string;
  export let token1: Kong.Token;
  export let amount1: string;
  export let lpAmount: string = '';

  const dispatch = createEventDispatcher();

  let kongSolanaAddress = '';
  let userSolanaAddress = '';
  let solTransactionId = '';
  let icrcTransactionId: bigint | undefined = undefined;
  let currentPhase = '';

  // Determine which token is SOL and which is ICRC
  $: solToken = token0?.symbol === 'SOL' ? token0 : token1;
  $: icrcToken = token0?.symbol === 'SOL' ? token1 : token0;
  $: solAmount = token0?.symbol === 'SOL' ? amount0 : amount1;
  $: icrcAmount = token0?.symbol === 'SOL' ? amount1 : amount0;

  // Simple reactive - when show becomes true, start the flow
  $: if (show) {
    startFlow();
  }


  async function startFlow() {
    try {
      currentPhase = 'Loading addresses...';

      // Get addresses
      [kongSolanaAddress, userSolanaAddress] = await Promise.all([
        CrossChainSwapService.getKongSolanaAddress(),
        CrossChainSwapService.getSolanaWalletAddress()
      ]);

      if (operation === 'add') {
        // Step 1: Handle ICRC token (approve or transfer)
        await handleIcrcToken();

        // Step 2: Send SOL
        await handleSolTransfer();

        // Step 3: Auto-sign and complete (like swap modal)
        await handleConfirmLiquidity();
      } else {
        // For remove liquidity, auto-sign
        await handleConfirmLiquidity();
      }
    } catch (error) {
      console.error('Error in flow:', error);
      toastStore.error(error instanceof Error ? error.message : 'Failed to process');
      handleClose();
    }
  }

  async function handleIcrcToken() {
    currentPhase = `Processing ${icrcToken.symbol}...`;

    const icrcAmountBigInt = BigInt(Math.floor(parseFloat(icrcAmount) * Math.pow(10, icrcToken.decimals)));

    if (icrcToken.standards?.includes("ICRC-2")) {
      console.log("Requesting ICRC-2 approval for", icrcToken.symbol);
      toastStore.info(`Approving ${icrcToken.symbol} spending...`);
      await IcrcService.checkAndRequestIcrc2Allowances(icrcToken, icrcAmountBigInt);
      icrcTransactionId = undefined;
    } else {
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
  }

  async function handleSolTransfer() {
    currentPhase = 'Sending SOL...';

    const provider = auth.pnp?.provider;
    if (!provider) {
      throw new Error('Wallet not connected');
    }

    const connection = new Connection(SOLANA_RPC_ENDPOINT);
    const amountInLamports = Math.floor(parseFloat(solAmount) * LAMPORTS_PER_SOL);

    const transaction = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: new PublicKey(userSolanaAddress),
        toPubkey: new PublicKey(kongSolanaAddress),
        lamports: amountInLamports,
      })
    );

    const { blockhash } = await connection.getLatestBlockhash();
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = new PublicKey(userSolanaAddress);

    toastStore.info('Please approve the SOL transfer in your Phantom wallet...');

    let signature = '';

    // Method 1: Native Phantom
    if (window.solana && window.solana.isPhantom) {
      try {
        const phantomProvider = window.solana;
        if (!phantomProvider.isConnected) {
          await phantomProvider.connect();
        }
        const { signature: txSig } = await phantomProvider.signAndSendTransaction(transaction);
        signature = txSig;
      } catch (e) {
        console.error('Phantom direct method failed:', e);
      }
    }

    // Method 2: Provider sendTransaction
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
    } else {
      throw new Error('Failed to send SOL transaction');
    }
  }

  async function handleConfirmLiquidity() {
    currentPhase = 'Please sign the message in your wallet...';

    const authStore = get(auth);
    const icPrincipal = authStore.account?.owner || '';

    let canonicalMessage: string;

    if (operation === 'add') {
      const amount0BigInt = toBigInt(amount0, token0.decimals);
      const amount1BigInt = toBigInt(amount1, token1.decimals);

      canonicalMessage = JSON.stringify({
        token_0: token0.symbol === 'SOL' ? `SOL.${token0.address}` : `IC.${token0.address}`,
        amount_0: amount0BigInt.toString(),
        token_1: token1.symbol === 'SOL' ? `SOL.${token1.address}` : `IC.${token1.address}`,
        amount_1: amount1BigInt.toString()
      });
    } else {
      const lpAmountBigInt = lpAmount ? toBigInt(lpAmount, 8) : 0n;

      canonicalMessage = JSON.stringify({
        token_0: token0.symbol === 'SOL' ? `SOL.${token0.address}` : `IC.${token0.address}`,
        token_1: token1.symbol === 'SOL' ? `SOL.${token1.address}` : `IC.${token1.address}`,
        remove_lp_token_amount: lpAmountBigInt.toString(),
        payout_address_0: token0.symbol === 'SOL' ? userSolanaAddress : null,
        payout_address_1: token1.symbol === 'SOL' ? userSolanaAddress : null
      });
    }

    const signature = await CrossChainSwapService.signMessage(canonicalMessage);
    console.log('[SolanaLiquidityModal] Message signed:', canonicalMessage);

    dispatch('confirm', {
      solTransactionId: operation === 'add' ? solTransactionId : undefined,
      icrcTransactionId: operation === 'add' ? icrcTransactionId : undefined,
      pay_signature: signature,
      canonicalMessage
    });

    handleClose();
  }

  function handleClose() {
    show = false;
    solTransactionId = '';
    icrcTransactionId = undefined;
    currentPhase = '';
    dispatch('close');
  }

  function toBigInt(amount: string, decimals: number): bigint {
    const num = parseFloat(amount);
    return BigInt(Math.floor(num * Math.pow(10, decimals)));
  }
</script>

<Modal bind:show title="Cross-Chain Liquidity {operation === 'add' ? 'Addition' : 'Removal'}" onClose={handleClose}>
  <div class="liquidity-modal">
    <div class="processing-step">
      <div class="spinner"></div>
      <h3>{currentPhase}</h3>
      <p>Please approve the transactions in your wallet...</p>
    </div>
  </div>
</Modal>

<style>
  .liquidity-modal {
    padding: 1rem;
  }

  .processing-step {
    text-align: center;
    padding: 3rem 1rem;
  }

  .processing-step h3 {
    margin: 1rem 0 0.5rem 0;
    color: var(--text-primary);
  }

  .processing-step p {
    color: var(--text-secondary);
    font-size: 0.875rem;
  }

  .spinner {
    width: 48px;
    height: 48px;
    border: 3px solid var(--border-color, #e5e7eb);
    border-top-color: var(--primary, #3b82f6);
    border-radius: 50%;
    margin: 0 auto;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
