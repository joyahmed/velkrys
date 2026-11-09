// The place view's reactivity: which sections the active tab's place shows, opening a tile in
// the active tab (a file with its app), F5 to reload the places and Home's recent files.
// A right-click on a folder or drive tile opens the sidebar's place menu (`place-tile-menu`).
// The tiles are one roving tab stop: Tab lands on one, the arrow keys, Home and End move between
// them (`utils/tile-keys`).
// Returns getters so the component stays live.
import { openTerminal } from '$lib/api/shell';
import { placeTileMenuFor } from '$lib/data/place-tile-menu';
import { DRIVE_NOT_RENAMEABLE, renameRefusal } from '$lib/data/drive-menu';
import { placeSections, type PlaceSection, type PlaceTile } from '$lib/data/place-tiles';
import { useMenu, type MenuItem } from '$lib/hooks/use-menu.svelte';
import { actions } from '$lib/state/actions.svelte';
import { driveRename } from '$lib/state/drive-rename.svelte';
import { places } from '$lib/state/places.svelte';
import { recent } from '$lib/state/recent.svelte';
import { tabs } from '$lib/state/tabs.svelte';
import { heldByDialog } from '$lib/state/modal.svelte';
import { locationLabel } from '$lib/utils/location';
import { rovingKey, tileMove } from '$lib/utils/tile-keys';

/** The attribute every tile carries (its key), so the view can find them in screen order. */
export const TILE_ATTR = 'data-place-tile';

export const usePlaceView = () => {
  const title = $derived(locationLabel(tabs.active.location));
  const sections = $derived<PlaceSection[]>(
    tabs.active.location.kind === 'path' || tabs.active.location.kind === 'archive'
      ? []
      : placeSections[tabs.active.location.kind],
  );

  // Home lists Windows' recent files: fetched each time Home shows (and on F5), never at
  // first paint of a folder.
  $effect(() => {
    if (tabs.active.location.kind === 'home') void recent.loadFiles();
  });

  const open = (tile: PlaceTile) =>
    tile.kind === 'file' ? void actions.open(tile.entry) : void tabs.active.navigate(tile.path);

  const menu = useMenu();
  let items = $state<MenuItem[]>([]);

  const openTerminalAt = async (path: string) => {
    try {
      await openTerminal(path);
    } catch (e) {
      actions.notify(String(e), 'error');
    }
  };

  /** The sidebar's menu for the place the tile stands for; a file tile has none. */
  const oncontextmenu = (tile: PlaceTile, e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const built = placeTileMenuFor(tile, { openTerminal: openTerminalAt });
    if (built.length === 0) return;
    items = built;
    menu.openAt(e.clientX, e.clientY);
  };

  // the tile that holds the one Tab stop: the last one focused, while it is still shown
  let remembered = $state<string | null>(null);
  const tabStop = $derived(
    rovingKey(
      sections.flatMap((s) => s.tiles().map((t) => t.key)),
      remembered,
    ),
  );
  const tabindexFor = (key: string) => (key === tabStop ? 0 : -1);

  /** Focus that lands on a tile (click, Tab, an arrow) makes it the Tab stop. */
  const onfocusin = (e: FocusEvent) => {
    const key = (e.target as HTMLElement | null)?.getAttribute?.(TILE_ATTR);
    if (key) remembered = key;
  };

  const onkeydown = (e: KeyboardEvent) => {
    if (heldByDialog(e)) return;
    if (e.key === 'F5') {
      e.preventDefault();
      void tabs.active.refresh();
      return;
    }
    // F2 on a drive tile edits its label in place, as Explorer's This PC does (the menu's Rename)
    if (e.key === 'F2' && !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey) {
      const key = (e.target as HTMLElement | null)?.getAttribute?.(TILE_ATTR);
      const tile = sections.flatMap((s) => s.tiles()).find((t) => t.key === key);
      const drive = tile?.kind === 'drive' ? places.driveAt(tile.path) : undefined;
      if (!drive) return;
      e.preventDefault();
      // the menu offers no Rename here either (network, optical, a root that cannot be relabelled)
      if (drive.canRename !== true) return void actions.notify(DRIVE_NOT_RENAMEABLE);
      const why = renameRefusal(drive);
      if (why !== null) actions.notify(why);
      else driveRename.start(drive, 'tile');
      return;
    }
    const group = e.currentTarget as HTMLElement;
    const tiles = [...group.querySelectorAll<HTMLElement>(`[${TILE_ATTR}]`)];
    const from = tiles.indexOf(e.target as HTMLElement);
    const to = tileMove(
      e,
      from,
      tiles.map((t) => t.getBoundingClientRect()),
    );
    if (to === null) return;
    e.preventDefault();
    tiles[to].focus();
  };

  return {
    get title() {
      return title;
    },
    /** which place shows; Gallery is its own view, the rest are tile sections */
    get kind() {
      return tabs.active.location.kind;
    },
    get sections() {
      return sections;
    },
    /** a typed path that failed while a place was showing */
    get error() {
      return tabs.active.error;
    },
    open,
    onkeydown,
    onfocusin,
    tabindexFor,
    menu,
    get items() {
      return items;
    },
    oncontextmenu,
  };
};
