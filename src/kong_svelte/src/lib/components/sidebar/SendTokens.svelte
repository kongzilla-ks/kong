<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { IcrcService } from "$lib/services/icrc/IcrcService";
  import { toastStore } from "$lib/stores/toastStore";
  import { Principal } from "@dfinity/principal";
  import { formatBalance } from "$lib/utils/numberFormatUtils";
  import BigNumber from "bignumber.js";
  import QrScanner from "$lib/components/common/QrScanner.svelte";
  import { onMount } from "svelte";
  import { Clipboard } from "lucide-svelte";
  import { Camera } from "lucide-svelte";
  import { AccountIdentifier } from "@dfinity/ledger-icp";
  import { SubAccount } from "@dfinity/ledger-icp";
  import { auth } from "$lib/services/auth";
  import { tooltip } from "$lib/actions/tooltip";

  export let token: FE.Token;

  let recipientAddress = "";
  let amount = "";
  let isValidating = false;
  let errorMessage = "";
  let tokenFee: bigint;
  let showScanner = false;
  let hasCamera = false;
  let selectedAccount: "main" | "subaccount" = "main";
  let accounts = {
    subaccount: "",
    main: "",
  };

  let balances: { default: bigint; subaccount?: bigint } =
    token?.symbol === "ICP"
      ? {
          default: BigInt(0),
          subaccount: BigInt(0),
        }
      : {
          default: BigInt(0),
        };

  const dispatch = createEventDispatcher();

  async function loadTokenFee() {
    try {
      tokenFee = await IcrcService.getTokenFee(token);
    } catch (error) {
      console.error("Error loading token fee:", error);
      tokenFee = BigInt(10000); // Fallback to default fee
    }
  }

  $: if (token) {
    loadTokenFee();
  }

  $: maxAmount =
    token?.symbol === "ICP"
      ? selectedAccount === "main"
        ? new BigNumber(balances.default.toString())
            .dividedBy(new BigNumber(10).pow(token.decimals))
            .minus(
              new BigNumber(tokenFee?.toString() || "10000").dividedBy(
                new BigNumber(10).pow(token.decimals),
              ),
            )
            .toNumber()
        : new BigNumber(balances.subaccount.toString())
            .dividedBy(new BigNumber(10).pow(token.decimals))
            .minus(
              new BigNumber(tokenFee?.toString() || "10000").dividedBy(
                new BigNumber(10).pow(token.decimals),
              ),
            )
            .toNumber()
      : new BigNumber(balances.default.toString())
          .dividedBy(new BigNumber(10).pow(token.decimals))
          .minus(
            new BigNumber(tokenFee?.toString() || "10000").dividedBy(
              new BigNumber(10).pow(token.decimals),
            ),
          )
          .toNumber();
  let addressType: "principal" | "account" | null = null;
  let showConfirmation = false;

  function isValidHex(str: string): boolean {
    const hexRegex = /^[0-9a-fA-F]+$/;
    return hexRegex.test(str);
  }

  function detectAddressType(address: string): "principal" | "account" | null {
    if (!address) return null;

    // Check for Account ID (64 character hex string)
    if (address.length === 64 && isValidHex(address)) {
      return "account";
    }

    // Check for Principal ID
    try {
      Principal.fromText(address);
      return "principal";
    } catch {
      return null;
    }
  }

  function validateAddress(address: string): boolean {
    if (!address) {
      errorMessage = "Address is required";
      return false;
    }

    const cleanAddress = address.trim();

    if (cleanAddress.length === 0) {
      errorMessage = "Address cannot be empty";
      return false;
    }

    addressType = detectAddressType(cleanAddress);

    if (addressType === "account" && token.symbol !== "ICP") {
      errorMessage = `Account ID can't be used with ${token.name}`;
      return false;
    }

    if (addressType === null) {
      errorMessage = "Invalid address format";
      return false;
    }

    // Clear error message on success
    errorMessage = "";
    return true;
  }

  function validateAmount(value: string): boolean {
    if (!value) {
      errorMessage = "Amount is required";
      return false;
    }
    
    const numValue = parseFloat(value);
    if (isNaN(numValue) || numValue <= 0) {
      errorMessage = "Amount must be greater than 0";
      return false;
    }

    const currentBalance = selectedAccount === "main" ? balances.default : balances.subaccount;
    const maxAmount = new BigNumber(currentBalance.toString())
      .dividedBy(new BigNumber(10).pow(token.decimals))
      .minus(
        new BigNumber(tokenFee?.toString() || "10000").dividedBy(
          new BigNumber(10).pow(token.decimals),
        ),
      )
      .toNumber();

    if (numValue > maxAmount) {
      errorMessage = "Insufficient balance for transfer + fee";
      return false;
    }

    errorMessage = "";
    return true;
  }

  function handleAmountInput(event: Event) {
    const input = event.target as HTMLInputElement;
    let value = input.value.replace(/[^0-9.]/g, "");

    const parts = value.split(".");
    if (parts.length > 2) {
      value = `${parts[0]}.${parts[1]}`;
    }

    if (parts[1]?.length > token.decimals) {
      value = `${parts[0]}.${parts[1].slice(0, token.decimals)}`;
    }

    amount = value;
    validateAmount(value); // Remove errorMessage = "" to let validation set proper message
  }

  async function handleSubmit() {
    dispatch('confirmTransfer', {
      amount,
      token,
      tokenFee,
      isValidating,
      toPrincipal: recipientAddress
    });
  }

  function getAccountIds(
    principalStr: string,
    rawSubaccount: any,
  ): { subaccount: string; main: string } {
    try {
      const principal = Principal.fromText(principalStr);

      // Create account ID with provided subaccount
      const subAccount = convertToSubaccount(rawSubaccount);
      const subaccountId = AccountIdentifier.fromPrincipal({
        principal,
        subAccount,
      }).toHex();

      // Create account ID with main (zero) subaccount
      const mainAccountId = AccountIdentifier.fromPrincipal({
        principal,
        subAccount: undefined,
      }).toHex();

      return {
        subaccount: subaccountId,
        main: mainAccountId,
      };
    } catch (error) {
      console.error("Error creating account identifier:", error);
      return {
        subaccount: "",
        main: "",
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
      } else if (typeof raw === "number") {
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
      console.error("Error converting subaccount:", error);
      return undefined;
    }
  }

  $: if (auth.pnp?.account?.owner) {
    const principal = auth.pnp.account.owner;
    const principalStr =
      typeof principal === "string" ? principal : principal?.toText?.() || "";
    accounts = getAccountIds(principalStr, auth.pnp?.account?.subaccount);
  }

  async function confirmTransfer() {
    isValidating = true;
    errorMessage = "";
    showConfirmation = false;

    try {
      const decimals = token.decimals || 8;
      const amountBigInt = BigInt(new BigNumber(amount).times(new BigNumber(10).pow(decimals)).toString());

      toastStore.info(`Sending ${amount} ${token.symbol}...`);

      const fromSubaccount =
        selectedAccount === "subaccount"
          ? auth.pnp?.account?.subaccount
          : undefined;

      const result = await IcrcService.transfer(
        token,
        recipientAddress,
        amountBigInt,
        {
          fee: BigInt(token.fee_fixed),
          fromSubaccount: fromSubaccount
            ? Array.from(fromSubaccount)
            : undefined,
        },
      );

      if (result?.Ok) {
        toastStore.success(`Successfully sent ${token.symbol}`);
        recipientAddress = "";
        amount = "";
        dispatch('close');
        await loadBalances();
      } else if (result?.Err) {
        const errMsg =
          typeof result.Err === "object"
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
    if (maxAmount <= 0) {
      toastStore.warning(
        "Hmm... Looks like you don't have enough balance for a transfer",
      );
      return;
    }
    amount = maxAmount.toFixed(token.decimals);
    errorMessage = "";
  }

  $: {
    if (recipientAddress) {
      validateAddress(recipientAddress);
    }
    if (amount) {
      validateAmount(amount);
    }
  }

  $: isFormValid = amount && 
                   recipientAddress && 
                   !errorMessage && 
                   addressType !== null && 
                   validateAddress(recipientAddress) && 
                   validateAmount(amount);

  $: validationMessage = (() => {
    if (!recipientAddress)
      return {
        type: "info",
        text: "Enter a Principal ID or Account ID",
      };
    if (errorMessage)
      return {
        type: "error",
        text: errorMessage,
      };
    if (addressType === "principal")
      return {
        type: "success",
        text: "Valid Principal ID",
      };
    if (addressType === "account")
      return {
        type: "success",
        text: "Valid Account ID",
      };
    return {
      type: "error",
      text: "Invalid address format",
    };
  })();

  async function handlePaste() {
    try {
      const text = await navigator.clipboard.readText();
      recipientAddress = text.trim();
    } catch (err) {
      toastStore.error("Failed to paste from clipboard");
    }
  }

  function handleScan(scannedText: string) {
    const cleanedText = scannedText.trim();

    if (validateAddress(cleanedText)) {
      recipientAddress = cleanedText;
      toastStore.success("QR code scanned successfully");
      showScanner = false;
    } else {
      toastStore.error(
        "Invalid QR code. Please scan a valid Principal ID or Account ID",
      );
    }
  }

  async function checkCameraAvailability() {
    try {
      const devices = await navigator.mediaDevices.enumerateDevices();
      hasCamera = devices.some((device) => device.kind === "videoinput");
    } catch (err) {
      console.debug("Error checking camera:", err);
      hasCamera = false;
    }
  }

  onMount(() => {
    checkCameraAvailability();
    document.addEventListener('confirmTransfer', confirmTransfer);
    return () => {
      document.removeEventListener('confirmTransfer', confirmTransfer);
    };
  });

  async function loadBalances() {
    try {
      if (token?.symbol === "ICP") {
        const result: any = await IcrcService.getIcrc1Balance(
          token,
          auth.pnp?.account?.owner,
          auth.pnp?.account?.subaccount
            ? Array.from(auth.pnp.account.subaccount)
            : undefined,
          true,
        );

        balances = {
          default: result.default || BigInt(0),
          subaccount: result.subaccount || BigInt(0),
        };

        // Update token object with both balances
        token = {
          ...token,
          balance: balances.default.toString(),
        };
      } else {
        // For non-ICP tokens, just get the main balance
        const result = await IcrcService.getIcrc1Balance(
          token,
          auth.pnp?.account?.owner,
          undefined,
          false,
        );
        balances = {
          default: typeof result === "bigint" ? result : BigInt(0),
        };
      }
    } catch (error) {
      console.error("Error loading balances:", error);
      balances =
        token?.symbol === "ICP"
          ? { default: BigInt(0), subaccount: BigInt(0) }
          : { default: BigInt(0) };
    }
  }

  $: if (selectedAccount) {
    loadBalances();
  }

  function getTooltipMessage(): string {
    if (!recipientAddress) {
      return "Enter a recipient address";
    }
    if (recipientAddress && !validateAddress(recipientAddress)) {
      return "Invalid address format";
    }
    if (!amount) {
      return "Enter an amount";
    }
    if (amount && !validateAmount(amount)) {
      return "Invalid amount";
    }
    if (errorMessage) {
      return errorMessage;
    }
    return "";
  }
</script>

<div class="container pb-6 px-4">
  <form on:submit|preventDefault={handleSubmit}>
    <div class="id-card mt-2">
      <div class="id-header">
        <span>Recipient Address</span>
        <div class="header-actions">
          <button
            type="button"
            class="utility-button"
            on:click={() => (showScanner = true)}
            title="Scan QR Code"
          >
            <Camera class="w-4 h-4" />
            <span class="button-text text-nowrap">Scan QR</span>
          </button>
          <button
            type="button"
            class="utility-button"
            on:click={recipientAddress
              ? () => (recipientAddress = "")
              : handlePaste}
          >
            {#if recipientAddress}
              ✕
              <span class="button-text">Clear</span>
            {:else}
              <Clipboard class="w-4 h-4" />
              <span class="button-text">Paste</span>
            {/if}
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
            class:valid={addressType === "principal" && !errorMessage}
          />
        </div>

        {#if recipientAddress}
          <div
            class="validation-status"
            class:success={validationMessage.type === "success"}
            class:error={validationMessage.type === "error"}
          >
            <span class="status-text">{validationMessage.text}</span>
          </div>
        {/if}
      </div>
    </div>

    <div class="id-card mt-4">
      <div class="id-header">
        <span>Amount</span>
        <div class="header-actions">
          {#if token.symbol === "ICP" && balances.subaccount && balances.subaccount > BigInt(0)}
            <div class="account-tabs">
              <button
                type="button"
                class="tab-button"
                class:active={selectedAccount === "main"}
                on:click={() => (selectedAccount = "main")}
              >
                Main
              </button>
              <button
                type="button"
                class="tab-button"
                class:active={selectedAccount === "subaccount"}
                on:click={() => (selectedAccount = "subaccount")}
              >
                Sub
              </button>
            </div>
          {/if}
          <button type="button" class="utility-button" on:click={setMaxAmount}
            >Max</button
          >
        </div>
      </div>

      <div class="input-group">
        <div class="input-wrapper">
          <input
            type="text"
            inputmode="decimal"
            placeholder="Enter amount"
            bind:value={amount}
            on:input={handleAmountInput}
            class:error={errorMessage.includes("balance") ||
              errorMessage.includes("Amount")}
          />
          <div class="balance-display">
            Balance: {formatBalance(
              selectedAccount === "main"
                ? balances.default.toString()
                : (balances.subaccount?.toString() ?? "0"),
              token.decimals,
            )}
            {token.symbol}
          </div>
        </div>
      </div>
    </div>

    <button
      type="submit"
      class="send-btn"
      class:disabled={!isFormValid || isValidating}
      class:error={errorMessage}
      disabled={!isFormValid || isValidating}
      use:tooltip={{
        text: getTooltipMessage(),
        direction: "top",
        background: errorMessage ? "bg-kong-accent-red" : "bg-kong-bg-dark"
      }}
    >
      {isValidating ? "Processing..." : "Send Tokens"}
    </button>
  </form>

  {#if showScanner}
    <QrScanner
      isOpen={showScanner}
      onClose={() => (showScanner = false)}
      onScan={handleScan}
    />
  {/if}
</div>

<style scoped lang="postcss">
  .container {
    @apply flex flex-col gap-6;
  }

  .id-card {
    @apply flex flex-col gap-2;
  }

  .id-header {
    @apply flex justify-between items-center text-kong-text-primary/70 text-sm;
  }

  .header-actions {
    @apply flex items-center gap-2;
  }

  .utility-button {
    @apply px-3 rounded-lg py-0.5 
             hover:border-kong-border/20 hover:bg-kong-bg-light
             text-kong-text-primary/70 hover:text-kong-text-primary
             transition-all duration-200
             flex items-center justify-center gap-2;

    .button-text {
      @apply hidden md:inline;
    }

    &:active {
      @apply border-kong-primary bg-kong-primary/20;
    }
  }

  .input-wrapper input {
    @apply w-full h-11 rounded-lg text-kong-text-primary px-4
             bg-kong-bg-light backdrop-blur-sm
             border border-kong-border 
             hover:border-kong-border
             focus:border-kong-primary focus:outline-none
             transition-colors;

    &::placeholder {
      @apply text-kong-text-primary/30;
    }

    &.error {
      @apply border-kong-accent-red/50 bg-kong-accent-red/5;
    }

    &.valid {
      @apply border-kong-accent-green/50 bg-kong-accent-green/5;
    }
  }

  .send-btn {
    @apply h-12 w-full rounded-lg font-medium mt-4
           transition-all duration-200;

    &:not(.disabled):not(.error) {
      @apply bg-kong-primary text-white 
             hover:bg-kong-accent-green;
    }

    &.disabled {
      @apply bg-kong-bg-light text-kong-text-primary/50 
             cursor-not-allowed border border-kong-border/20;
    }

    &.error {
      @apply bg-kong-accent-red text-white
             hover:bg-kong-accent-red/90;
    }
  }

  .validation-status {
    @apply text-sm mt-1 px-1;
    &.success {
      @apply text-kong-text-accent-green;
    }
    &.error {
      @apply text-kong-accent-red;
    }
  }

  .account-tabs {
    @apply flex gap-1 mr-2;
  }

  .tab-button {
    @apply px-3 py-1 rounded-lg text-sm
             bg-kong-bg-light backdrop-blur-sm
             border border-kong-border/10
             text-kong-text-primary/70
             transition-all duration-200;

    &.active {
      @apply bg-kong-primary/20 border-kong-primary text-kong-text-primary;
    }
  }

  .balance-display {
    @apply text-sm text-kong-text-primary/60 mt-2 px-1;
  }
</style>
