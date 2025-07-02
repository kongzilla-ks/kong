<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { CrossChainSwapService } from '$lib/services/swap/CrossChainSwapService';
  import { toastStore } from '$lib/stores/toastStore';
  import Button from '../common/Button.svelte';
  import Modal from '../common/Modal.svelte';
  import { Copy, ExternalLink } from 'lucide-svelte';
  import { auth } from '$lib/stores/auth';
  import { Connection, PublicKey, Transaction, SystemProgram, LAMPORTS_PER_SOL } from '@solana/web3.js';
  import { 
    TOKEN_PROGRAM_ID,
    getAssociatedTokenAddress,
    createAssociatedTokenAccountInstruction,
    createTransferInstruction
  } from '@solana/spl-token';
  import { SOLANA_RPC_ENDPOINT } from '$lib/config/solana.config';
  import { solanaPollingService } from '$lib/services/solana/SolanaPollingService';

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
  export let payToken: Kong.Token;
  export let payAmount: string;
  export let receiveToken: Kong.Token;
  export let receiveAmount: string;
  export let maxSlippage: number;

  const dispatch = createEventDispatcher();

  let kongSolanaAddress = '';
  let userSolanaAddress = '';
  let transactionId = '';
  let step: 'transfer' | 'confirm' | 'signing' = 'transfer';
  let loading = false;

  $: if (show) {
    loadAddresses();
    // Enable silent mode to reduce logging during swap
    solanaPollingService.setSilentMode(true);
  }

  onDestroy(() => {
    // Re-enable logging when modal closes
    solanaPollingService.setSilentMode(false);
  });

  async function loadAddresses() {
    try {
      loading = true;
      [kongSolanaAddress, userSolanaAddress] = await Promise.all([
        CrossChainSwapService.getKongSolanaAddress(),
        CrossChainSwapService.getSolanaWalletAddress()
      ]);
      // Automatically initiate the transfer
      await initiateTransfer();
    } catch (error) {
      console.error('Error loading addresses:', error);
      toastStore.error('Failed to load addresses');
    } finally {
      loading = false;
    }
  }

  async function initiateTransfer() {
    try {
      loading = true;
      
      // Get the Phantom provider
      const provider = auth.pnp?.provider;
      if (!provider) {
        throw new Error('Wallet not connected');
      }

      // For native SOL transfers
      if (payToken.symbol === 'SOL') {
        // Create connection - use Validation Cloud RPC
        const connection = new Connection('https://mainnet.solana.validationcloud.io/v1/vEH0znkrmOFeAxXCJAyIGLQFSPVY38NzDe2NkNvNQt4');
        
        // Convert amount to lamports
        const amountInLamports = Math.floor(parseFloat(payAmount) * LAMPORTS_PER_SOL);
        
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
        
        // Method 3: Try using request method (for some wallet adapters)
        if (!signature && (provider as any).request) {
          try {
            const serializedTx = transaction.serialize({ requireAllSignatures: false });
            const base64Tx = btoa(String.fromCharCode(...serializedTx));
            
            signature = await (provider as any).request({
              method: 'sendTransaction',
              params: {
                transaction: base64Tx,
                options: { skipPreflight: false }
              }
            });
          } catch (e) {
            console.error('request method failed:', e);
          }
        }
        
        if (signature) {
          transactionId = signature;
          toastStore.success('Transfer completed! Preparing swap...');
          step = 'confirm';
          // Automatically proceed to sign the message
          await handleConfirmSwap();
        } else {
          throw new Error('Unable to send transaction with current wallet');
        }
      } else {
        // SPL token transfer implementation
        console.log('[SolanaTransferModal] Initiating SPL token transfer:', payToken.symbol);
        
        // Create connection
        const connection = new Connection(SOLANA_RPC_ENDPOINT);
        
        // Get token mint address from token data
        const tokenMintAddress = payToken.address;
        if (!tokenMintAddress) {
          throw new Error('Token mint address not found');
        }
        
        const mintPubkey = new PublicKey(tokenMintAddress);
        const fromPubkey = new PublicKey(userSolanaAddress);
        const toPubkey = new PublicKey(kongSolanaAddress);
        
        // Get associated token accounts
        const fromTokenAccount = await getAssociatedTokenAddress(
          mintPubkey,
          fromPubkey
        );
        
        const toTokenAccount = await getAssociatedTokenAddress(
          mintPubkey,
          toPubkey
        );
        
        // Check if the recipient's token account exists
        const toTokenAccountInfo = await connection.getAccountInfo(toTokenAccount);
        
        // Create transaction
        const transaction = new Transaction();
        
        // If the recipient's token account doesn't exist, create it
        if (!toTokenAccountInfo) {
          transaction.add(
            createAssociatedTokenAccountInstruction(
              fromPubkey, // payer
              toTokenAccount, // associated token account
              toPubkey, // owner
              mintPubkey // token mint
            )
          );
        }
        
        // Convert amount to token units (handle decimals)
        const tokenDecimals = payToken.decimals || 6; // Default to 6 decimals for USDC
        const amountInTokenUnits = Math.floor(parseFloat(payAmount) * Math.pow(10, tokenDecimals));
        
        // Add transfer instruction
        transaction.add(
          createTransferInstruction(
            fromTokenAccount, // from token account
            toTokenAccount, // to token account
            fromPubkey, // from owner
            amountInTokenUnits, // amount
            [], // multi-signers
            TOKEN_PROGRAM_ID // token program
          )
        );
        
        // Get recent blockhash
        const { blockhash } = await connection.getLatestBlockhash();
        transaction.recentBlockhash = blockhash;
        transaction.feePayer = fromPubkey;
        
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
        
        // Method 3: Try using request method (for some wallet adapters)
        if (!signature && (provider as any).request) {
          try {
            const serializedTx = transaction.serialize({ requireAllSignatures: false });
            const base64Tx = btoa(String.fromCharCode(...serializedTx));
            
            signature = await (provider as any).request({
              method: 'sendTransaction',
              params: {
                transaction: base64Tx,
                options: { skipPreflight: false }
              }
            });
          } catch (e) {
            console.error('request method failed:', e);
          }
        }
        
        if (signature) {
          transactionId = signature;
          console.log('[SolanaTransferModal] SPL transfer signature:', signature);
          toastStore.success('SPL token transfer completed! Preparing swap...');
          step = 'confirm';
          // Automatically proceed to sign the message
          await handleConfirmSwap();
        } else {
          throw new Error('Unable to send SPL token transaction with current wallet');
        }
      }
    } catch (error) {
      console.error('Transfer error:', error);
      toastStore.error(error instanceof Error ? error.message : 'Transfer failed');
      // Fall back to manual transfer
      step = 'transfer';
    } finally {
      loading = false;
    }
  }

  function copyAddress() {
    navigator.clipboard.writeText(kongSolanaAddress);
    toastStore.success('Address copied to clipboard');
  }

  function handleTransferComplete() {
    if (!transactionId.trim()) {
      toastStore.error('Please enter the transaction ID');
      return;
    }
    step = 'confirm';
  }

  async function handleConfirmSwap() {
    try {
      loading = true;
      step = 'signing';

      // Create canonical message
      const timestamp = BigInt(Date.now());
      const payAmountBigInt = toBigInt(payAmount, payToken.decimals);
      const receiveAmountBigInt = toBigInt(receiveAmount, receiveToken.decimals);

      // Get the IC principal for receive address when swapping to IC tokens
      const authStore = get(auth);
      const icPrincipal = authStore.account?.owner || '';
      
      console.log('[SolanaTransferModal] IC Principal:', icPrincipal);
      console.log('[SolanaTransferModal] Solana Address:', userSolanaAddress);
      console.log('[SolanaTransferModal] Receive token chain:', receiveToken.chain);

      // Use IC principal for IC tokens, Solana address for Solana tokens
      // Note: Token chain might be 'ICP' or 'IC' depending on the token
      const receiveAddress = (receiveToken.chain === 'IC' || receiveToken.chain === 'ICP') ? icPrincipal : userSolanaAddress;

      // Use the actual Solana wallet address for signature verification
      const canonicalMessage = CrossChainSwapService.createCanonicalMessage({
        payToken: payToken.symbol,
        payAmount: payAmountBigInt,
        payAddress: userSolanaAddress, // Must match the address that sent the transaction
        receiveToken: receiveToken.symbol,
        receiveAmount: receiveAmountBigInt,
        receiveAddress,
        maxSlippage,
        timestamp,
      });

      // Sign the message
      const signature = await CrossChainSwapService.signMessage(canonicalMessage);
      console.log('[SolanaTransferModal] Message signed:', canonicalMessage);
      console.log('[SolanaTransferModal] Signature:', signature);

      // Dispatch event with swap details
      dispatch('confirm', {
        transactionId,
        pay_signature: signature,
        timestamp,
        canonicalMessage
      });

      handleClose();
    } catch (error) {
      console.error('Error signing message:', error);
      toastStore.error(error instanceof Error ? error.message : 'Failed to sign message');
      step = 'confirm';
    } finally {
      loading = false;
    }
  }

  function handleClose() {
    show = false;
    transactionId = '';
    step = 'transfer';
    dispatch('close');
  }

  // Helper to convert bigint to string for display
  function toBigInt(value: string, decimals: number): bigint {
    const parts = value.split('.');
    const wholePart = parts[0] || '0';
    const decimalPart = (parts[1] || '').padEnd(decimals, '0').slice(0, decimals);
    return BigInt(wholePart + decimalPart);
  }
