// Ejecting a removable drive (plan 23.9). Once it is safe to unplug, every tab on that drive goes to
// This PC (a tab left on an ejected drive would only show an error) and the drive list reloads.
// Windows says "safe to remove" itself; Mac and Linux are told here. No inverse: plug it in again.
import { ejectDrive } from '$lib/api/drive';
import { notices } from '$lib/state/notices.svelte';
import { places } from '$lib/state/places.svelte';
import { tabs } from '$lib/state/tabs.svelte';
import type { DriveInfo } from '$lib/types/shell';
import { THIS_PC } from '$lib/utils/location';
import { isUnderPath } from '$lib/utils/path';
import { onWindows } from '$lib/utils/platform';

class DriveEjectState {
  /** the drives being ejected now, so a second click while one runs makes no second call */
  #busy = new Set<string>();

  isBusy(path: string) {
    return this.#busy.has(path);
  }

  eject = async (drive: DriveInfo) => {
    if (this.#busy.has(drive.path)) return;
    this.#busy.add(drive.path);
    try {
      const safe = await ejectDrive(drive.path);
      if (!safe) return;
      for (const tab of tabs.list) {
        if (tab.location.kind === 'path' && isUnderPath(tab.path, drive.path))
          void tab.navigate(THIS_PC, { replace: true });
      }
      await places.retryDrives();
      if (!onWindows())
        notices.push({
          kind: 'success',
          message: `${drive.label || drive.path} can be removed safely.`,
        });
    } catch (e) {
      notices.push({ kind: 'error', message: String(e) });
    } finally {
      this.#busy.delete(drive.path);
    }
  };
}

export const driveEject = new DriveEjectState();
