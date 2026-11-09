// Renaming a drive in place (plan 23.7): one box at a time, in the sidebar or on a This PC tile
// (`target` names which), or from the file list through `commitFor`. Same keys as a file: an empty
// or unchanged name makes no call. No undo entry (see the plan's Inverses): the label is one
// Rename away, and Ctrl+Z replaying a UAC or password prompt would be a surprise.
import { renameDrive } from '$lib/api/drive';
import { notices } from '$lib/state/notices.svelte';
import { places } from '$lib/state/places.svelte';
import { settings } from '$lib/state/settings.svelte';
import { tabs } from '$lib/state/tabs.svelte';
import type { DriveInfo } from '$lib/types/shell';
import { samePath } from '$lib/utils/path';

export type RenameSurface = 'sidebar' | 'tile';

class DriveRenameState {
  /** The drive whose name is being edited, and where its box is drawn. */
  target = $state<{ path: string; surface: RenameSurface } | null>(null);
  draft = $state('');
  #label = '';

  /** Whether the box on `surface` for the drive at `path` is the open one. */
  isTarget(path: string, surface: RenameSurface) {
    const t = this.target;
    return t !== null && t.surface === surface && samePath(t.path, path);
  }

  start = (drive: DriveInfo, surface: RenameSurface | 'list') => {
    // the file list edits in its own row box (`actions.startRename`), not here
    if (surface === 'list') return;
    this.target = { path: drive.path, surface };
    this.draft = drive.label;
    this.#label = drive.label;
  };

  cancel = () => {
    this.target = null;
  };

  /** Enter or blur in the box. */
  commit = async () => {
    const t = this.target;
    const name = this.draft.trim();
    // cleared first: Enter and the blur it causes must not send the name twice
    this.target = null;
    if (!t || name === '' || name === this.#label) return;
    await this.#rename(t.path, name);
  };

  /** The file list's rename of a drive row: the name already typed, no box of ours. */
  commitFor = async (drive: DriveInfo, name: string) => {
    const typed = name.trim();
    if (typed === '' || typed === drive.label) return;
    await this.#rename(drive.path, typed);
  };

  #rename = async (path: string, label: string) => {
    try {
      const renamed = await renameDrive(path, label);
      // the mount point can change with the label (macOS): what pointed into the old one follows
      if (!samePath(renamed.path, path)) {
        tabs.retargetPrefix(path, renamed.path);
        settings.retargetPrefix(path, renamed.path);
      }
      await places.retryDrives();
    } catch (e) {
      notices.push({ kind: 'error', message: String(e) });
    }
  };
}

export const driveRename = new DriveRenameState();
