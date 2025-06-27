<script lang="ts">
  export let chain: 'ICP' | 'Solana' | 'IC' | 'SOL' | string;
  export let size: 'small' | 'medium' = 'small';
  export let variant: 'default' | 'minimal' | 'icon-only' = 'minimal';
  
  // Normalize chain names
  const normalizedChain = chain === 'IC' ? 'ICP' : chain === 'SOL' ? 'Solana' : chain;
  const displayName = chain === 'IC' || chain === 'ICP' ? 'IC' : chain === 'SOL' || chain === 'Solana' ? 'SOL' : chain;
</script>

<span 
  class="chain-badge {variant}"
  class:small={size === 'small'}
  class:medium={size === 'medium'}
  class:icp={normalizedChain === 'ICP'}
  class:solana={normalizedChain === 'Solana'}
>
  {#if normalizedChain === 'Solana'}
    <svg viewBox="0 0 24 24" fill="currentColor" class="chain-icon">
      <path d="M4.08 7.92C4.2 7.68 4.44 7.56 4.68 7.56H19.2C19.56 7.56 19.8 7.92 19.68 8.16L17.52 12.48C17.4 12.72 17.16 12.84 16.92 12.84H2.4C2.04 12.84 1.8 12.48 1.92 12.24L4.08 7.92ZM4.08 16.08C4.2 15.84 4.44 15.72 4.68 15.72H19.2C19.56 15.72 19.8 16.08 19.68 16.32L17.52 20.64C17.4 20.88 17.16 21 16.92 21H2.4C2.04 21 1.8 20.64 1.92 20.4L4.08 16.08ZM17.52 3.36C17.4 3.12 17.16 3 16.92 3H2.4C2.04 3 1.8 3.36 1.92 3.6L4.08 7.92C4.2 8.16 4.44 8.28 4.68 8.28H19.2C19.56 8.28 19.8 7.92 19.68 7.68L17.52 3.36Z"/>
    </svg>
    {#if variant !== 'icon-only'}
      <span class="chain-text">{displayName}</span>
    {/if}
  {:else if normalizedChain === 'ICP'}
    <svg viewBox="0 0 24 24" fill="none" class="chain-icon">
      <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2"/>
      <circle cx="8" cy="12" r="2" fill="currentColor"/>
      <circle cx="16" cy="12" r="2" fill="currentColor"/>
      <path d="M8 12C8 9.79086 9.79086 8 12 8C14.2091 8 16 9.79086 16 12C16 14.2091 14.2091 16 12 16C9.79086 16 8 14.2091 8 12Z" stroke="currentColor" stroke-width="2"/>
    </svg>
    {#if variant !== 'icon-only'}
      <span class="chain-text">{displayName}</span>
    {/if}
  {:else}
    {displayName}
  {/if}
</span>

<style>
  .chain-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-weight: 600;
    white-space: nowrap;
    transition: all 0.2s ease;
  }

  /* Minimal variant - sleek modern look */
  .chain-badge.minimal {
    padding: 0;
    background: none;
    border: none;
    color: var(--kong-text-secondary);
  }

  .chain-badge.minimal.small {
    font-size: 0.625rem;
    gap: 0.125rem;
  }

  .chain-badge.minimal.medium {
    font-size: 0.75rem;
    gap: 0.1875rem;
  }

  /* Default variant - subtle badge */
  .chain-badge.default {
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    background-color: var(--badge-bg);
    color: var(--badge-text);
    border: 1px solid transparent;
  }

  .chain-badge.default.small {
    font-size: 0.625rem;
    padding: 0.0625rem 0.25rem;
  }

  .chain-badge.default.medium {
    font-size: 0.75rem;
    padding: 0.125rem 0.375rem;
  }

  /* Icon only variant */
  .chain-badge.icon-only {
    padding: 0;
    background: none;
    border: none;
    color: var(--kong-text-secondary);
  }

  /* Chain specific colors */
  .chain-badge.icp {
    --badge-bg: rgba(237, 49, 165, 0.08);
    --badge-text: rgb(237, 49, 165);
  }

  .chain-badge.solana {
    --badge-bg: rgba(20, 241, 149, 0.08);
    --badge-text: rgb(20, 241, 149);
  }

  .chain-badge.minimal.icp {
    color: rgb(237, 49, 165);
  }

  .chain-badge.minimal.solana {
    color: rgb(20, 241, 149);
  }

  .chain-icon {
    width: 1em;
    height: 1em;
    flex-shrink: 0;
  }

  .chain-text {
    letter-spacing: 0.025em;
    text-transform: uppercase;
  }

  /* Hover effects for default variant */
  .chain-badge.default:hover {
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  /* Dark mode support */
  :global(.dark) .chain-badge.default {
    --badge-bg: rgba(255, 255, 255, 0.05);
    --badge-text: rgba(255, 255, 255, 0.9);
  }

  :global(.dark) .chain-badge.default.icp {
    --badge-bg: rgba(237, 49, 165, 0.15);
    --badge-text: rgb(250, 100, 200);
  }

  :global(.dark) .chain-badge.default.solana {
    --badge-bg: rgba(20, 241, 149, 0.15);
    --badge-text: rgb(100, 255, 200);
  }

  :global(.dark) .chain-badge.minimal {
    color: var(--kong-text-secondary);
  }

  :global(.dark) .chain-badge.minimal.icp {
    color: rgb(250, 100, 200);
  }

  :global(.dark) .chain-badge.minimal.solana {
    color: rgb(100, 255, 200);
  }
</style>