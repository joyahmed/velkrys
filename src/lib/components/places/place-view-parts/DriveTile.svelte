<!-- One drive in This PC: icon, label, capacity bar (red past 90 %), "x free of y" (neither when the size is unknown). -->
<script lang="ts">
  import type { PlaceTile } from '$lib/data/place-tiles';
  import Glyph from '$lib/components/ui/Glyph.svelte';
  import UsageBar from '$lib/components/ui/UsageBar.svelte';
  import DriveRenameBox from '$lib/components/ui/DriveRenameBox.svelte';
  import { driveRename } from '$lib/state/drive-rename.svelte';

  interface Props {
    tile: Extract<PlaceTile, { kind: 'drive' }>;
    /** 0 for the view's one Tab stop, -1 for the rest (the arrow keys reach them) */
    tabindex: number;
    onopen: () => void;
    /** the sidebar's place menu (`place-tile-menu`) */
    oncontextmenu?: (e: MouseEvent) => void;
  }
  let { tile, tabindex, onopen, oncontextmenu }: Props = $props();

  const renaming = $derived(driveRename.isTarget(tile.path, 'tile'));

  // Enter or Space opens; Shift+F10 / the Menu key ask for the context menu, as on a file row —
  // sent as a `contextmenu` event at the tile so whatever menu is wired up above gets it.
  const onkeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onopen();
    } else if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) {
      e.preventDefault();
      const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
      e.currentTarget?.dispatchEvent(
        new MouseEvent('contextmenu', {
          bubbles: true,
          cancelable: true,
          clientX: r.left + r.width / 2,
          clientY: r.top + r.height / 2,
        }),
      );
    }
  };
</script>

<div
  class="rounded-card hover:bg-subtle-hover active:bg-subtle-pressed flex w-[264px] items-center gap-3 px-2 py-2"
  role="button"
  {tabindex}
  data-place-tile={tile.key}
  title={tile.label}
  ondblclick={onopen}
  {oncontextmenu}
  {onkeydown}
>
  <Glyph name="drive" size={40} color="#9aa4b2" class="h-12 w-12" />
  <div class="flex min-w-0 flex-1 flex-col gap-1">
    {#if renaming}<DriveRenameBox />{:else}<span class="truncate text-sm">{tile.label}</span>{/if}
    {#if tile.usedFraction !== null}
      <UsageBar thick fraction={tile.usedFraction} level={tile.critical ? 'critical' : 'ok'} />
    {/if}
    {#if tile.caption !== null}
      <span class="text-text-secondary truncate">{tile.caption}</span>
    {/if}
  </div>
</div>
