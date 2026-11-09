import { beforeEach, describe, expect, it, vi } from 'vitest';
import { renameDrive } from '$lib/api/drive';
import { pathLocation } from '$lib/utils/location';
import type { DriveInfo } from '$lib/types/shell';
import { driveRename } from './drive-rename.svelte';
import { notices } from './notices.svelte';
import { places } from './places.svelte';
import { settings } from './settings.svelte';
import { tabs } from './tabs.svelte';

vi.mock('$lib/api/drive', () => ({
  renameDrive: vi.fn(),
  driveDetails: vi.fn(),
  ejectDrive: vi.fn(),
}));
vi.mock('$lib/api/shell', () => ({
  drives: vi.fn(async () => []),
  knownFolders: vi.fn(async () => []),
  oneDriveRoot: vi.fn(async () => null),
  openTerminal: vi.fn(async () => {}),
}));
vi.mock('$lib/api/wsl', () => ({ wslDistros: vi.fn(async () => []) }));
vi.mock('$lib/api/window', () => ({ close: vi.fn(), openWindow: vi.fn() }));

const drive = (path: string, label: string): DriveInfo => ({
  path,
  label,
  totalBytes: null,
  freeBytes: null,
});
const D = drive('D:\\', 'Data');

describe('driveRename', () => {
  let retry: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    vi.mocked(renameDrive).mockReset();
    retry = vi.spyOn(places, 'retryDrives').mockResolvedValue(undefined as never);
    driveRename.cancel();
  });

  it('an emptied name makes no call', async () => {
    driveRename.start(D, 'sidebar');
    driveRename.draft = '   ';
    await driveRename.commit();
    expect(renameDrive).not.toHaveBeenCalled();
    expect(driveRename.target).toBeNull();
  });

  it('the same name makes no call', async () => {
    driveRename.start(D, 'tile');
    await driveRename.commit();
    expect(renameDrive).not.toHaveBeenCalled();
  });

  it('a failure shows a notice with the message and refreshes nothing', async () => {
    vi.mocked(renameDrive).mockRejectedValue('Windows asked for administrator rights');
    const push = vi.spyOn(notices, 'push');
    driveRename.start(D, 'sidebar');
    driveRename.draft = 'Backup';
    await driveRename.commit();
    expect(push).toHaveBeenCalledWith({
      kind: 'error',
      message: 'Windows asked for administrator rights',
    });
    expect(retry).not.toHaveBeenCalled();
  });

  it('a success reloads the drives', async () => {
    vi.mocked(renameDrive).mockResolvedValue(drive('D:\\', 'Backup'));
    driveRename.start(D, 'sidebar');
    driveRename.draft = ' Backup ';
    await driveRename.commit();
    expect(renameDrive).toHaveBeenCalledWith('D:\\', 'Backup');
    expect(retry).toHaveBeenCalledTimes(1);
  });

  it('Enter then the blur it causes send the name once', async () => {
    vi.mocked(renameDrive).mockResolvedValue(drive('D:\\', 'Backup'));
    driveRename.start(D, 'sidebar');
    driveRename.draft = 'Backup';
    await Promise.all([driveRename.commit(), driveRename.commit()]);
    expect(renameDrive).toHaveBeenCalledTimes(1);
  });

  it('only the box of the surface that opened it is the target', () => {
    driveRename.start(D, 'tile');
    expect(driveRename.isTarget('D:\\', 'tile')).toBe(true);
    expect(driveRename.isTarget('D:\\', 'sidebar')).toBe(false);
    expect(driveRename.isTarget('E:\\', 'tile')).toBe(false);
  });

  it('a Mac mount point that changes moves a tab and a pin with it', async () => {
    const navigate = vi.fn(async () => {});
    const tab = tabs.active;
    tab.navigate = navigate;
    tab.location = pathLocation('/Volumes/Old/docs');
    settings.pins = ['/Volumes/Old/docs', '/Users/j'];
    vi.mocked(renameDrive).mockResolvedValue(drive('/Volumes/New', 'New'));
    driveRename.start(drive('/Volumes/Old', 'Old'), 'sidebar');
    driveRename.draft = 'New';
    await driveRename.commit();
    expect(navigate).toHaveBeenCalledWith(pathLocation('/Volumes/New/docs'), { replace: true });
    expect(settings.pins).toEqual(['/Volumes/New/docs', '/Users/j']);
  });

  it('commitFor (the file list) renames without a box and skips empty or unchanged', async () => {
    vi.mocked(renameDrive).mockResolvedValue(drive('D:\\', 'Backup'));
    await driveRename.commitFor(D, '');
    await driveRename.commitFor(D, 'Data');
    expect(renameDrive).not.toHaveBeenCalled();
    await driveRename.commitFor(D, 'Backup');
    expect(renameDrive).toHaveBeenCalledWith('D:\\', 'Backup');
  });
});
