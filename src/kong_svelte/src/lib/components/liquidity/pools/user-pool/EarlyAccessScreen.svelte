<script lang="ts">
  import { fade } from "svelte/transition";
  import { Rocket, Lock, Calendar, Users } from "lucide-svelte";
  import ButtonV2 from "$lib/components/common/ButtonV2.svelte";

  interface Props {
    onBack?: () => void;
  }

  let { onBack }: Props = $props();
</script>

<div class="w-full h-full flex items-center justify-center p-4" transition:fade={{ duration: 200 }}>
  <div class="max-w-md w-full">
    <div class="bg-kong-bg-secondary rounded-2xl border border-kong-border shadow-lg p-8 text-center">
      <!-- Icon -->
      <div class="flex justify-center mb-6">
        <div class="relative">
          <div class="w-20 h-20 bg-kong-primary/10 rounded-full flex items-center justify-center">
            <Rocket class="w-10 h-10 text-kong-primary" />
          </div>
          <div class="absolute -bottom-1 -right-1 w-8 h-8 bg-kong-bg-secondary rounded-full flex items-center justify-center border-2 border-kong-border">
            <Lock class="w-4 h-4 text-kong-text-secondary" />
          </div>
        </div>
      </div>

      <!-- Title -->
      <h2 class="text-2xl font-bold text-kong-text-primary mb-3">
        Early Access Only
      </h2>

      <!-- Description -->
      <p class="text-kong-text-secondary mb-6 leading-relaxed">
        Adding liquidity is not available in early access at this moment. 
        Please check again later.
      </p>

      <!-- Features list -->
      <div class="space-y-3 mb-8">
        <div class="flex items-center gap-3 text-left bg-kong-bg-tertiary rounded-lg p-3">
          <Calendar class="w-5 h-5 text-kong-primary flex-shrink-0" />
          <span class="text-sm text-kong-text-secondary">
            Public launch coming soon
          </span>
        </div>
        <div class="flex items-center gap-3 text-left bg-kong-bg-tertiary rounded-lg p-3">
          <Users class="w-5 h-5 text-kong-primary flex-shrink-0" />
          <span class="text-sm text-kong-text-secondary">
            Join our community for early access opportunities
          </span>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex flex-col gap-3">
        <a 
          href="https://x.com/kongswapx" 
          target="_blank" 
          rel="noopener noreferrer"
          class="w-full"
        >
          <ButtonV2 
            variant="primary" 
            size="lg"
            class="w-full"
          >
            Follow on X
          </ButtonV2>
        </a>
        
        {#if onBack}
          <ButtonV2 
            variant="secondary" 
            size="lg"
            onclick={onBack}
            class="w-full"
          >
            Back to Pools
          </ButtonV2>
        {/if}
      </div>
    </div>

    <!-- Additional info -->
    <p class="text-xs text-kong-text-secondary text-center mt-4">
      You can still view pool information and track your positions
    </p>
  </div>
</div>

<style>
  /* Add any custom styles if needed */
</style>