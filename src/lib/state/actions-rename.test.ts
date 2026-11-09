import { beforeEach, describe, expect, it, vi } from 'vitest';
import { actions } from './actions.svelte';
import { selection } from './selection.svelte';
import { tabs } from './tabs.svelte';
import { pathLocation } from '$lib/utils/location';
import { renameEntry } from '$lib/api/fs';
import { listing } from './listing.svelte';
import { settings } from './settings.svelte';

vi.mock('$lib/api/fs', () => ({
  readDir: vi.fn(async (path: string) => ({ path, entries: [] })),
  renameEntry: vi.fn(async (_from: string, name: string) => `C:\\scratch\\${name}`),
  trashEntries: vi.fn(async () => {}),
}));
vi.mock('$lib/api/shell', () => ({
  drives: vi.fn(async () => []),
  knownFolders: vi.fn(async () => []),
  oneDriveRoot: vi.fn(async () => null),
  openTerminal: vi.fn(async () => {}),
}));
vi.mock('$lib/api/drive', () => ({
  renameDrive: vi.fn(async (path: string, label: string) => ({
    path,
    label,
    totalBytes: null,
    freeBytes: null,
  })),
  driveDetails: vi.fn(),
  ejectDrive: vi.fn(),
}));
vi.mock('$lib/api/transfer', () => ({
  restoreFromTrash: vi.fn(async () => {}),
  transferConflicts: vi.fn(async () => []),
  startTransfer: vi.fn(async () => 1),
  cancelTransfer: vi.fn(async () => {}),
  onTransferProgress: vi.fn(async () => () => {}),
  onTransferDone: vi.fn(async () => () => {}),
}));
vi.mock('$lib/api/clipboard', () => ({
  clipboardWritePaths: vi.fn(async () => {}),
  clipboardReadPaths: vi.fn(async () => null),
}));

const OLD = 'C:\\scratch\\a.txt';

describe('actions.commitRename', () => {
  beforeEach(() => {
    vi.mocked(renameEntry).mockClear();
    tabs.active.location = pathLocation('C:\\scratch');
    tabs.active.entries = [
      {
        name: 'a.txt',
        path: OLD,
        isDir: false,
        size: 1,
        modifiedMs: null,
        createdMs: null,
        ext: 'txt',
        isHidden: false,
        iconKey: 'ext/txt',
        typeName: 'File',
      },
    ];
    selection.clear();
    vi.spyOn(tabs.active, 'reload').mockResolvedValue();
    expect(listing.rows.map((e) => e.path)).toEqual([OLD]);
  });

  it('with extensions hidden the box holds the base name and the extension is kept', async () => {
    settings.showExtensions = false;
    try {
      actions.startRename(OLD);
      expect(actions.draft).toBe('a');
      expect(actions.renameHidesExtension).toBe(true);
      actions.draft = 'notes';
      await actions.commitRename();
      expect(renameEntry).toHaveBeenCalledWith(OLD, 'notes.txt');
      // unchanged base name: no rename at all
      vi.mocked(renameEntry).mockClear();
      actions.startRename(OLD);
      await actions.commitRename();
      expect(renameEntry).not.toHaveBeenCalled();
    } finally {
      settings.showExtensions = true;
    }
  });

  it('with extensions shown the box holds the whole name', () => {
    actions.startRename(OLD);
    expect(actions.draft).toBe('a.txt');
    expect(actions.renameHidesExtension).toBe(false);
    actions.cancelRename();
  });

  it('an emptied name is a cancel, not a rename to nothing', async () => {
    actions.startRename(OLD);
    actions.draft = '   ';
    await actions.commitRename();
    expect(renameEntry).not.toHaveBeenCalled();
    expect(actions.renaming).toBeNull();
  });

  it('selects the renamed row while its folder is still the one on screen', async () => {
    actions.startRename(OLD);
    actions.draft = 'b.txt';
    await actions.commitRename();
    expect(selection.has('C:\\scratch\\b.txt')).toBe(true);
  });

  it('does not select anything if the person left the folder while it was renaming', async () => {
    actions.startRename(OLD);
    actions.draft = 'b.txt';
    vi.mocked(renameEntry).mockImplementationOnce(async (_from, name) => {
      tabs.active.location = pathLocation('C:\\elsewhere');
      return `C:\\scratch\\${name}`;
    });
    await actions.commitRename();
    expect(selection.count).toBe(0);
  });
});

describe('F2 on a drive row', () => {
  it('edits the label and calls renameDrive, never renameEntry', async () => {
    const { renameDrive } = await import('$lib/api/drive');
    const { places } = await import('./places.svelte');
    vi.mocked(renameEntry).mockClear();
    vi.mocked(renameDrive).mockClear();
    const root = 'D:\\';
    const d = { path: root, label: 'Data', totalBytes: null, freeBytes: null, canRename: true };
    vi.spyOn(places, 'driveAt').mockImplementation((p: string) => (p === root ? d : undefined));
    vi.spyOn(places, 'retryDrives').mockResolvedValue(undefined as never);
    tabs.active.location = pathLocation('C:\\scratch');
    tabs.active.entries = [
      {
        name: 'D:\\',
        path: root,
        isDir: true,
        size: 0,
        modifiedMs: null,
        createdMs: null,
        ext: '',
        isHidden: false,
        iconKey: 'drive',
        typeName: 'Drive',
      },
    ];
    actions.startRename(root);
    expect(actions.draft).toBe('Data');
    actions.draft = 'Backup';
    await actions.commitRename();
    expect(renameDrive).toHaveBeenCalledWith(root, 'Backup');
    expect(renameEntry).not.toHaveBeenCalled();
  });

  it('opens no box on a drive the menu offers no Rename for (a network drive)', async () => {
    const { places } = await import('./places.svelte');
    const root = 'J:\\';
    const d = { path: root, label: 'joy', totalBytes: null, freeBytes: null, canRename: false };
    vi.spyOn(places, 'driveAt').mockImplementation((p: string) => (p === root ? d : undefined));
    const notify = vi.spyOn(actions, 'notify');
    tabs.active.location = pathLocation('C:\\scratch');
    tabs.active.entries = [
      {
        name: 'J:\\',
        path: root,
        isDir: true,
        size: 0,
        modifiedMs: null,
        createdMs: null,
        ext: '',
        isHidden: false,
        iconKey: 'drive',
        typeName: 'Drive',
      },
    ];
    actions.renaming = null;
    actions.startRename(root);
    expect(actions.renaming).toBeNull();
    expect(notify).toHaveBeenCalledWith('This drive can’t be renamed here.');
  });
});
