<script lang="ts">
    import { fade, fly } from 'svelte/transition';
    import { IcrcService } from "$lib/services/icrc/IcrcService";
    import { toastStore } from "$lib/stores/toastStore";
    import { Principal } from "@dfinity/principal";
    import Modal from '$lib/components/common/Modal.svelte';
    import { formatTokenAmount, toMinimalUnit } from '$lib/utils/numberFormatUtils';
    import BigNumber from 'bignumber.js';
    import QrScanner from '$lib/components/common/QrScanner.svelte';
    import { onMount, onDestroy } from 'svelte';
    import { Clipboard } from 'lucide-svelte';
    import { Camera } from 'lucide-svelte';
    import { AccountIdentifier } from '@dfinity/ledger-icp';
    import { SubAccount } from '@dfinity/ledger-icp';
    import { auth } from "$lib/services/auth";

    export let token: FE.Token;

    let recipientAddress = '';
    let amount = '';
    let isValidating = false;
    let errorMessage = '';
    let tokenFee: bigint;
    let showScanner = false;
    let hasCamera = false;
    let selectedAccount: 'subaccount' | 'main' = 'main';
    let accounts = {
        subaccount: '',
        main: ''
    };

    let balances = token?.symbol === 'ICP' 
        ? {
            default: BigInt(0),
            subaccount: BigInt(0)
        }
        : {
            default: BigInt(0)
        };

    async function loadTokenFee() {
        try {
            tokenFee = await IcrcService.getTokenFee(token);
        } catch (error) {
            console.error('Error loading token fee:', error);
            tokenFee = BigInt(10000); // Fallback to default fee
        }
    }

    $: if (token) {
        loadTokenFee();
    }

    $: maxAmount = token?.symbol === 'ICP' 
        ? (selectedAccount === 'main' 
            ? new BigNumber(balances.default.toString())
                .dividedBy(new BigNumber(10).pow(token.decimals))
                .minus(new BigNumber(tokenFee?.toString() || '10000').dividedBy(new BigNumber(10).pow(token.decimals)))
                .toNumber()
            : new BigNumber(balances.subaccount.toString())
                .dividedBy(new BigNumber(10).pow(token.decimals))
                .minus(new BigNumber(tokenFee?.toString() || '10000').dividedBy(new BigNumber(10).pow(token.decimals)))
                .toNumber())
        : new BigNumber(balances.default.toString())
            .dividedBy(new BigNumber(10).pow(token.decimals))
            .minus(new BigNumber(tokenFee?.toString() || '10000').dividedBy(new BigNumber(10).pow(token.decimals)))
            .toNumber();
    let addressType: 'principal' | 'account' | null = null;
    let showConfirmation = false;

    function isValidHex(str: string): boolean {
        const hexRegex = /^[0-9a-fA-F]+$/;
        return hexRegex.test(str);
    }

    function detectAddressType(address: string): 'principal' | 'account' | null {
        if (!address) return null;

        // Check for Account ID (64 character hex string)
        if (address.length === 64 && isValidHex(address)) {
            return 'account';
        }

        // Check for Principal ID
        try {
            Principal.fromText(address);
            return 'principal';
        } catch {
            return null;
        }
    }

    function validateAddress(address: string): boolean {
        if (!address) return false;

        const cleanAddress = address.trim();
        
        if (cleanAddress.length === 0) {
            errorMessage = 'Address cannot be empty';
            return false;
        }

        addressType = detectAddressType(cleanAddress);

        if (addressType === null) {
            errorMessage = 'Invalid address format';
            return false;
        }

        // Handle Account ID validation
        if (addressType === 'account') {
            if (cleanAddress.length !== 64 || !isValidHex(cleanAddress)) {
                errorMessage = 'Invalid Account ID format';
                return false;
            }
            return true;
        }

        // Handle Principal ID validation
        try {
            const principal = Principal.fromText(cleanAddress);
            if (principal.isAnonymous()) {
                errorMessage = 'Cannot send to anonymous principal';
                return false;
            }
        } catch (err) {
            errorMessage = 'Invalid Principal ID format';
            return false;
        }

        errorMessage = '';
        return true;
    }

    function validateAmount(value: string): boolean {
        if (!value) return false;
        const numValue = parseFloat(value);
        
        if (isNaN(numValue) || numValue <= 0) {
            errorMessage = 'Amount must be greater than 0';
            return false;
        }
        
        const currentBalance = selectedAccount === 'main' ? balances.default : balances.subaccount;
        const maxAmount = new BigNumber(currentBalance.toString())
            .dividedBy(new BigNumber(10).pow(token.decimals))
            .minus(new BigNumber(tokenFee?.toString() || '10000').dividedBy(new BigNumber(10).pow(token.decimals)))
            .toNumber();
        
        if (numValue > maxAmount) {
            errorMessage = 'Insufficient balance';
            return false;
        }
        
        return true;
    }

    function handleAmountInput(event: Event) {
        const input = event.target as HTMLInputElement;
        let value = input.value.replace(/[^0-9.]/g, '');
        
        const parts = value.split('.');
        if (parts.length > 2) {
            value = `${parts[0]}.${parts[1]}`;
        }

        if (parts[1]?.length > token.decimals) {
            value = `${parts[0]}.${parts[1].slice(0, token.decimals)}`;
        }

        amount = value;
        errorMessage = '';
        validateAmount(value);
    }

    async function handleSubmit() {
        showConfirmation = true;
    }

    function getAccountIds(principalStr: string, rawSubaccount: any): { subaccount: string, main: string } {
        try {
            const principal = Principal.fromText(principalStr);
            
            // Create account ID with provided subaccount
            const subAccount = convertToSubaccount(rawSubaccount);
            const subaccountId = AccountIdentifier.fromPrincipal({
                principal,
                subAccount
            }).toHex();
            
            // Create account ID with main (zero) subaccount
            const mainAccountId = AccountIdentifier.fromPrincipal({
                principal,
                subAccount: undefined
            }).toHex();
            
            return {
                subaccount: subaccountId,
                main: mainAccountId
            };
        } catch (error) {
            console.error('Error creating account identifier:', error);
            return {
                subaccount: '',
                main: ''
            };
        }
    }

    function convertToSubaccount(raw: any): SubAccount | undefined {
        try {
            if (!raw) return undefined;
            
            if (raw instanceof SubAccount) return raw;
            
            let bytes: Uint8Array;
            if (raw instanceof Uint8Array) {
                bytes = raw;
            } else if (Array.isArray(raw)) {
                bytes = new Uint8Array(raw);
            } else if (typeof raw === 'number') {
                bytes = new Uint8Array(32).fill(0);
                bytes[31] = raw;
            } else {
                return undefined;
            }
            
            if (bytes.length !== 32) {
                const paddedBytes = new Uint8Array(32).fill(0);
                paddedBytes.set(bytes.slice(0, 32));
                bytes = paddedBytes;
            }
            
            const subAccountResult = SubAccount.fromBytes(bytes);
            if (subAccountResult instanceof Error) {
                throw subAccountResult;
            }
            return subAccountResult;
        } catch (error) {
            console.error('Error converting subaccount:', error);
            return undefined;
        }
    }

    $: if (auth.pnp?.account?.owner) {
        const principal = auth.pnp.account.owner;
        const principalStr = typeof principal === 'string' ? principal : principal?.toText?.() || '';
        accounts = getAccountIds(principalStr, auth.pnp?.account?.subaccount);
    }

    async function confirmTransfer() {
        isValidating = true;
        errorMessage = '';
        showConfirmation = false;

        try {
            const decimals = token.decimals || 8;
            const amountBigInt = BigInt(Math.floor(Number(amount) * 10 ** decimals).toString());

            toastStore.info(`Sending ${token.symbol}...`);
            
            // Get the correct subaccount based on selection
            const fromSubaccount = selectedAccount === 'subaccount' 
                ? auth.pnp?.account?.subaccount 
                : undefined;

            let result;
            if (addressType === 'account') {
                result = await IcrcService.icrc1Transfer(
                    token,
                    recipientAddress,
                    amountBigInt,
                    { 
                        fee: BigInt(token.fee_fixed),
                        fromSubaccount: fromSubaccount ? Array.from(fromSubaccount) : undefined,
                    }
                );
            } else {
                result = await IcrcService.icrc1Transfer(
                    token,
                    recipientAddress,
                    amountBigInt,
                    { 
                        fee: BigInt(token.fee_fixed),
                        fromSubaccount: fromSubaccount ? Array.from(fromSubaccount) : undefined,
                    }
                );
            }

            if (result?.Ok) {
                toastStore.success(`Successfully sent ${token.symbol}`);
                recipientAddress = '';
                amount = '';
                // Reload balances after successful transfer
                await loadBalances();
            } else if (result?.Err) {
                const errMsg = typeof result.Err === "object" 
                    ? Object.keys(result.Err)[0]
                    : String(result.Err);
                errorMessage = `Transfer failed: ${errMsg}`;
                toastStore.error(errorMessage);
            }
        } catch (err) {
            errorMessage = err.message || "Transfer failed";
            toastStore.error(errorMessage);
        } finally {
            isValidating = false;
        }
    }

    function setMaxAmount() {
        amount = maxAmount.toFixed(token.decimals);
        errorMessage = '';
    }

    $: {
        if (recipientAddress) {
            validateAddress(recipientAddress);
        } else {
            addressType = null;
            errorMessage = '';
        }
    }

    $: validationMessage = (() => {
        if (!recipientAddress) return { 
            type: 'info', 
            text: 'Enter a Principal ID or Account ID' 
        };
        if (errorMessage) return { 
            type: 'error', 
            text: errorMessage 
        };
        if (addressType === 'principal') return { 
            type: 'success', 
            text: 'Valid Principal ID' 
        };
        if (addressType === 'account') return { 
            type: 'success', 
            text: 'Valid Account ID' 
        };
        return { 
            type: 'error', 
            text: 'Invalid address format' 
        };
    })();

    async function handlePaste() {
        try {
            const text = await navigator.clipboard.readText();
            recipientAddress = text.trim();
        } catch (err) {
            toastStore.error('Failed to paste from clipboard');
        }
    }

    function handleScan(scannedText: string) {
        const cleanedText = scannedText.trim();
        console.log('Scanned text:', cleanedText);
        
        if (validateAddress(cleanedText)) {
            recipientAddress = cleanedText;
            toastStore.success('QR code scanned successfully');
            showScanner = false;
        } else {
            toastStore.error('Invalid QR code. Please scan a valid Principal ID or Account ID');
        }
    }

    async function checkCameraAvailability() {
        try {
            const devices = await navigator.mediaDevices.enumerateDevices();
            hasCamera = devices.some(device => device.kind === 'videoinput');
        } catch (err) {
            console.debug('Error checking camera:', err);
            hasCamera = false;
        }
    }

    onMount(() => {
        checkCameraAvailability();
    });

    async function loadBalances() {
        try {
            if (!auth.pnp?.account?.owner || !token) return;
            
            if (token.symbol === 'ICP') {
                const result = await IcrcService.getIcrc1Balance(
                    token,
                    auth.pnp.account.owner,
                    auth.pnp?.account?.subaccount ? Array.from(auth.pnp.account.subaccount) : undefined,
                    true
                );
                
                if ('default' in result) {
                    balances = result;
                } else {
                    balances = {
                        default: result,
                        subaccount: BigInt(0)
                    };
                }
            } else {
                const result = await IcrcService.getIcrc1Balance(
                    token,
                    auth.pnp.account.owner,
                    undefined,
                    false
                );
                balances = {
                    default: result
                };
            }
        } catch (error) {
            console.error('Error loading balances:', error);
            balances = token.symbol === 'ICP' 
                ? { default: BigInt(0), subaccount: BigInt(0) }
                : { default: BigInt(0) };
        }
    }

    $: if (token && auth.pnp?.account?.owner) {
        loadBalances();
    }

    $: if (selectedAccount) {
        loadBalances();
    }
