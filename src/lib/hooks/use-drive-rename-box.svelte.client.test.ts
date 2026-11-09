// The drive rename box on Svelte's client runtime: keys turn into driveRename calls.
import { flushSync } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

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
vi.mock('$lib/api/shell', () => ({
  drives: vi.fn(async () => []),
  knownFolders: vi.fn(async () => []),
  oneDriveRoot: vi.fn(async () => null),
  openTerminal: vi.fn(async () => {}),
}));
vi.mock('$lib/api/wsl', () => ({ wslDistros: vi.fn(async () => []) }));
vi.mock('$lib/utils/focus-return', () => ({ refocusFileList: vi.fn() }));
vi.mock('$lib/api/window', () => ({ close: vi.fn(), openWindow: vi.fn() }));

const { renameDrive } = await import('$lib/api/drive');
const { driveRename } = await import('$lib/state/drive-rename.svelte');
const { refocusFileList } = await import('$lib/utils/focus-return');
const { places } = await import('$lib/state/places.svelte');
const { useDriveRenameBox } = await import('./use-drive-rename-box.svelte');

const D = { path: 'D:\\', label: 'Data', totalBytes: null, freeBytes: null };

const key = (k: string) =>
  ({ key: k, stopPropagation: () => {}, preventDefault: () => {} }) as unknown as KeyboardEvent;

const settle = async () => {
  for (let i = 0; i < 5; i++) await Promise.resolve();
  flushSync();
};

const mount = () => {
  let box!: ReturnType<typeof useDriveRenameBox>;
  const unmount = $effect.root(() => {
    box = useDriveRenameBox();
  });
  flushSync();
  return { box, unmount };
};

describe('the drive rename box', () => {
  beforeEach(() => {
    vi.mocked(renameDrive).mockClear();
    vi.mocked(refocusFileList).mockClear();
    vi.spyOn(places, 'retryDrives').mockResolvedValue(undefined as never);
    driveRename.start(D, 'sidebar');
  });

  it('Enter commits the typed name', async () => {
    const { box, unmount } = mount();
    box.draft = 'Backup';
    box.onkeydown(key('Enter'));
    await settle();
    expect(renameDrive).toHaveBeenCalledWith('D:\\', 'Backup');
    unmount();
  });

  it('Escape cancels, and the blur after it renames nothing', async () => {
    const { box, unmount } = mount();
    box.draft = 'Backup';
    box.onkeydown(key('Escape'));
    box.onblur();
    await settle();
    expect(renameDrive).not.toHaveBeenCalled();
    expect(driveRename.target).toBeNull();
    unmount();
  });

  it('Enter and Escape hand focus back to the file list; blur and other keys do not', async () => {
    const { box, unmount } = mount();
    box.onkeydown(key('a'));
    box.onblur();
    await settle();
    expect(refocusFileList).not.toHaveBeenCalled();
    driveRename.start(D, 'sidebar');
    box.onkeydown(key('Escape'));
    await settle();
    expect(refocusFileList).toHaveBeenCalledTimes(1);
    driveRename.start(D, 'sidebar');
    box.onkeydown(key('Enter'));
    await settle();
    expect(refocusFileList).toHaveBeenCalledTimes(2);
    unmount();
  });

  it('blur commits', async () => {
    const { box, unmount } = mount();
    box.draft = 'Backup';
    box.onblur();
    await settle();
    expect(renameDrive).toHaveBeenCalledTimes(1);
    unmount();
  });

  it('unmounting while open cancels the target', () => {
    const { unmount } = mount();
    unmount();
    expect(driveRename.target).toBeNull();
  });
});
