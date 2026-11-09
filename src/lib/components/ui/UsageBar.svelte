<!-- How full a drive is: one bar for the sidebar row (thin), the This PC tile and the Details pane
     (thick). Neutral until the drive is getting full; amber when tight, red when full. -->
<script lang="ts">
  import type { UsageLevel } from '$lib/data/sidebar-sections';

  interface Props {
    /** 0..1, how much is used */
    fraction: number;
    level: UsageLevel;
    /** the sidebar's hairline (default) or the tile and pane's chunky bar */
    thick?: boolean;
  }
  let { fraction, level, thick = false }: Props = $props();

  const FILL: Record<UsageLevel, string> = {
    ok: 'bg-text-secondary/55',
    low: 'bg-caution',
    critical: 'bg-critical',
  };
  // the chunky bar's ordinary fill is the accent, as This PC always drew it
  const fill = $derived(thick && level === 'ok' ? 'bg-accent' : FILL[level]);
</script>

{#if thick}
  <div class="bg-subtle-selected h-2 w-full overflow-hidden rounded-xs">
    <div class="h-full {fill}" style:width="{fraction * 100}%"></div>
  </div>
{:else}
  <span class="bg-text-secondary/15 mt-[3px] block h-[2px] overflow-hidden rounded-full">
    <span
      class="block h-full rounded-full {fill}"
      style:width="{Math.max(2, Math.round(fraction * 100))}%"
    ></span>
  </span>
{/if}
