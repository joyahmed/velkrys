import { beforeEach, describe, expect, it, vi } from 'vitest';
import { driveMenuItems, renameRefusal, type DriveMenuHandlers } from './drive-menu';
import { notices } from '$lib/state/notices.svelte';
import type { DriveInfo } from '$lib/types/shell';
import type { MenuItem } from '$lib/hooks/use-menu.svelte';

const on = (): DriveMenuHandlers => ({
  copyPath: vi.fn(),
  rename: vi.fn(),
  eject: vi.fn(),
  properties: vi.fn(),
});
const drive = (over: Partial<DriveInfo>): DriveInfo => ({
  path: 'C:\\',
  label: 'OS',
  totalBytes: 10,
  freeBytes: 5,
  driveType: 'fixed',
  fileSystem: 'NTFS',
  canRename: true,
  ejectable: false,
  ...over,
});
const shown = (items: MenuItem[]) =>
  items.filter((i) => i.visible?.() !== false && i.kind === 'item').map((i) => i.key);
const row = (items: MenuItem[], key: string) => {
  const r = items.find((i) => i.key === key);
  if (!r || r.kind !== 'item') throw new Error(`no row ${key}`);
  return r;
};

describe('the drive menu rows', () => {
  beforeEach(() => notices.clear());

  it('a fixed C: has Copy as path, Rename and Properties, never Eject', () => {
    expect(shown(driveMenuItems(drive({}), 'sidebar', on()))).toEqual([
      'copy-path',
      'rename-drive',
      'properties',
    ]);
  });

  it('a USB stick adds Eject; a USB HDD (external) does too', () => {
    const stick = drive({ driveType: 'removable', ejectable: true, path: 'E:\\' });
    const hdd = drive({ driveType: 'external', ejectable: true, path: 'F:\\' });
    for (const d of [stick, hdd])
      expect(shown(driveMenuItems(d, 'tile', on()))).toEqual([
        'eject',
        'copy-path',
        'rename-drive',
        'properties',
      ]);
  });

  it('a network drive has Properties and Copy as path only', () => {
    const net = drive({ driveType: 'network', canRename: false, path: 'Z:\\' });
    expect(shown(driveMenuItems(net, 'sidebar', on()))).toEqual(['copy-path', 'properties']);
  });

  it('an optical drive can eject but not rename', () => {
    const cd = drive({ driveType: 'optical', canRename: false, ejectable: true });
    expect(shown(driveMenuItems(cd, 'sidebar', on()))).toEqual([
      'eject',
      'copy-path',
      'properties',
    ]);
  });

  it('Linux / offers neither Rename nor Eject', () => {
    const root = drive({ path: '/', driveType: 'fixed', fileSystem: 'ext4', canRename: false });
    expect(shown(driveMenuItems(root, 'sidebar', on()))).toEqual(['copy-path', 'properties']);
  });

  it('the file list gets no Copy as path (it has its own)', () => {
    expect(shown(driveMenuItems(drive({}), 'list', on()))).toEqual(['rename-drive', 'properties']);
  });

  it('the rows call their handlers with the drive', () => {
    const h = on();
    const d = drive({ ejectable: true, driveType: 'removable' });
    const items = driveMenuItems(d, 'tile', h);
    row(items, 'eject').run();
    row(items, 'copy-path').run();
    row(items, 'rename-drive').run();
    row(items, 'properties').run();
    expect(h.eject).toHaveBeenCalledWith(d);
    expect(h.copyPath).toHaveBeenCalledWith('C:\\');
    expect(h.rename).toHaveBeenCalledWith(d, 'tile');
    expect(h.properties).toHaveBeenCalledWith(d);
  });

  it('exfat on Linux may be renamed; xfs shows Rename and the notice names xfs', () => {
    const h = on();
    const exfat = drive({ path: '/media/u/X', fileSystem: 'exfat' });
    expect(renameRefusal(exfat)).toBeNull();
    const xfs = drive({ path: '/mnt/data', fileSystem: 'xfs' });
    const items = driveMenuItems(xfs, 'sidebar', h);
    expect(shown(items)).toContain('rename-drive');
    row(items, 'rename-drive').run();
    expect(h.rename).not.toHaveBeenCalled();
    expect(notices.list.at(-1)?.message).toBe("Renaming isn't supported for xfs drives.");
  });

  it('an unknown file system is left to the OS: no refusal', () => {
    expect(renameRefusal(drive({ fileSystem: null }))).toBeNull();
    expect(renameRefusal(drive({ fileSystem: undefined }))).toBeNull();
  });

  it('a Mac says Get Info, Windows and Linux say Properties', () => {
    const items = driveMenuItems(drive({}), 'sidebar', on());
    for (const [os, word] of [
      ['macos', 'Get Info'],
      ['windows', 'Properties'],
      ['linux', 'Properties'],
    ]) {
      vi.stubGlobal('window', { __VELKRYS_PLATFORM__: { os, thisPcLabel: 'This PC' } });
      expect(row(items, 'properties').label).toBe(word);
    }
    vi.unstubAllGlobals();
  });
});
