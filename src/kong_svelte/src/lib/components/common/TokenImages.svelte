<script lang="ts">
  import { tooltip as tooltipAction } from "$lib/actions/tooltip";
  import { DEFAULT_LOGOS } from "$lib/services/tokens";

  // Define props using the $props rune
  type TokenImagesProps = {
    tokens: FE.Token[];
    size: number;
    containerClass?: string;
    imageWrapperClass?: string;
    overlap?: boolean;
    tooltip?: {
      text: string;
      direction: "top" | "bottom" | "left" | "right";
    };
  };

  let { 
    tokens = [], 
    size = 48, 
    containerClass = "", 
    imageWrapperClass = "", 
    overlap = false, 
    tooltip = { text: "", direction: "top" as const }
  }: TokenImagesProps = $props();

  const DEFAULT_IMAGE = "/tokens/not_verified.webp";

  // Filter out invalid tokens and memoize result
  const validTokens = $derived(tokens.filter((token): token is FE.Token => {
    return token && typeof token === "object";
  }));

  // Handle image error with proper typing
  function handleImageError(
    e: Event & { currentTarget: EventTarget & HTMLImageElement },
  ) {
    console.error(`Failed to load image: ${e.currentTarget.src}`);
    e.currentTarget.src = DEFAULT_IMAGE;
  }

  // Helper to get token alt text
  function getTokenAlt(token: FE.Token): string {
    return token.symbol ?? token.name ?? "Unknown Token";
  }
</script>

<div
  use:tooltipAction={tooltip}
  class="flex items-center {containerClass} p-0 m-0"
  style="margin-right: {overlap ? '10px' : '0'}"
>
  {#each validTokens as token, index}
    <div
      style="height: {size}px; width: {size}px; z-index: {validTokens.length -
        index};"
      class="flex items-center rounded-full {imageWrapperClass} {overlap
        ? 'mr-[-10px]'
        : ''} relative"
    >
      <img
        class="w-full h-full rounded-full bg-transparent"
        src={token?.logo_url ||
          DEFAULT_LOGOS[token.canister_id] ||
          DEFAULT_IMAGE}
        alt={getTokenAlt(token)}
        loading="eager"
        on:error={handleImageError}
      />
    </div>
  {/each}
</div>
