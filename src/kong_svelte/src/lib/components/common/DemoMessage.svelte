<script lang="ts">
  import { goto } from "$app/navigation";
  import ButtonV2 from "$lib/components/common/ButtonV2.svelte";
  import { onMount, onDestroy } from "svelte";
  
  let { feature = "feature" } = $props();
  
  // Countdown timer state
  let timeLeft = $state({ days: 0, hours: 0, minutes: 0, seconds: 0 });
  let countdownInterval: number | null = null;
  
  // Launch date: July 14, 2025
  const launchDate = new Date('2025-07-14T00:00:00Z');
  
  function updateCountdown() {
    const now = new Date();
    const difference = launchDate.getTime() - now.getTime();
    
    if (difference > 0) {
      timeLeft = {
        days: Math.floor(difference / (1000 * 60 * 60 * 24)),
        hours: Math.floor((difference % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60)),
        minutes: Math.floor((difference % (1000 * 60 * 60)) / (1000 * 60)),
        seconds: Math.floor((difference % (1000 * 60)) / 1000)
      };
    } else {
      timeLeft = { days: 0, hours: 0, minutes: 0, seconds: 0 };
    }
  }
  
  onMount(() => {
    updateCountdown();
    countdownInterval = setInterval(updateCountdown, 1000);
  });
  
  onDestroy(() => {
    if (countdownInterval) clearInterval(countdownInterval);
  });
</script>

<section class="flex-1 flex items-center justify-center relative min-h-[calc(100vh-var(--navbar-height))]">
  
  <div class="flex flex-col items-center justify-center text-center px-6 relative z-10 py-8">
    <!-- Large header text with animated effect -->
    <div class="relative mb-8">
      <div class="text-5xl md:text-6xl font-bold bg-gradient-to-r from-kong-primary via-kong-info to-kong-secondary bg-clip-text text-transparent relative animate-float leading-tight">
        Early Access
      </div>
    </div>
    
    <!-- Feature name -->
    <h2 class="text-lg md:text-xl font-medium mb-6 text-kong-text-secondary">
      {feature} is live on the main platform
    </h2>
    
    <!-- Description -->
    <p class="mb-6 max-w-lg text-kong-text-secondary/80 leading-relaxed">
      This early access environment is for testing DeFi swaps only.
      For {feature}, please visit the live platform at kongswap.io
      where all features are fully operational.
    </p>
    
    <!-- Countdown Timer -->
    <div class="mb-10">
      <p class="text-sm text-kong-text-secondary mb-3">Multi-chain DeFi launches in:</p>
      <div class="flex gap-4 justify-center">
        <div class="countdown-unit">
          <div class="countdown-value">{timeLeft.days}</div>
          <div class="countdown-label">Days</div>
        </div>
        <div class="countdown-unit">
          <div class="countdown-value">{timeLeft.hours.toString().padStart(2, '0')}</div>
          <div class="countdown-label">Hours</div>
        </div>
        <div class="countdown-unit">
          <div class="countdown-value">{timeLeft.minutes.toString().padStart(2, '0')}</div>
          <div class="countdown-label">Minutes</div>
        </div>
        <div class="countdown-unit">
          <div class="countdown-value">{timeLeft.seconds.toString().padStart(2, '0')}</div>
          <div class="countdown-label">Seconds</div>
        </div>
      </div>
    </div>
    
    <!-- Action buttons -->
    <div class="flex flex-col sm:flex-row gap-4 items-center">
      <ButtonV2
        label="Visit KongSwap.io"
        theme="primary"
        variant="solid"
        size="lg"
        onclick={() => window.open('https://www.kongswap.io', '_blank')}
        animationIterations={3}
      />
      
      <button
        onclick={() => goto("/")}
        class="try-demo-btn relative px-8 py-4 text-lg font-bold text-kong-primary border-2 border-kong-primary rounded-xl overflow-hidden group transition-all duration-300 hover:text-white hover:shadow-2xl hover:shadow-kong-primary/50 hover:-translate-y-1"
      >
        <span class="relative z-10 flex items-center gap-2">
          Try Demo Now
          <svg class="w-5 h-5 animate-bounce-x" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7l5 5m0 0l-5 5m5-5H6" />
          </svg>
        </span>
        <div class="absolute inset-0 bg-kong-primary transform scale-x-0 group-hover:scale-x-100 transition-transform duration-300 origin-left"></div>
        
        <!-- Animated particles -->
        <div class="particles">
          <span class="particle"></span>
          <span class="particle"></span>
          <span class="particle"></span>
          <span class="particle"></span>
        </div>
      </button>
    </div>
    
    <!-- Additional help text -->
    <div class="mt-12 text-sm text-kong-text-secondary/60">
      Want to explore more features? 
      <a 
        href="https://www.kongswap.io" 
        target="_blank" 
        rel="noopener noreferrer"
        class="text-kong-primary hover:text-kong-primary/80 transition-colors underline underline-offset-2"
      >
        Visit the full platform
      </a>
    </div>
  </div>
