<script lang="ts">
  /**
   * Badge component for displaying status indicators, tags, or labels
   * 
   * Usage:
   * <Badge>Default</Badge>
   * <Badge variant="blue">Blue</Badge>
   * <Badge variant="green">Green</Badge>
   * <Badge variant="red">Red</Badge>
   * <Badge variant="yellow">Yellow</Badge>
   * <Badge variant="purple">Purple</Badge>
   * <Badge variant="gray">Gray</Badge>
   * <Badge icon="🐋">Whale</Badge>
   * <Badge tooltip="This is a tooltip">With tooltip</Badge>
   */
  import { tooltip } from "$lib/actions/tooltip";

  // Props
  let { 
    variant = "blue", // blue, green, red, yellow, purple, gray
    size = "sm",      // xs, sm, md, lg
    icon = null,      // optional icon to display before text
    pill = true,      // whether to use pill shape (rounded-full) or slightly rounded (rounded)
    tooltip: tooltipText = null, // optional tooltip text
    tooltipDirection = "top" as const, // tooltip direction: top, bottom, left, right
    class: className = "", // additional classes
    children = () => null
  } = $props<{
    variant?: "blue" | "green" | "red" | "yellow" | "purple" | "gray";
    size?: "xs" | "sm" | "md" | "lg";
    icon?: string | null;
    pill?: boolean;
    tooltip?: string | null;
    tooltipDirection?: "top" | "bottom" | "left" | "right";
    class?: string;
    children?: () => any;
  }>();

  // Computed styles based on variant
  const variantStyles = {
    blue: "bg-kong-accent-blue/20 text-kong-accent-blue",
    green: "bg-kong-accent-green/20 text-kong-accent-green",
    red: "bg-kong-accent-red/20 text-kong-accent-red",
    yellow: "bg-kong-accent-yellow/20 text-kong-accent-yellow",
    purple: "bg-kong-accent-purple/20 text-kong-accent-purple",
    gray: "bg-kong-text-secondary/20 text-kong-text-secondary"
  };

  // Size styles
  const sizeStyles = {
    xs: "text-xs px-1 py-0.5 px-2",
    sm: "text-xs px-1.5 py-0.5 px-2",
    md: "text-sm px-2 py-1 px-2",
    lg: "text-sm px-2.5 py-1.5 px-2"
  };

  // Computed classes
  const badgeClasses = `
    inline-flex items-center gap-1 
    ${variantStyles[variant] || variantStyles.blue} 
    ${sizeStyles[size] || sizeStyles.sm} 
    ${pill ? 'rounded-full' : 'rounded'} 
    font-medium
    ${tooltipText ? 'cursor-help' : ''}
    ${className}
  `;
</script>

<span 
  class={badgeClasses}
  use:tooltip={tooltipText ? { text: tooltipText, direction: tooltipDirection } : undefined}
>
  {#if icon}<span class="inline-block">{icon}</span>{/if}
  {@render children()}
</span> 