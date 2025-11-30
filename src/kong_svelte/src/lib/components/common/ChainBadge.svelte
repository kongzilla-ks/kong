<script lang="ts">
  export let chain: 'ICP' | 'Solana' | 'IC' | 'SOL' | string;
  export let size: 'small' | 'medium' | 'large' = 'small';
  export let variant: 'default' | 'minimal' | 'icon-only' | 'glow' = 'minimal';

  // Normalize chain names
  const normalizedChain = chain === 'IC' ? 'ICP' : chain === 'SOL' ? 'Solana' : chain;
  const displayName = chain === 'IC' || chain === 'ICP' ? 'IC' : chain === 'SOL' || chain === 'Solana' ? 'SOL' : chain;
</script>

<span
  class="chain-badge {variant}"
  class:small={size === 'small'}
  class:medium={size === 'medium'}
  class:large={size === 'large'}
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
    font-weight: 700;
    white-space: nowrap;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    letter-spacing: 0.05em;
  }

  /* Minimal variant - sleek modern look */
  .chain-badge.minimal {
    padding: 0.125rem 0.375rem;
    background: var(--badge-bg);
    border: none;
    border-radius: 0.375rem;
    color: var(--badge-text);
  }

  .chain-badge.minimal.small {
    font-size: 0.6rem;
    gap: 0.2rem;
    padding: 0.1rem 0.3rem;
  }

  .chain-badge.minimal.medium {
    font-size: 0.7rem;
    gap: 0.25rem;
  }

  .chain-badge.minimal.large {
    font-size: 0.8rem;
    gap: 0.3rem;
    padding: 0.2rem 0.5rem;
  }

  /* Default variant - prominent badge */
  .chain-badge.default {
    padding: 0.2rem 0.5rem;
    border-radius: 0.5rem;
    background: var(--badge-bg);
    color: var(--badge-text);
    border: 1px solid var(--badge-border, transparent);
    box-shadow: 0 0 8px var(--badge-glow, transparent);
  }

  .chain-badge.default.small {
    font-size: 0.65rem;
    padding: 0.125rem 0.375rem;
  }

  .chain-badge.default.medium {
    font-size: 0.75rem;
    padding: 0.2rem 0.5rem;
  }

  .chain-badge.default.large {
    font-size: 0.85rem;
    padding: 0.25rem 0.625rem;
  }

  /* Glow variant - eye-catching for cross-chain */
  .chain-badge.glow {
    padding: 0.25rem 0.625rem;
    border-radius: 0.5rem;
    background: var(--badge-bg);
    color: var(--badge-text);
    border: 1px solid var(--badge-border);
    box-shadow: 0 0 12px var(--badge-glow), 0 0 24px var(--badge-glow-outer, transparent);
    animation: subtle-pulse 2s ease-in-out infinite;
  }

  .chain-badge.glow.small {
    font-size: 0.65rem;
    padding: 0.15rem 0.4rem;
  }

  .chain-badge.glow.medium {
    font-size: 0.75rem;
    padding: 0.25rem 0.625rem;
  }

  .chain-badge.glow.large {
    font-size: 0.85rem;
    padding: 0.3rem 0.75rem;
  }

  @keyframes subtle-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.85; }
  }

  /* Icon only variant */
  .chain-badge.icon-only {
    padding: 0;
    background: none;
    border: none;
    color: var(--badge-text);
  }

  /* Chain specific colors - Solana */
  .chain-badge.solana {
    --badge-bg: rgba(153, 69, 255, 0.15);
    --badge-text: rgb(180, 130, 255);
    --badge-border: rgba(153, 69, 255, 0.4);
    --badge-glow: rgba(153, 69, 255, 0.3);
    --badge-glow-outer: rgba(153, 69, 255, 0.1);
  }

  /* Chain specific colors - ICP */
  .chain-badge.icp {
    --badge-bg: rgba(41, 171, 226, 0.12);
    --badge-text: rgb(100, 200, 255);
    --badge-border: rgba(41, 171, 226, 0.35);
    --badge-glow: rgba(41, 171, 226, 0.25);
    --badge-glow-outer: rgba(41, 171, 226, 0.08);
  }

  .chain-icon {
    width: 1.1em;
    height: 1.1em;
    flex-shrink: 0;
  }

  .chain-text {
    letter-spacing: 0.05em;
    text-transform: uppercase;
    font-weight: 700;
  }

  /* Hover effects */
  .chain-badge.default:hover,
  .chain-badge.glow:hover {
    transform: translateY(-1px) scale(1.02);
    box-shadow: 0 4px 12px var(--badge-glow), 0 0 20px var(--badge-glow);
  }

  .chain-badge.minimal:hover {
    transform: scale(1.05);
    filter: brightness(1.1);
  }
</style>