</section>

<style>
  @keyframes float {
    0%, 100% {
      transform: translateY(0) scale(1);
    }
    50% {
      transform: translateY(-10px) scale(1.02);
    }
  }
  
  @keyframes bounce-x {
    0%, 100% {
      transform: translateX(0);
    }
    50% {
      transform: translateX(5px);
    }
  }
  
  .animate-float {
    animation: float 3s ease-in-out infinite;
  }
  
  .animate-bounce-x {
    animation: bounce-x 1s ease-in-out infinite;
  }
  
  /* Try Demo button special effects */
  .try-demo-btn {
    position: relative;
    transform-style: preserve-3d;
    perspective: 1000px;
  }
  
  .try-demo-btn::before {
    content: '';
    position: absolute;
    top: -2px;
    left: -2px;
    right: -2px;
    bottom: -2px;
    background: linear-gradient(45deg, 
      rgb(var(--brand-primary)), 
      rgb(var(--brand-secondary)), 
      rgb(var(--brand-primary)),
      rgb(var(--brand-info))
    );
    background-size: 400% 400%;
    border-radius: inherit;
    z-index: -1;
    opacity: 0;
    transition: opacity 0.3s;
    animation: gradient-shift 3s ease infinite;
  }
  
  .try-demo-btn:hover::before {
    opacity: 1;
  }
  
  @keyframes gradient-shift {
    0% { background-position: 0% 50%; }
    50% { background-position: 100% 50%; }
    100% { background-position: 0% 50%; }
  }
  
  /* Particle effects */
  .particles {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  
  .particle {
    position: absolute;
    width: 4px;
    height: 4px;
    background: rgb(var(--brand-primary));
    border-radius: 50%;
    opacity: 0;
  }
  
  .try-demo-btn:hover .particle {
    animation: particle-float 2s infinite;
  }
  
  .particle:nth-child(1) {
    top: 20%;
    left: 10%;
    animation-delay: 0s;
  }
  
  .particle:nth-child(2) {
    top: 80%;
    left: 20%;
    animation-delay: 0.5s;
  }
  
  .particle:nth-child(3) {
    top: 40%;
    left: 80%;
    animation-delay: 1s;
  }
  
  .particle:nth-child(4) {
    top: 70%;
    left: 90%;
    animation-delay: 1.5s;
  }
  
  @keyframes particle-float {
    0% {
      opacity: 0;
      transform: translateY(0) scale(0);
    }
    20% {
      opacity: 1;
      transform: translateY(-20px) scale(1);
    }
    100% {
      opacity: 0;
      transform: translateY(-60px) translateX(20px) scale(0);
    }
  }
  
  /* Additional animations for other elements */
  @keyframes fadeInUp {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  
  h2, p {
    animation: fadeInUp 0.8s ease-out forwards;
    opacity: 0;
  }
  
  h2 {
    animation-delay: 0.2s;
  }
  
  p {
    animation-delay: 0.4s;
  }
  
  .flex.gap-4 {
    animation: fadeInUp 0.8s ease-out forwards;
    animation-delay: 0.6s;
    opacity: 0;
  }
  
  /* Countdown timer styles */
  .countdown-unit {
    background: rgba(var(--brand-primary) / 0.1);
    border: 1px solid rgba(var(--brand-primary) / 0.3);
    border-radius: 12px;
    padding: 1rem 1.5rem;
    min-width: 80px;
    transition: all 0.3s ease;
  }
  
  .countdown-unit:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(var(--brand-primary) / 0.2);
    background: rgba(var(--brand-primary) / 0.15);
  }
  
  .countdown-value {
    font-size: 2rem;
    font-weight: 700;
    color: rgb(var(--brand-primary));
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }
  
  .countdown-label {
    font-size: 0.75rem;
    color: rgb(var(--text-secondary));
    margin-top: 0.25rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  
  @media (max-width: 640px) {
    .countdown-unit {
      padding: 0.75rem 1rem;
      min-width: 60px;
    }
    
    .countdown-value {
      font-size: 1.5rem;
    }
  }
</style>