// What the drive menu rows do (plan 23): the real handlers behind `driveMenuItems`, for the
// sidebar and the This PC tiles.
import { clipboardWritePathsText } from '$lib/api/clipboard';
import type { DriveMenuHandlers } from '$lib/data/drive-menu';
import { details } from '$lib/state/details.svelte';
import { driveEject } from '$lib/state/drive-eject.svelte';
import { driveRename } from '$lib/state/drive-rename.svelte';
import { notices } from '$lib/state/notices.svelte';

export const driveMenuHandlers: DriveMenuHandlers = {
  copyPath: async (path) => {
    try {
      await clipboardWritePathsText([path]);
    } catch (e) {
      notices.push({ kind: 'error', message: String(e) });
    }
  },
  rename: driveRename.start,
  eject: (drive) => driveEject.eject(drive),
  properties: (drive) => details.showDrive(drive.path),
};
