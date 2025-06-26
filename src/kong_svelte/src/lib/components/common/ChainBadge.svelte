<script lang="ts">
  export let chain: 'ICP' | 'Solana' | string;
  export let size: 'small' | 'medium' = 'small';
</script>

<span 
  class="chain-badge"
  class:small={size === 'small'}
  class:medium={size === 'medium'}
  class:icp={chain === 'ICP'}
  class:solana={chain === 'Solana'}
>
  {#if chain === 'Solana'}
    <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" class="chain-icon">
      <path d="M4.08 7.92C4.2 7.68 4.44 7.56 4.68 7.56H19.2C19.56 7.56 19.8 7.92 19.68 8.16L17.52 12.48C17.4 12.72 17.16 12.84 16.92 12.84H2.4C2.04 12.84 1.8 12.48 1.92 12.24L4.08 7.92ZM4.08 16.08C4.2 15.84 4.44 15.72 4.68 15.72H19.2C19.56 15.72 19.8 16.08 19.68 16.32L17.52 20.64C17.4 20.88 17.16 21 16.92 21H2.4C2.04 21 1.8 20.64 1.92 20.4L4.08 16.08ZM17.52 3.36C17.4 3.12 17.16 3 16.92 3H2.4C2.04 3 1.8 3.36 1.92 3.6L4.08 7.92C4.2 8.16 4.44 8.28 4.68 8.28H19.2C19.56 8.28 19.8 7.92 19.68 7.68L17.52 3.36Z"/>
    </svg>
    SOL
  {:else if chain === 'ICP'}
    <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" class="chain-icon">
      <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm-2-8c0 1.1.9 2 2 2s2-.9 2-2-.9-2-2-2-2 .9-2 2z"/>
    </svg>
    ICP
  {:else}
    {chain}
  {/if}
</span>

<style>
  .chain-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.125rem 0.5rem;
    border-radius: 9999px;
    font-weight: 500;
    white-space: nowrap;
    background-color: var(--badge-bg, rgba(0, 0, 0, 0.05));
    color: var(--badge-text, rgba(0, 0, 0, 0.7));
    border: 1px solid var(--badge-border, rgba(0, 0, 0, 0.1));
  }

  .chain-badge.small {
    font-size: 0.625rem;
    padding: 0.125rem 0.375rem;
  }

  .chain-badge.medium {
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
  }

  .chain-badge.icp {
    --badge-bg: rgba(237, 49, 165, 0.1);
    --badge-text: rgb(237, 49, 165);
    --badge-border: rgba(237, 49, 165, 0.2);
  }

  .chain-badge.solana {
    --badge-bg: rgba(0, 255, 163, 0.1);
    --badge-text: rgb(0, 200, 130);
    --badge-border: rgba(0, 255, 163, 0.2);
  }

  .chain-icon {
    width: 1em;
    height: 1em;
  }

  /* Dark mode support */
  :global(.dark) .chain-badge {
    --badge-bg: rgba(255, 255, 255, 0.1);
    --badge-text: rgba(255, 255, 255, 0.9);
    --badge-border: rgba(255, 255, 255, 0.2);
  }

  :global(.dark) .chain-badge.icp {
    --badge-bg: rgba(237, 49, 165, 0.2);
    --badge-text: rgb(250, 100, 200);
    --badge-border: rgba(237, 49, 165, 0.3);
  }

  :global(.dark) .chain-badge.solana {
    --badge-bg: rgba(0, 255, 163, 0.2);
    --badge-text: rgb(100, 255, 200);
    --badge-border: rgba(0, 255, 163, 0.3);
  }
</style>