import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ejectDrive } from '$lib/api/drive';
import { THIS_PC, pathLocation } from '$lib/utils/location';
import type { DriveInfo } from '$lib/types/shell';
import { driveEject } from './drive-eject.svelte';
import { notices } from './notices.svelte';
import { places } from './places.svelte';
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

const os = (name: string) =>
  vi.stubGlobal('window', { __VELKRYS_PLATFORM__: { os: name, thisPcLabel: 'This PC' } });
const stick = (path: string, label: string): DriveInfo => ({
  path,
  label,
  totalBytes: null,
  freeBytes: null,
  ejectable: true,
});
const E = stick('E:\\', 'Stick');

describe('driveEject', () => {
  let retry: ReturnType<typeof vi.spyOn>;
  let navigate: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.mocked(ejectDrive).mockReset();
    retry = vi.spyOn(places, 'retryDrives').mockResolvedValue(undefined as never);
    notices.clear();
    navigate = vi.fn(async () => {});
    tabs.active.navigate = navigate as never;
    tabs.active.location = pathLocation('E:\\photos');
    os('windows');
  });
  afterEach(() => vi.unstubAllGlobals());

  it('a tab on the drive goes to This PC and the drives reload', async () => {
    vi.mocked(ejectDrive).mockResolvedValue(true);
    await driveEject.eject(E);
    expect(ejectDrive).toHaveBeenCalledWith('E:\\');
    expect(navigate).toHaveBeenCalledWith(THIS_PC, { replace: true });
    expect(retry).toHaveBeenCalledTimes(1);
  });

  it('a tab on another drive stays', async () => {
    tabs.active.location = pathLocation('D:\\docs');
    vi.mocked(ejectDrive).mockResolvedValue(true);
    await driveEject.eject(E);
    expect(navigate).not.toHaveBeenCalled();
  });

  it('Windows adds no notice of its own (Windows says it)', async () => {
    vi.mocked(ejectDrive).mockResolvedValue(true);
    await driveEject.eject(E);
    expect(notices.list).toHaveLength(0);
  });

  it.each(['macos', 'linux'])('%s says the drive can be removed safely', async (name) => {
    os(name);
    const push = vi.spyOn(notices, 'push');
    vi.mocked(ejectDrive).mockResolvedValue(true);
    await driveEject.eject(E);
    expect(push).toHaveBeenCalledWith({ kind: 'success', message: 'Stick can be removed safely.' });
  });

  it('an error is a notice and nothing moves', async () => {
    const push = vi.spyOn(notices, 'push');
    vi.mocked(ejectDrive).mockRejectedValue('Explorer.exe is using this drive.');
    await driveEject.eject(E);
    expect(push).toHaveBeenCalledWith({
      kind: 'error',
      message: 'Explorer.exe is using this drive.',
    });
    expect(navigate).not.toHaveBeenCalled();
    expect(retry).not.toHaveBeenCalled();
    expect(driveEject.isBusy('E:\\')).toBe(false);
  });

  it('a double click while busy makes one call', async () => {
    let done!: (v: boolean) => void;
    vi.mocked(ejectDrive).mockReturnValue(new Promise<boolean>((r) => (done = r)));
    const first = driveEject.eject(E);
    await driveEject.eject(E);
    expect(driveEject.isBusy('E:\\')).toBe(true);
    done(true);
    await first;
    expect(ejectDrive).toHaveBeenCalledTimes(1);
    expect(driveEject.isBusy('E:\\')).toBe(false);
  });
});
