<!-- Explorer's Details pane: the selected item's icon, name and properties; the totals of a
     multi-selection; the folder itself when nothing is selected. Lazy-mounted by PaneHost. -->
<script lang="ts">
  import { shortKindOf } from '$lib/utils/short-kind';
  import { useDetailsPane } from '$lib/hooks/use-details-pane.svelte';
  import EntryIcon from '$lib/components/ui/EntryIcon.svelte';
  import FolderIcon from '$lib/components/ui/FolderIcon.svelte';
  import UsageBar from '$lib/components/ui/UsageBar.svelte';
  import Glyph from '$lib/components/ui/Glyph.svelte';
  import DetailsRow from './details-parts/DetailsRow.svelte';
  import { attributesHeading } from '$lib/data/details-rows';
  import AttributeToggle from './details-parts/AttributeToggle.svelte';
  import DecodedThumb from '$lib/components/ui/DecodedThumb.svelte';
  import { wantsDetailsThumb } from '$lib/data/preview-poster';
  import { thumbStamp } from '$lib/utils/thumb-stamp';

  const pane = useDetailsPane();
  // the same short kind the list shows ("Text", not "TXT File"); the shell's long name is the tooltip
  const kind = $derived.by(() => {
    const p = pane.props;
    if (!p) return '';
    const dot = p.name.lastIndexOf('.');
    const ext = !p.isDir && dot > 0 ? p.name.slice(dot + 1).toLowerCase() : null;
    return shortKindOf({ name: p.name, isDir: p.isDir, ext, typeName: p.typeName });
  });
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <header class="flex h-9 shrink-0 items-center justify-end gap-2 pr-1 pl-4 text-sm">
    <span class="text-text-secondary flex-1 truncate">Details</span>
    <button
      type="button"
      class="rounded-control hover:bg-subtle-hover grid size-7 place-items-center"
      title="Close details pane"
      aria-label="Close details pane"
      onclick={pane.close}
    >
      <Glyph name="close" size={10} />
    </button>
  </header>
  <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-auto px-4 pb-4 text-xs" data-mac="pane">
    {#if pane.target === 'many'}
      <div class="flex items-center gap-3" data-mac="info-head">
        <Glyph name="viewDetails" size={32} class="text-text-secondary h-12 w-12" />
        <div class="min-w-0">
          <div class="text-base">{pane.count} items selected</div>
          <div class="text-text-secondary">{pane.manySummary}</div>
        </div>
      </div>
    {:else if pane.drive}
      <div class="flex items-center gap-3" data-mac="info-head">
        <Glyph name="drive" size={40} color="#9aa4b2" class="h-12 w-12" />
        <div class="min-w-0">
          <div class="truncate text-base" title={pane.drive.label}>
            {pane.drive.label || pane.drive.path}
          </div>
          <div class="text-text-secondary truncate">{pane.drive.path}</div>
        </div>
      </div>
      {#if pane.driveFill}
        <div class="flex flex-col gap-1" data-mac="info-usage">
          <UsageBar thick fraction={pane.driveFill.fraction} level={pane.driveFill.level} />
          <span class="text-text-secondary">{pane.driveFill.caption}</span>
        </div>
      {/if}
      <div class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1" data-mac="info-grid">
        {#each pane.driveRows as row (row.key)}
          <DetailsRow label={row.label} value={row.value} />
        {/each}
      </div>
    {:else if pane.props}
      <!-- wraps: a full-width thumbnail puts the name under it (the Mac stacks it anyway) -->
      <div class="flex flex-wrap items-center gap-3" data-mac="info-head">
        {#if pane.icon === 'bin'}
          <Glyph name="delete" size={48} />
        {:else if pane.icon === 'folder'}
          <FolderIcon size={48} />
        {:else if pane.iconEntry && wantsDetailsThumb(pane.iconEntry)}
          <!-- a picture or (Mac) a video: its 256 px thumbnail, as wide as the pane allows -->
          <span class="flex w-full justify-center" data-mac="info-thumb">
            <DecodedThumb
              path={pane.iconEntry.path}
              name={pane.iconEntry.name}
              size={256}
              request={256}
              glyphSize={48}
              fluid
              stamp={thumbStamp(pane.iconEntry)}
              modifiedMs={pane.iconEntry.modifiedMs ?? null}
            />
          </span>
        {:else if pane.iconEntry}
          <span class="grid h-12 w-12 shrink-0 place-items-center">
            <EntryIcon entry={pane.iconEntry} size={32} />
          </span>
        {/if}
        <div class="min-w-0">
          <div class="truncate text-base" title={pane.props.name}>{pane.props.name}</div>
          <div class="text-text-secondary truncate" title={pane.props.typeName}>{kind}</div>
        </div>
      </div>
      <div class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1" data-mac="info-grid">
        {#each pane.rows as row (row.key)}
          <DetailsRow label={row.label} value={row.value} shown={row.shown} />
        {/each}
      </div>
      {#if !pane.inArchive}
        <div class="border-divider flex flex-col gap-1.5 border-t pt-3" data-mac="info-attrs">
          <span class="text-text-secondary">{attributesHeading()}</span>
          {#each pane.toggles as t (t.key)}
            <AttributeToggle
              label={t.label}
              checked={t.checked}
              disabled={pane.writing}
              onchange={(v) => pane.setAttribute(t.key, v)}
            />
          {/each}
        </div>
      {/if}
    {:else if pane.loading}
      <div class="text-text-secondary">Loading…</div>
    {:else if pane.place?.item}
      <!-- one Trash item selected: its name and rows, as a file shows (no attribute boxes) -->
      <div class="flex items-center gap-3" data-mac="info-head">
        <span class="grid h-12 w-12 shrink-0 place-items-center">
          <EntryIcon entry={pane.place.item.entry} size={32} thumbs={false} />
        </span>
        <div class="min-w-0">
          <div class="truncate text-base" title={pane.place.item.name}>{pane.place.item.name}</div>
          <div class="text-text-secondary truncate">{pane.place.label}</div>
        </div>
      </div>
      <!-- a narrow pane (800 px window) stacks each label over its value, so the date is whole -->
      <div class="@container">
        <div
          class="grid grid-cols-1 gap-x-4 @[16rem]:grid-cols-[auto_1fr] @[16rem]:gap-y-1 [&>:nth-child(2n+3)]:mt-1.5 @[16rem]:[&>:nth-child(2n+3)]:mt-0"
          data-mac="info-grid"
        >
          {#each pane.place.item.rows as row (row.key)}
            <DetailsRow label={row.label} value={row.value} shown={row.shown} />
          {/each}
        </div>
      </div>
    {:else if pane.place}
      <div class="flex items-center gap-3" data-mac="info-head">
        {#if pane.place.icon === 'bin'}
          <Glyph name="delete" size={48} />
        {:else}
          <FolderIcon size={48} />
        {/if}
        <div class="truncate text-base">{pane.place.label}</div>
      </div>
      {#if pane.place.summary}
        <div class="text-text-secondary">{pane.place.summary}</div>
      {:else}
        <div class="text-text-secondary">Select an item to see its details.</div>
      {/if}
    {:else}
      <div class="text-text-secondary">Select an item to see its details.</div>
    {/if}
    {#if pane.error}
      <p class="text-critical" role="alert">{pane.error}</p>
    {/if}
  </div>
</div>
