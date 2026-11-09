// The inline box for renaming a drive (sidebar row or This PC tile): Enter commits, Escape
// cancels, blur commits, the whole label is pre-selected. The rules for an empty or unchanged
// name live in `driveRename`; this only turns keys into those calls. A box that unmounts while
// still open (its section folded away) cancels, so no stale target waits to reappear.
import { tick } from 'svelte';
import { driveRename } from '$lib/state/drive-rename.svelte';
import { refocusFileList } from '$lib/utils/focus-return';

export const useDriveRenameBox = () => {
  $effect(() => {
    return () => driveRename.cancel();
  });

  const onkeydown = (e: KeyboardEvent) => {
    // the row's own keys (arrows, Enter to open, F2) must not see what is typed here
    e.stopPropagation();
    if (e.key === 'Enter') void driveRename.commit();
    else if (e.key === 'Escape') driveRename.cancel();
    else return;
    // the box unmounts with the key; without this focus falls to <body> and the file list's keys
    // do nothing until a click (same as the file rename box)
    void tick().then(() => refocusFileList());
  };

  /** `use:` action: focus and select the whole label. */
  const focus = (el: HTMLInputElement) => {
    el.focus();
    el.select();
    el.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  };

  return {
    get draft() {
      return driveRename.draft;
    },
    set draft(v: string) {
      driveRename.draft = v;
    },
    onkeydown,
    onblur: () => void driveRename.commit(),
    focus,
  };
};