</script>

<Modal bind:show title="Complete Solana Transfer" onClose={handleClose}>
  <div class="solana-transfer-modal">
    {#if step === 'transfer'}
      <div class="transfer-step">
        <p class="instructions">
          To complete your cross-chain swap, please transfer your {payToken.symbol} to Kong's Solana address:
        </p>

        <div class="transfer-details">
          <div class="detail-row">
            <span class="label">Amount:</span>
            <span class="value">{payAmount} {payToken.symbol}</span>
          </div>
          
          <div class="detail-row">
            <span class="label">To Address:</span>
            <div class="address-container">
              {#if loading}
                <span class="loading">Loading...</span>
              {:else}
                <span class="address">{kongSolanaAddress}</span>
                <button class="copy-btn" on:click={copyAddress} title="Copy address">
                  <Copy size={16} />
                </button>
              {/if}
            </div>
          </div>
        </div>

        <div class="warning">
          <p>⚠️ Important:</p>
          <ul>
            <li>Send exactly {payAmount} {payToken.symbol}</li>
            <li>Save the transaction ID after sending</li>
            <li>Do not close this window until the transfer is complete</li>
          </ul>
        </div>

        <div class="tx-input">
          <label for="txId">Transaction ID:</label>
          <input
            id="txId"
            type="text"
            bind:value={transactionId}
            placeholder="Enter Solana transaction ID"
            disabled={loading}
          />
        </div>

        <div class="actions">
          <Button variant="secondary" on:click={handleClose}>Cancel</Button>
          <Button 
            variant="primary" 
            on:click={handleTransferComplete}
            disabled={!transactionId.trim() || loading}
          >
            I've Completed the Transfer
          </Button>
        </div>
      </div>
    {/if}

    {#if step === 'confirm'}
      <div class="confirm-step">
        <h3>Confirm Swap Details</h3>
        
        <div class="swap-summary">
          <div class="summary-row">
            <span class="label">You're swapping:</span>
            <span class="value">{payAmount} {payToken.symbol}</span>
          </div>
          <div class="summary-row">
            <span class="label">You'll receive:</span>
            <span class="value">{receiveAmount} {receiveToken.symbol}</span>
          </div>
          <div class="summary-row">
            <span class="label">Max slippage:</span>
            <span class="value">{maxSlippage}%</span>
          </div>
          <div class="summary-row">
            <span class="label">Transaction ID:</span>
            <span class="value tx-id">{transactionId}</span>
          </div>
        </div>

        <p class="sign-message">
          Click confirm to sign the swap message with your Solana wallet.
        </p>

        <div class="actions">
          <Button variant="secondary" on:click={() => step = 'transfer'}>Back</Button>
          <Button 
            variant="primary" 
            on:click={handleConfirmSwap}
            disabled={loading}
          >
            Confirm & Sign
          </Button>
        </div>
      </div>
    {/if}

    {#if step === 'signing'}
      <div class="signing-step">
        <div class="spinner"></div>
        <p>Please sign the message in your Solana wallet...</p>
      </div>
    {/if}
  </div>
</Modal>

<style>
  .solana-transfer-modal {
    padding: 1rem;
  }

  .instructions {
    margin-bottom: 1.5rem;
    color: var(--text-secondary);
  }

  .transfer-details {
    background: var(--bg-secondary);
    border-radius: 8px;
    padding: 1rem;
    margin-bottom: 1.5rem;
  }

  .detail-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .detail-row:last-child {
    margin-bottom: 0;
  }

  .label {
    color: var(--text-secondary);
    font-size: 0.875rem;
  }

  .value {
    font-weight: 600;
    color: var(--text-primary);
  }

  .address-container {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .address {
    font-family: monospace;
    font-size: 0.875rem;
    word-break: break-all;
  }

  .copy-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--text-secondary);
    padding: 0.25rem;
    border-radius: 4px;
    transition: all 0.2s;
  }

  .copy-btn:hover {
    color: var(--text-primary);
    background: var(--bg-tertiary);
  }

  .warning {
    background: var(--warning-bg, #fef3c7);
    border: 1px solid var(--warning-border, #f59e0b);
    border-radius: 8px;
    padding: 1rem;
    margin-bottom: 1.5rem;
  }

  .warning p {
    margin: 0 0 0.5rem 0;
    font-weight: 600;
    color: var(--warning-text, #92400e);
  }

  .warning ul {
    margin: 0;
    padding-left: 1.5rem;
  }

  .warning li {
    color: var(--warning-text, #92400e);
    font-size: 0.875rem;
  }

  .tx-input {
    margin-bottom: 1.5rem;
  }

  .tx-input label {
    display: block;
    margin-bottom: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.875rem;
  }

  .tx-input input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-family: monospace;
    font-size: 0.875rem;
  }

  .tx-input input:focus {
    outline: none;
    border-color: var(--primary);
  }

  .actions {
    display: flex;
    gap: 1rem;
    justify-content: flex-end;
  }

  .confirm-step h3 {
    margin: 0 0 1.5rem 0;
    color: var(--text-primary);
  }

  .swap-summary {
    background: var(--bg-secondary);
    border-radius: 8px;
    padding: 1rem;
    margin-bottom: 1.5rem;
  }

  .summary-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .summary-row:last-child {
    margin-bottom: 0;
  }

  .tx-id {
    font-family: monospace;
    font-size: 0.75rem;
    word-break: break-all;
  }

  .sign-message {
    color: var(--text-secondary);
    font-size: 0.875rem;
    margin-bottom: 1.5rem;
  }

  .signing-step {
    text-align: center;
    padding: 3rem 1rem;
  }

  .spinner {
    width: 48px;
    height: 48px;
    border: 3px solid var(--border-color);
    border-top-color: var(--primary);
    border-radius: 50%;
    margin: 0 auto 1.5rem;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .loading {
    color: var(--text-secondary);
    font-style: italic;
  }
</style>