</script>

<div class="container" transition:fade>
    <form on:submit|preventDefault={handleSubmit}>
        <div class="id-card">
            <div class="id-header">
                <span>Recipient Address</span>
                <div class="header-actions">
                        <button 
                            type="button"
                            class="header-button"
                            on:click={() => showScanner = true}
                            title="Scan QR Code"
                        >
                            <Camera class="w-4 h-4" />
                        </button>
                    <button 
                        type="button"
                        class="header-button"
                        on:click={recipientAddress ? () => recipientAddress = '' : handlePaste}
                    >
                        {#if recipientAddress}✕{:else}<Clipboard class="w-4 h-4" /> {/if}
                    </button>
                </div>
            </div>

            <div class="input-group">
                <div class="input-wrapper">
                    <input
                        type="text"
                        bind:value={recipientAddress}
                        placeholder="Paste address or enter manually"
                        class:error={errorMessage && recipientAddress}
                        class:valid={addressType === 'principal' && !errorMessage}
                    />
                </div>
                
                {#if recipientAddress}
                    <div 
                        class="validation-status" 
                        class:success={validationMessage.type === 'success'} 
                        class:error={validationMessage.type === 'error'}
                    >
                        <span class="status-text">{validationMessage.text}</span>
                    </div>
                {/if}
            </div>
        </div>

        <div class="id-card">
            <div class="id-header">
                <span>Amount</span>
                <button type="button" class="header-button" on:click={setMaxAmount}>MAX</button>
            </div>

            <div class="input-group">
                <div class="input-wrapper">
                    <input
                        type="text"
                        inputmode="decimal"
                        placeholder="Enter amount"
                        bind:value={amount}
                        on:input={handleAmountInput}
                        class:error={errorMessage.includes('balance') || errorMessage.includes('Amount')}
                    />
                </div>
                <div class="balance-info">
                    {#if token.symbol === 'ICP'}
                        <div class="balance-row">
                            <span>Default Account:</span>
                            <span>{formatTokenAmount(balances.default, token.decimals)} {token.symbol}</span>
                        </div>
                        <div class="balance-row">
                            <span>Subaccount:</span>
                            <span>{formatTokenAmount(balances.subaccount, token.decimals)} {token.symbol}</span>
                        </div>
                        <div class="balance-row total">
                            <span>Selected Balance:</span>
                            <span>{formatTokenAmount(selectedAccount === 'main' ? balances.default : balances.subaccount, token.decimals)} {token.symbol}</span>
                        </div>
                    {:else}
                        <div class="balance-row total">
                            <span>Available Balance:</span>
                            <span>{formatTokenAmount(balances.default, token.decimals)} {token.symbol}</span>
                        </div>
                    {/if}
                </div>
            </div>
        </div>

        {#if token.symbol === 'ICP'}
            <div class="id-card">
                <div class="id-header">
                    <span>Source Account</span>
                </div>
                <div class="input-group">
                    <select 
                        bind:value={selectedAccount}
                        class="account-select"
                    >
                        <option value="subaccount">Subaccount ({accounts.subaccount.slice(0, 6)}...{accounts.subaccount.slice(-6)})</option>
                        <option value="main">Main Account ({accounts.main.slice(0, 6)}...{accounts.main.slice(-6)})</option>
                    </select>
                </div>
            </div>
        {/if}

        {#if errorMessage}
            <div class="error-message" transition:fade={{duration: 200}}>
                {errorMessage}
            </div>
        {/if}

        <button 
            type="submit" 
            class="send-btn"
            disabled={isValidating || !amount || !recipientAddress || addressType === 'account'}
        >
            {#if addressType === 'account'}
                Sending to Account IDs coming soon
            {:else}
                Send Tokens
            {/if}
        </button>
    </form>

    {#if showConfirmation}
        <Modal
            isOpen={showConfirmation}
            onClose={() => showConfirmation = false}
            title="Confirm Your Transfer"
            width="min(450px, 95vw)"
            height="auto"
        >
            <div class="confirm-box">
                <div class="confirm-details">
                    <div class="transfer-summary">
                        <div class="amount-display">
                            <span class="amount">{amount}</span>
                            <span class="symbol">{token.symbol}</span>
                        </div>
                    </div>
                    
                    <div class="details-grid">
                        <div class="detail-item">
                            <span class="label">You Send</span>
                            <span class="value">{amount} {token.symbol}</span>
                        </div>
                        <div class="detail-item">
                            <span class="label">Network Fee</span>
                            <span class="value">{formatTokenAmount(tokenFee?.toString() || '10000', token.decimals)} {token.symbol}</span>
                        </div>
                        <div class="detail-item">
                            <span class="label">Receiver Gets</span>
                            <span class="value">{parseFloat(amount).toFixed(token.decimals)} {token.symbol}</span>
                        </div>
                        <div class="detail-item total">
                            <span class="label">Total Amount</span>
                            <span class="value">{(parseFloat(amount) + parseFloat(tokenFee?.toString() || '10000') / 10 ** token.decimals).toFixed(4)} {token.symbol}</span>
                        </div>
                    </div>
                </div>

                <div class="confirm-actions">
                    <button class="cancel-btn" on:click={() => showConfirmation = false}>Cancel</button>
                    <button 
                        class="confirm-btn" 
                        class:loading={isValidating}
                        on:click={confirmTransfer}
                        disabled={isValidating}
                    >
                        {#if isValidating}
                            <span class="loading-spinner"></span>
                            Processing...
                        {:else}
                            Confirm Transfer
                        {/if}
                    </button>
                </div>
            </div>
        </Modal>
    {/if}

    {#if showScanner}
        <QrScanner 
            isOpen={showScanner}
            onClose={() => showScanner = false}
            onScan={handleScan}
        />
    {/if}
</div>

<style lang="postcss">
    .container {
        @apply flex flex-col gap-4 py-4;
    }

    .id-card {
        @apply bg-white/5 rounded-xl p-4 mb-2;
    }

    .id-header {
        @apply flex justify-between items-center mb-2 text-white/70 text-sm;
    }

    .header-actions {
        @apply flex items-center gap-2;
    }

    .header-button {
        @apply px-3 py-1 bg-white/10 rounded-lg hover:bg-white/20 text-white;
    }

    .max-btn {
        @apply px-3 py-1 bg-white/10 rounded-lg hover:bg-white/20 text-white;
    }

    .input-wrapper {
        @apply relative flex items-center;

        input {
            @apply w-full px-3 py-2 bg-black/20 rounded-lg text-white
                   border border-white/10 hover:border-white/20
                   focus:border-indigo-500 focus:outline-none;

            &.error { 
                @apply border-red-500/50 bg-red-500/10; 
            }
        }
    }

    .error-message {
        @apply text-red-400 text-sm px-2 mb-2;
    }

    .send-btn {
        @apply w-full py-3 bg-indigo-500 text-white rounded-lg
               font-medium hover:bg-indigo-600 disabled:opacity-50;
    }

    .validation-status {
        @apply text-sm mt-1 px-1;
        &.success { @apply text-green-400; }
        &.error { @apply text-red-400; }
    }

    .balance-info {
        @apply text-right text-sm text-white/60;
    }

    .confirm-box {
        @apply p-6;
        
        .transfer-summary {
            @apply mb-6 text-center;
            
            .amount-display {
                @apply flex items-baseline justify-center gap-2;
                
                .amount {
                    @apply text-3xl font-bold text-white;
                }
                
                .symbol {
                    @apply text-lg text-white/70;
                }
            }
        }
        
        .details-grid {
            @apply space-y-3 mb-6;
            
            .detail-item {
                @apply flex justify-between items-center p-3 rounded-lg bg-white/5;
                
                .label {
                    @apply text-sm text-white/60;
                }
                
                .value {
                    @apply text-sm text-white/90;
                    
                    &.address {
                        @apply max-w-[200px] truncate;
                    }
                    
                    &.type {
                        @apply capitalize;
                    }
                }
                
                &.total {
                    @apply mt-4 bg-white/10;
                    .label, .value {
                        @apply font-medium text-white;
                    }
                }
            }
        }
        
        .confirm-actions {
            @apply flex gap-3 pt-4 border-t border-white/10;
            
            button {
                @apply flex-1 py-3 rounded-lg font-medium text-center justify-center items-center gap-2;
            }
            
            .cancel-btn {
                @apply bg-white/10 hover:bg-white/15 text-white/90;
            }
            
            .confirm-btn {
                @apply bg-indigo-500 hover:bg-indigo-600 text-white disabled:opacity-50 disabled:cursor-not-allowed;
                &.loading {
                    @apply bg-indigo-500/50;
                }
            }
        }
    }

    .loading-spinner {
        @apply inline-block h-4 w-4 border-2 border-white/30 border-t-white rounded-full animate-spin;
    }

    .scanner-container {
        @apply p-4 flex flex-col items-center gap-4;

        :global(#qr-reader) {
            @apply w-full max-w-[300px] mx-auto bg-black/20 rounded-lg overflow-hidden;
            
            :global(video) {
                @apply rounded-lg;
            }

            :global(#qr-reader__header_message),
            :global(#qr-reader__filescan_input),
            :global(#qr-reader__dashboard_section_csr) {
                @apply hidden;
            }

            :global(#qr-reader__scan_region) {
                @apply bg-transparent rounded-lg relative;
                border: 2px solid theme('colors.indigo.500') !important;
            }
        }

        .scanner-controls {
            @apply flex gap-3 mt-4;
        }

        .switch-camera-btn {
            @apply px-4 py-2 bg-white/10 hover:bg-white/15 text-white/90 
                   rounded-lg flex items-center justify-center;
        }

        .cancel-scan-btn {
            @apply px-4 py-2 bg-white/10 hover:bg-white/15 text-white/90 rounded-lg;
        }
    }

    @keyframes scan {
        0% { 
            transform: translateY(-100%);
            opacity: 0;
        }
        50% { 
            opacity: 1;
        }
        100% { 
            transform: translateY(100%);
            opacity: 0;
        }
    }

    .account-select {
        @apply w-full px-3 py-2 bg-black/20 rounded-lg text-white
               border border-white/10 hover:border-white/20
               focus:border-indigo-500 focus:outline-none;
    }

    .balance-row {
        @apply flex justify-between text-sm text-white/60;
        
        &.total {
            @apply mt-1 pt-1 border-t border-white/10 text-white/90 font-medium;
        }
    }

    .account-info {
        @apply w-full px-3 py-2 bg-black/20 rounded-lg text-white/70;
    }
</style>
