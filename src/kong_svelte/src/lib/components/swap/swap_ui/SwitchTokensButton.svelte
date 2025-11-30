<script lang="ts">
  import { themeStore } from "$lib/stores/themeStore";
  import { getThemeById } from "$lib/themes/themeRegistry";
  import { onMount } from 'svelte';

  // Theme-specific styling
  let theme = $derived(getThemeById($themeStore));
  let backgroundColor = $derived(theme.colors.switchButtonBg || theme.colors.bgSecondary || '#1c2333');
  let hoverBackgroundColor = $derived(theme.colors.switchButtonHoverBg || theme.colors.bgPrimary || '#252b3d');
  let borderColor = $derived(theme.colors.switchButtonBorder || theme.colors.borderLight || 'rgba(255, 255, 255, 0.1)');
  let buttonShadow = $derived(theme.colors.switchButtonShadow || '0 8px 32px rgba(0, 0, 0, 0.32)');
  let ismicroswapTheme = $derived(theme.id === 'microswap');

  // Props
  let {
    isDisabled = false,
    debounceTime = 250,
    onSwitch = () => {} // Default no-op function
  } = $props<{
    isDisabled?: boolean;
    debounceTime?: number;
    onSwitch?: () => void;
  }>();

  // Internal state
  let debouncing = false;
  let isHovered = $state(false);
  let isPressed = $state(false);
  let buttonElement;

  // Memoize button styles to avoid recalculation on each render
  let buttonStyle = $derived(`
    background: ${isHovered ? hoverBackgroundColor : backgroundColor};
    ${!ismicroswapTheme ? `border-color: ${isHovered ? borderColor : borderColor};` :
    `border-style: solid; border-width: 2px; border-color: ${borderColor};`}
    ${ismicroswapTheme ? `box-shadow: ${buttonShadow};` : ''}
    color: ${theme.colors.textPrimary || 'white'};
  `);

  // Memoize disabled state classes
  let disabledClasses = $derived(isDisabled ? 'opacity-50 cursor-not-allowed' : '');

  // Optimized click handler with debounce
  function handleClick() {
    if (isDisabled || debouncing) return;

    // Set debounce flag to prevent rapid clicks
    debouncing = true;
    setTimeout(() => {
      debouncing = false;
    }, debounceTime);

    // Call the onSwitch callback
    onSwitch();
  }

  function handleMouseDown() {
    if (!isDisabled) isPressed = true;
  }

  function handleMouseUp() {
    isPressed = false;
  }

  // Use passive event listeners for better performance
  onMount(() => {
    if (buttonElement) {
      const handleMouseEnter = () => isHovered = true;
      const handleMouseLeave = () => { isHovered = false; isPressed = false; };

      buttonElement.addEventListener('mouseenter', handleMouseEnter, { passive: true });
      buttonElement.addEventListener('mouseleave', handleMouseLeave, { passive: true });

      return () => {
        buttonElement.removeEventListener('mouseenter', handleMouseEnter);
        buttonElement.removeEventListener('mouseleave', handleMouseLeave);
      };
    }
  });

  // Simple arrow swap icon paths
  const arrowDown = "M12 5v14M5 12l7 7 7-7";
  const arrowUp = "M12 19V5M19 12l-7-7-7 7";
</script>

<button
  bind:this={buttonElement}
  class="switch-tokens-button w-11 h-11 rounded-full border shadow-lg {ismicroswapTheme ? 'rounded-none win98-button' : ''} {disabledClasses}"
  class:pressed={isPressed}
  class:hovered={isHovered}
  style={buttonStyle}
  onclick={handleClick}
  onmousedown={handleMouseDown}
  onmouseup={handleMouseUp}
  disabled={isDisabled}
  aria-label="Switch tokens position"
>
  <div class="icon-container">
    <svg
      class="w-5 h-5"
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <path
        d="M7.5 3.5L4.5 6.5L7.5 9.5M4.5 6.5H16.5C18.71 6.5 20.5 8.29 20.5 10.5C20.5 11.48 20.14 12.37 19.55 13.05M16.5 20.5L19.5 17.5L16.5 14.5M19.5 17.5H7.5C5.29 17.5 3.5 15.71 3.5 13.5C3.5 12.52 3.86 11.63 4.45 10.95"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
  </div>
</button>

<style>
  .switch-tokens-button {
    cursor: pointer;
    transition: all 0.15s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
  }

  .icon-container {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    transition: transform 0.1s ease;
  }

  /* Hover state - subtle lift */
  .switch-tokens-button.hovered {
    transform: scale(1.08);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
  }

  /* Pressed state - satisfying click */
  .switch-tokens-button.pressed {
    transform: scale(0.92);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
  }

  .switch-tokens-button.pressed .icon-container {
    transform: scale(0.9);
  }

  /* Subtle inner glow on hover */
  .switch-tokens-button::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: radial-gradient(circle at 30% 30%, rgba(255, 255, 255, 0.15) 0%, transparent 60%);
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .switch-tokens-button.hovered::after {
    opacity: 1;
  }

  /* Win98 specific styles */
  .win98-button {
    transition: all 0.05s ease-out;
  }

  .win98-button.pressed {
    box-shadow: inset -1px -1px 0 #FFFFFF, inset 1px 1px 0 #808080 !important;
    transform: translateY(1px) !important;
  }

  .win98-button::after {
    display: none;
  }
</style>
