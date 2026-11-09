// The drive rows every drive menu shares (plan 23): the sidebar row, the This PC tile and the file
// list's drive row all get them from `driveMenuItems`, so the three cannot drift apart. Pure data:
// what each row does comes in through `DriveMenuHandlers`.
import type { MenuItem } from '$lib/hooks/use-menu.svelte';
import { notices } from '$lib/state/notices.svelte';
import type { DriveInfo } from '$lib/types/shell';
import { propertiesLabel } from '$lib/utils/platform';

/** Where the menu was opened: the file list has its own Copy as path, so it gets none here. */
export type DriveSurface = 'sidebar' | 'tile' | 'list';

export interface DriveMenuHandlers {
  copyPath: (path: string) => unknown;
  rename: (drive: DriveInfo, surface: DriveSurface) => unknown;
  eject: (drive: DriveInfo) => unknown;
  properties: (drive: DriveInfo) => unknown;
}

/** File systems `check_label` (`core/drive.rs`) knows. Anything else gets the explaining notice. */
const RENAMEABLE_FS = new Set([
  'ntfs',
  'refs',
  'fat',
  'fat32',
  'vfat',
  'msdos',
  'exfat',
  'ext2',
  'ext3',
  'ext4',
  'btrfs',
  'apfs',
  'hfs+',
  'hfs',
]);

/** The one sentence for a volume Velkrys cannot relabel; same words as the Rust side. */
export const unsupportedRename = (fileSystem: string) =>
  `Renaming isn't supported for ${fileSystem} drives.`;

/** F2 on a drive the menu offers no Rename for (network, optical, a root that cannot be relabelled). */
export const DRIVE_NOT_RENAMEABLE = 'This drive can’t be renamed here.';

/** `null` when the drive can be renamed (or its file system is unknown: the OS decides), else why not. */
export const renameRefusal = (drive: DriveInfo): string | null => {
  const fs = drive.fileSystem;
  if (!fs || RENAMEABLE_FS.has(fs.toLowerCase())) return null;
  return unsupportedRename(fs);
};

/**
 * The rows a drive adds, divider first: Eject (removable only), Copy as path (not on the list,
 * which has its own), Rename (not on network, optical or a root that cannot be relabelled), and
 * Properties / Get Info. An unsupported file system keeps its Rename row: clicking it says why.
 */
export const driveMenuItems = (
  drive: DriveInfo,
  surface: DriveSurface,
  on: DriveMenuHandlers,
): MenuItem[] => {
  const rename = (): unknown => {
    const why = renameRefusal(drive);
    if (why !== null) return notices.push({ kind: 'info', message: why });
    return on.rename(drive, surface);
  };
  return [
    { kind: 'divider', key: 'd-drive' },
    {
      kind: 'item',
      key: 'eject',
      label: 'Eject',
      visible: () => drive.ejectable === true,
      run: () => on.eject(drive),
    },
    {
      kind: 'item',
      key: 'copy-path',
      label: 'Copy as path',
      visible: () => surface !== 'list',
      run: () => on.copyPath(drive.path),
    },
    {
      kind: 'item',
      key: 'rename-drive',
      label: 'Rename',
      glyph: 'rename',
      visible: () => drive.canRename === true,
      run: rename,
    },
    {
      kind: 'item',
      key: 'properties',
      get label() {
        return propertiesLabel();
      },
      run: () => on.properties(drive),
    },
  ];
};
