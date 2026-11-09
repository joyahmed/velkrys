import { describe, expect, it, vi } from 'vitest';
import { details } from './details.svelte';
import { driveEject } from './drive-eject.svelte';
import { driveMenuHandlers } from './drive-menu-handlers';
import type { DriveInfo } from '$lib/types/shell';

vi.mock('$lib/api/drive', () => ({
  renameDrive: vi.fn(),
  driveDetails: vi.fn(),
  ejectDrive: vi.fn(),
}));
vi.mock('$lib/api/props', () => ({
  fileProps: vi.fn(),
  setFileAttributes: vi.fn(),
  startFolderTotals: vi.fn(),
  cancelFolderTotals: vi.fn(),
  onFolderTotals: vi.fn(async () => () => {}),
}));
vi.mock('$lib/api/shell', () => ({
  drives: vi.fn(async () => []),
  knownFolders: vi.fn(async () => []),
  oneDriveRoot: vi.fn(async () => null),
}));
vi.mock('$lib/api/wsl', () => ({ wslDistros: vi.fn(async () => []) }));
vi.mock('$lib/api/window', () => ({ close: vi.fn(), openWindow: vi.fn() }));

const D: DriveInfo = { path: 'D:\\', label: 'Data', totalBytes: null, freeBytes: null };

describe('the drive menu handlers', () => {
  it('Properties calls showDrive with the drive path', () => {
    const show = vi.spyOn(details, 'showDrive').mockImplementation(() => {});
    driveMenuHandlers.properties(D);
    expect(show).toHaveBeenCalledWith('D:\\');
  });

  it('Eject runs the eject state', () => {
    const eject = vi.spyOn(driveEject, 'eject').mockResolvedValue(undefined);
    void driveMenuHandlers.eject(D);
    expect(driveMenuHandlers.eject).toBeTypeOf('function');
    expect(eject).toHaveBeenCalled();
  });
});
