// The sidebar's right-click menu, as data, built for one item: Open in new tab, Pin / Unpin
// (folders, and Home, Gallery, OneDrive, the Bin), Open in Terminal (folders). The handlers that reach outside the state come in
// from the hook, so this file stays data + state reads.
import type { MenuItem } from '$lib/hooks/use-menu.svelte';
import { driveMenuItems, type DriveMenuHandlers, type DriveSurface } from '$lib/data/drive-menu';
import { driveMenuHandlers } from '$lib/state/drive-menu-handlers';
import { places } from '$lib/state/places.svelte';
import { knownKeyOf, type SidebarItem } from '$lib/data/sidebar-sections';
import { isSidebarKey, settings } from '$lib/state/settings.svelte';
import { tabs } from '$lib/state/tabs.svelte';
import { openWindow } from '$lib/api/window';
import { locationText } from '$lib/utils/location';

export interface SidebarMenuHandlers {
  openTerminal: (path: string) => unknown;
  /** what the drive rows do; the real ones unless a test hands its own */
  drive?: DriveMenuHandlers;
}

const folderPath = (item: SidebarItem) =>
  item.location?.kind === 'path' && item.location.path ? item.location.path : null;

export const sidebarMenuFor = (
  item: SidebarItem,
  on: SidebarMenuHandlers,
  surface: Exclude<DriveSurface, 'list'> = 'sidebar',
): MenuItem[] => {
  const path = folderPath(item);
  // a drive row (any OS) adds Eject, Copy as path, Rename and Properties (plan 23)
  const drive = path === null ? undefined : places.driveAt(path);
  const location = item.location;
  const known = item.knownKey;
  // Home, Gallery, OneDrive and the Bin are not folders: listed means pinned, unpinning hides
  const builtIn = isSidebarKey(item.key) ? item.key : undefined;
  // a built-in folder is "pinned" by being listed; unpinning hides it, re-pinning its path shows it
  const pinned = () =>
    builtIn !== undefined ||
    (path !== null && (known !== undefined || settings.pins.includes(path)));
  return [
    {
      kind: 'item',
      key: 'open-tab',
      label: 'Open in new tab',
      visible: () => location !== undefined,
      run: () => location && tabs.open(location),
    },
    {
      kind: 'item',
      key: 'open-window',
      label: 'Open in new window',
      visible: () => location !== undefined,
      run: () => location && openWindow(locationText(location)),
    },
    { kind: 'divider', key: 'd1', visible: () => path !== null || builtIn !== undefined },
    {
      kind: 'item',
      key: 'pin',
      label: 'Pin to sidebar',
      glyph: 'pinTo',
      visible: () => path !== null && !pinned(),
      run: () => path && settings.togglePin(path, knownKeyOf(path)),
    },
    {
      kind: 'item',
      key: 'unpin',
      label: 'Unpin from sidebar',
      glyph: 'unpin',
      visible: pinned,
      run: () => {
        if (builtIn !== undefined) settings.toggleSidebarKey(builtIn);
        else if (known !== undefined) settings.toggleKnownFolder(known);
        else if (path) settings.togglePin(path);
      },
    },
    {
      kind: 'item',
      key: 'terminal',
      label: 'Open in Terminal',
      glyph: 'terminal',
      visible: () => path !== null,
      run: () => path && on.openTerminal(path),
    },
    ...(drive ? driveMenuItems(drive, surface, on.drive ?? driveMenuHandlers) : []),
  ];
};
