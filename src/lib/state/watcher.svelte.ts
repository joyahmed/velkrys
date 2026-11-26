// Live refresh: exactly the folders open in tabs are watched on the Rust side, and a change
// swaps the listing in without flicker. A folder that vanishes sends its tabs up with a
// notice. WSL shares get no events across the 9P boundary, so those tabs are re-read on
// window focus and every 5 s while the window is focused.
import { onFsChanged, unwatchPath, watchPath } from '$lib/api/watch';
import { applications } from '$lib/state/applications.svelte';
import { gallery } from '$lib/state/gallery.svelte';
import { actions } from '$lib/state/actions.svelte';
import { tabs } from '$lib/state/tabs.svelte';
import type { TabState } from '$lib/state/tab.svelte';
import { HOME, locationLabel, parentLocation } from '$lib/utils/location';
import { isWslPath, samePath } from '$lib/utils/path';

const WSL_POLL_MS = 5000;
/** After a reload lands, the next one waits this long for the burst to end... */
const RELOAD_QUIET_MS = 250;
/** ...but no change waits longer than this, however busy the folder stays. */
const MAX_WAIT_MS = 1500;

interface ReloadState {
  inflight: boolean;
  /** a change arrived while the reload was reading */
  dirty: boolean;
  /** when the oldest change still waiting for a reload arrived */
  since: number | null;
  timer?: ReturnType<typeof setTimeout>;
  endedAt: number;
}

class WatcherState {
  /** folders currently watched, as passed to `watch_path` */
  #watched: string[] = [];
  /** folders whose live watch was refused (e.g. the OS watch limit): polled like a WSL share */
  #failed: string[] = [];
  #told = false;
  #poll: ReturnType<typeof setInterval> | undefined;

  /** Tabs whose folder is `path`. */
  #tabsOn = (path: string): TabState[] => tabs.list.filter((t) => samePath(t.path, path));

  #polledTabs = () =>
    tabs.list.filter(
      (t) => t.isFolder && (isWslPath(t.path) || this.#failed.some((f) => samePath(f, t.path))),
    );

  /** Start one watch; if the OS refuses, poll that folder instead and say so once. */
  #watch(path: string) {
    watchPath(path).then(
      () => void (this.#failed = this.#failed.filter((f) => !samePath(f, path))),
      () => {
        if (!this.#watched.some((w) => samePath(w, path))) return; // closed meanwhile
        this.#watched = this.#watched.filter((w) => !samePath(w, path));
        if (!this.#failed.some((f) => samePath(f, path))) this.#failed.push(path);
        if (!this.#told) {
          this.#told = true;
          actions.notify(
            'Live updates are off for some folders, so they refresh every 5 seconds instead.',
            'warning',
          );
        }
      },
    );
  }

  /** A watched folder changed under us: re-read it in every tab that shows it. */
  changed(path: string) {
    for (const tab of this.#tabsOn(path)) this.#poke(tab);
    gallery.changed(path);
    applications.changed(path);
  }

  /**
   * One tab's reload bookkeeping. Every `tab.reload()` takes a fresh generation, so a reload
   * that is still reading when the next one starts is thrown away on landing: in a folder that
   * is written to constantly (a download, a build) a read slower than the gap between events
   * would never be shown. So at most one reload is in flight per tab; changes that arrive
   * meanwhile only mark it dirty and are served by ONE trailing reload once it lands. Trailing
   * reloads wait out a short quiet gap, but never longer than MAX_WAIT_MS from the first change
   * they hold, so a folder that never goes quiet still updates every ~1.5 s.
   */
  #states = new WeakMap<TabState, ReloadState>();

  #poke(tab: TabState) {
    let st = this.#states.get(tab);
    if (!st)
      this.#states.set(tab, (st = { inflight: false, dirty: false, since: null, endedAt: 0 }));
    if (st.inflight) {
      st.dirty = true;
      return;
    }
    const now = Date.now();
    // idle and quiet: re-read at once (the Rust side has already debounced the burst)
    if (st.timer === undefined && now - st.endedAt >= RELOAD_QUIET_MS) {
      void this.#run(tab, st);
      return;
    }
    st.since ??= now;
    clearTimeout(st.timer);
    const delay = Math.max(0, Math.min(RELOAD_QUIET_MS, st.since + MAX_WAIT_MS - now));
    st.timer = setTimeout(() => void this.#run(tab, st), delay);
  }

  async #run(tab: TabState, st: ReloadState) {
    clearTimeout(st.timer);
    st.timer = undefined;
    st.since = null;
    st.dirty = false;
    st.inflight = true;
    try {
      await tab.reload();
    } finally {
      st.inflight = false;
      st.endedAt = Date.now();
      if (st.dirty) {
        st.dirty = false;
        this.#poke(tab);
      }
    }
  }

  /** The folder itself is gone: its tabs go up, and the person is told. */
  gone(path: string) {
    for (const tab of this.#tabsOn(path)) {
      const name = locationLabel(tab.location) || path;
      void tab.navigate(parentLocation(tab.location) ?? HOME, { replace: true });
      actions.notify(`${name} was moved or deleted`, 'warning');
    }
  }

  /**
   * Bring the Rust watchers in line with the open tabs: watch what is newly open, unwatch
   * what closed. Called from an effect that reads every tab's path. A place ('' path) and a
   * WSL share are not watchable; the latter is polled instead, and so is a folder whose watch
   * failed (it is tried again on the next sync, i.e. when a tab navigates).
   */
  sync(paths: string[]) {
    // one watcher per folder, however the path is cased in each tab
    const wanted = paths
      .filter((p) => p !== '' && !isWslPath(p))
      .filter((p, i, all) => all.findIndex((q) => samePath(q, p)) === i);
    const fresh = wanted.filter((p) => !this.#watched.some((w) => samePath(w, p)));
    const closed = this.#watched.filter((w) => !wanted.some((p) => samePath(p, w)));
    for (const w of closed) void unwatchPath(w).catch(() => {});
    this.#failed = this.#failed.filter((f) => wanted.some((p) => samePath(p, f)));
    this.#watched = [...this.#watched.filter((w) => !closed.includes(w)), ...fresh];
    for (const p of fresh) this.#watch(p);
  }

  get watched(): readonly string[] {
    return this.#watched;
  }

  /** Re-read every WSL tab and every tab on a folder whose watch failed — on focus, and on the poll while focused. */
  reloadWsl = () => {
    for (const tab of this.#polledTabs()) void tab.reload();
  };

  #startPoll = () => {
    clearInterval(this.#poll);
    this.#poll = setInterval(this.reloadWsl, WSL_POLL_MS);
  };
  #stopPoll = () => {
    clearInterval(this.#poll);
    this.#poll = undefined;
  };

  #onFocus = () => {
    this.reloadWsl();
    this.#startPoll();
  };

  /** Subscribe to change events and the window's focus; returns the detach fn. */
  attach() {
    const unlisten = onFsChanged((c) =>
      c.kind === 'gone' ? this.gone(c.path) : this.changed(c.path),
    );
    window.addEventListener('focus', this.#onFocus);
    window.addEventListener('blur', this.#stopPoll);
    if (document.hasFocus()) this.#startPoll();
    return () => {
      void unlisten.then((f) => f());
      window.removeEventListener('focus', this.#onFocus);
      window.removeEventListener('blur', this.#stopPoll);
      this.#stopPoll();
    };
  }
}

export const watcher = new WatcherState();
