//! One-time carry-over from the app's old name, Trove, to Velkrys (plan 28). A new identifier
//! means new folders, so without this an updated person starts with empty settings, no tabs
//! and no index.
//!
//! A Tauri plugin and not the app's `.setup` hook: plugins start in registration order, before
//! the config windows, so `read_settings` can never race the copy. It is registered third,
//! after the log (so failures are written down) and single-instance (so a second launch exits
//! before it copies anything).
//!
//! What is copied, and only ever copied: every file of the old config folder, `index/` from
//! the old data folder and `parked.json` from the old local data folder. Never the webview
//! profile, icons, logs or a cache: those refill. Theme and coming-from in localStorage go with
//! the webview profile; both mirror `settings.json` and refill, at the cost of one first frame
//! in the OS theme.
//!
//! The OS entries the old name left behind move in one pass after the copy: the default file
//! manager and start at login on each OS. That pass runs only on the direct channel, in a
//! release build and from a stable path, so a build in `target/` never touches the real
//! registry or login entries.
//!
//! The rules: a file already in the new folder is never replaced, the old folder is never
//! changed, a failure logs and the app carries on, and running it twice changes nothing.
//! It runs only in a release build under the real identifier, so a dev build or a Screen build
//! never reads real data.

use std::ffi::OsStr;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::core::{channel, default_handler, parked, startup};

/// The folder name the old app used, beside each of ours.
pub const OLD_ID: &str = "com.joyahmed.trove";
/// The only identifier that may copy real data; a Screen build has its own.
pub const NEW_ID: &str = "com.joyahmed.velkrys";
/// What has been done already, in the new config folder.
pub const MARKER_FILE: &str = "from-trove.json";
const INDEX_DIR: &str = "index";
/// On macOS the three folders are one, so the config pass meets these too.
const NOT_COPIED: [&str; 4] = ["EBWebView", "icons", "logs", "cache"];

/// The app's three folders. On Windows config and data are the same folder, on macOS all three.
#[derive(Debug, Clone, PartialEq)]
pub struct Folders {
    pub config: PathBuf,
    pub data: PathBuf,
    pub local: PathBuf,
}

impl Folders {
    /// The same folders under the old name: same parent, old id.
    fn old(&self) -> Option<Folders> {
        let beside = |new: &Path| Some(new.parent()?.join(OLD_ID));
        Some(Folders {
            config: beside(&self.config)?,
            data: beside(&self.data)?,
            local: beside(&self.local)?,
        })
    }
}

/// `data`: the old folder was found and every file came across. `integration`: the one pass
/// over the OS entries has run. Both only ever go from false to true.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Marker {
    pub data: bool,
    pub integration: bool,
}

/// A marker that cannot be read counts as nothing done: repeating the work is safe.
fn read_marker(config: &Path) -> Marker {
    std::fs::read(config.join(MARKER_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_marker(config: &Path, marker: Marker) -> Result<()> {
    std::fs::create_dir_all(config)?;
    let path = config.join(MARKER_FILE);
    let tmp = tmp_beside(&path);
    std::fs::write(&tmp, serde_json::to_vec_pretty(&marker)?)?;
    std::fs::rename(&tmp, path)
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Tally {
    /// At least one old folder is there.
    pub found: bool,
    pub copied: usize,
    /// Already in the new folder.
    pub kept: usize,
    pub failed: usize,
}

fn tmp_beside(target: &Path) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.tmp", std::process::id()));
    target.with_file_name(name)
}

fn is_tmp(name: &OsStr) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("tmp"))
}

/// A real folder: a link to one does not count.
fn is_real_dir(path: &Path) -> bool {
    path.symlink_metadata().is_ok_and(|m| m.is_dir())
}

/// `from` to `to`, refusing to land on anything already at `to`.
fn rename_new(from: &Path, to: &Path) -> Result<()> {
    match parked::rename_no_replace(from, to) {
        Err(e) if e.kind() == ErrorKind::Unsupported => {
            if to.symlink_metadata().is_ok() {
                return Err(Error::from(ErrorKind::AlreadyExists));
            }
            std::fs::rename(from, to)
        }
        done => done,
    }
}

/// Whole or not at all: a temp file beside the target, then a rename. `false` when something
/// landed at `dst` first. Only the rename may say that: `create_dir_all` also answers
/// `AlreadyExists`, for a file where a folder must go, and that is a failure.
fn place(src: &Path, dst: &Path) -> Result<bool> {
    if let Some(dir) = dst.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = tmp_beside(dst);
    let done = std::fs::copy(src, &tmp).and_then(|_| rename_new(&tmp, dst));
    if done.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    match done {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
}

fn failed(tally: &mut Tally, path: &Path, e: &Error) {
    tally.failed += 1;
    // the reason at warn, the path only at debug (plan 15 D.8.2: release logs no paths)
    log::warn!("from trove: could not copy an item: {e}");
    log::debug!("from trove: could not copy {}: {e}", path.display());
}

fn copy_file(src: &Path, dst: &Path, tally: &mut Tally) {
    if dst.symlink_metadata().is_ok() {
        tally.kept += 1;
        return;
    }
    match place(src, dst) {
        Ok(true) => tally.copied += 1,
        Ok(false) => tally.kept += 1,
        Err(e) => failed(tally, src, &e),
    }
}

/// Every file under `src`, skipping links, `*.tmp` and, at this level only, the names in `skip`.
fn copy_tree(src: &Path, dst: &Path, skip: &[&str], tally: &mut Tally) {
    let entries = match std::fs::read_dir(src) {
        Ok(entries) => entries,
        Err(e) => return failed(tally, src, &e),
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                failed(tally, src, &e);
                continue;
            }
        };
        let name = entry.file_name();
        if is_tmp(&name) || skip.iter().any(|s| OsStr::new(s) == name) {
            continue;
        }
        let (from, to) = (entry.path(), dst.join(&name));
        match entry.file_type() {
            Err(e) => failed(tally, &from, &e),
            Ok(kind) if kind.is_dir() => copy_tree(&from, &to, &[], tally),
            Ok(kind) if kind.is_file() => copy_file(&from, &to, tally),
            // a link or anything stranger
            Ok(_) => {}
        }
    }
}

/// Copy what the spec lists from `old` into `new`. Reads `old`, never writes to it.
pub fn copy_data(old: &Folders, new: &Folders) -> Tally {
    let mut tally = Tally::default();
    if is_real_dir(&old.config) {
        tally.found = true;
        copy_tree(&old.config, &new.config, &NOT_COPIED, &mut tally);
    }
    if is_real_dir(&old.data) {
        tally.found = true;
        let index = old.data.join(INDEX_DIR);
        if is_real_dir(&index) {
            copy_tree(&index, &new.data.join(INDEX_DIR), &[], &mut tally);
        }
    }
    if is_real_dir(&old.local) {
        tally.found = true;
        let journal = old.local.join(parked::FILE);
        if journal.symlink_metadata().is_ok_and(|m| m.is_file()) {
            copy_file(&journal, &new.local.join(parked::FILE), &mut tally);
        }
    }
    tally
}

/// The whole job, with everything real passed in so a test can run it on throw-away folders.
/// `None` when the build may not run it. `integrate` runs at most once per machine, and not
/// while a copy has failed: the keys it reads need the copied backup in place first.
pub fn carry_over(
    identifier: &str,
    release: bool,
    new: &Folders,
    may_integrate: bool,
    integrate: &mut dyn FnMut(),
) -> Option<Marker> {
    if identifier != NEW_ID || !release {
        return None;
    }
    let old = new.old()?;
    let before = read_marker(&new.config);
    let mut marker = before;
    let mut copy_failed = false;
    if !marker.data {
        let tally = copy_data(&old, new);
        marker.data = tally.found && tally.failed == 0;
        copy_failed = tally.failed > 0;
        if tally.found {
            log::info!(
                "from trove: {} copied, {} already there, {} failed",
                tally.copied,
                tally.kept,
                tally.failed
            );
        }
    }
    if may_integrate && !marker.integration && !copy_failed {
        integrate();
        marker.integration = true;
    }
    if marker != before {
        if let Err(e) = write_marker(&new.config, marker) {
            log::warn!("from trove: could not save what was done: {e}");
        }
    }
    Some(marker)
}

/// Whether this build, at this path, may touch the OS entries.
pub fn may_integrate(direct: bool, release: bool, exe: &Path) -> bool {
    direct && release && default_handler::is_stable_path(exe)
}

/// Failure logs and the next step still runs.
fn logged<T: std::fmt::Debug>(what: &str, done: Result<T>) {
    match done {
        Ok(moved) => log::info!("from trove: {what}: {moved:?}"),
        Err(e) => log::warn!("from trove: {what} failed: {e}"),
    }
}

/// The real entries. Called only from the plugin below, behind `may_integrate`; every step it
/// calls is tested on a throw-away root.
#[cfg(windows)]
fn integrate(exe: &Path, config: &Path) {
    use crate::commands::default_handler::CLASSES;
    let backup = config.join(default_handler::BACKUP_FILE);
    logged(
        "default file manager",
        default_handler::retarget(CLASSES, exe, &backup, &startup::is_old_exe),
    );
    logged("start at login", startup::move_run_entry("", exe));
}

#[cfg(target_os = "macos")]
fn integrate(exe: &Path, _config: &Path) {
    logged("start at login", startup::move_login_entry(exe));
}

#[cfg(all(unix, not(target_os = "macos")))]
fn integrate(exe: &Path, config: &Path) {
    use crate::core::default_handler_linux as linux;
    logged("start at login", startup::move_login_entry(exe));
    match crate::commands::default_handler::desktop_path() {
        Ok(desktop) => logged(
            "default file manager",
            linux::retarget(
                &linux::run_xdg_mime,
                exe,
                &desktop,
                &config.join(linux::BACKUP_FILE),
            ),
        ),
        Err(e) => log::warn!("from trove: default file manager failed: {e}"),
    }
}

/// The app's real folders; `None` if the OS cannot name one.
fn folders<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<Folders> {
    let path = app.path();
    Some(Folders {
        config: path.app_config_dir().ok()?,
        data: path.app_data_dir().ok()?,
        local: path.app_local_data_dir().ok()?,
    })
}

/// Registered third in `lib.rs`, see the top of this file.
pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::<R, ()>::new("from-trove")
        .setup(|app, _api| {
            if let Some(new) = folders(app) {
                let identifier = app.config().identifier.clone();
                let release = !cfg!(debug_assertions);
                let direct = channel::CURRENT.offers_default_handler();
                let exe = std::env::current_exe().ok();
                let may = exe
                    .as_deref()
                    .is_some_and(|exe| may_integrate(direct, release, exe));
                let config = new.config.clone();
                carry_over(&identifier, release, &new, may, &mut || {
                    if let Some(exe) = exe.as_deref() {
                        integrate(exe, &config);
                    }
                });
            }
            Ok(())
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// `<temp>/<unique>/` with the app folders under it, the way the OS lays them out. Every
    /// path in these tests is under here.
    struct Scratch {
        root: PathBuf,
        new: Folders,
        old: Folders,
    }

    impl Scratch {
        fn new() -> Scratch {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let root =
                std::env::temp_dir().join(format!("velkrys-from-old-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let under = |kind: &str, id: &str| root.join(kind).join(id);
            let new = Folders {
                config: under("config", NEW_ID),
                data: under("data", NEW_ID),
                local: under("local", NEW_ID),
            };
            let old = Folders {
                config: under("config", OLD_ID),
                data: under("data", OLD_ID),
                local: under("local", OLD_ID),
            };
            Scratch { root, new, old }
        }

        /// A filled old install, Linux-shaped: three separate folders.
        fn filled(self) -> Scratch {
            put(&self.old.config.join("settings.json"), "old settings");
            put(&self.old.config.join("session.json"), "old session");
            put(&self.old.config.join("settings.json.bak"), "old bak");
            put(&self.old.config.join("sub").join("deep.json"), "deep");
            put(&self.old.data.join("index").join("home.idx"), "index");
            put(&self.old.data.join("stray.txt"), "not copied");
            put(&self.old.local.join("parked.json"), "parked");
            put(&self.old.local.join("EBWebView").join("x"), "not copied");
            put(&self.old.local.join("icons").join("y.png"), "not copied");
            self
        }

        fn run(&self) -> Option<Marker> {
            carry_over(NEW_ID, true, &self.new, false, &mut || {})
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn put(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn text(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    /// Every file under `dir` with its bytes and modified time, sorted.
    fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>, std::time::SystemTime)> {
        let mut out = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for entry in fs::read_dir(d).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    let modified = path.metadata().unwrap().modified().unwrap();
                    out.push((path.clone(), fs::read(&path).unwrap(), modified));
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn copies_config_all_data_index_and_local_parked_only() {
        let s = Scratch::new().filled();
        s.run();
        assert_eq!(text(&s.new.config.join("settings.json")), "old settings");
        assert_eq!(text(&s.new.config.join("session.json")), "old session");
        assert_eq!(text(&s.new.config.join("settings.json.bak")), "old bak");
        assert_eq!(text(&s.new.config.join("sub").join("deep.json")), "deep");
        assert_eq!(text(&s.new.data.join("index").join("home.idx")), "index");
        assert_eq!(text(&s.new.local.join("parked.json")), "parked");
        assert!(!s.new.data.join("stray.txt").exists());
        assert!(!s.new.local.join("EBWebView").exists());
        assert!(!s.new.local.join("icons").exists());
        assert_eq!(INDEX_DIR, crate::commands::index_settings::INDEX_DIR);
    }

    #[test]
    fn one_shared_folder_leaves_out_the_webview_icons_logs_and_cache() {
        // macOS: config, data and local data are one folder
        let s = Scratch::new();
        let (old, new) = (s.old.config.clone(), s.new.config.clone());
        let new_all = Folders {
            config: new.clone(),
            data: new.clone(),
            local: new.clone(),
        };
        put(&old.join("settings.json"), "s");
        put(&old.join("index").join("a"), "i");
        put(&old.join("parked.json"), "p");
        for skipped in NOT_COPIED {
            put(&old.join(skipped).join("f"), "x");
        }
        carry_over(NEW_ID, true, &new_all, false, &mut || {});
        assert_eq!(text(&new.join("settings.json")), "s");
        assert_eq!(text(&new.join("index").join("a")), "i");
        assert_eq!(text(&new.join("parked.json")), "p");
        for skipped in NOT_COPIED {
            assert!(!new.join(skipped).exists(), "{skipped}");
        }
    }

    #[test]
    fn an_existing_new_file_is_never_overwritten() {
        let s = Scratch::new().filled();
        put(&s.new.config.join("settings.json"), "new settings");
        let marker = s.run().unwrap();
        assert_eq!(text(&s.new.config.join("settings.json")), "new settings");
        assert_eq!(text(&s.new.config.join("session.json")), "old session");
        assert!(marker.data);
    }

    #[test]
    fn running_twice_changes_nothing_even_after_the_person_edits() {
        let s = Scratch::new().filled();
        assert!(s.run().unwrap().data);
        put(&s.new.config.join("settings.json"), "edited");
        fs::remove_file(s.new.config.join("session.json")).unwrap();
        let again = snapshot(&s.new.config);
        s.run();
        assert_eq!(snapshot(&s.new.config), again);
    }

    #[test]
    fn the_copy_itself_is_safe_to_repeat_without_a_marker() {
        let s = Scratch::new().filled();
        let first = copy_data(&s.old, &s.new);
        assert_eq!((first.copied, first.kept, first.failed), (6, 0, 0));
        let second = copy_data(&s.old, &s.new);
        assert_eq!((second.copied, second.kept, second.failed), (0, 6, 0));
    }

    #[test]
    fn nothing_there_is_fine_and_creates_nothing() {
        let s = Scratch::new();
        assert_eq!(s.run(), Some(Marker::default()));
        assert!(!s.new.config.exists());
    }

    #[test]
    fn no_old_folder_leaves_data_unset_so_a_later_start_looks_again() {
        let s = Scratch::new();
        assert!(!s.run().unwrap().data);
        put(&s.old.config.join("settings.json"), "late");
        assert!(s.run().unwrap().data);
        assert_eq!(text(&s.new.config.join("settings.json")), "late");
    }

    #[test]
    fn a_failure_keeps_going_leaves_data_unset_and_a_later_start_finishes() {
        let s = Scratch::new().filled();
        // a file where a folder must go: `sub/deep.json` cannot land
        put(&s.new.config.join("sub"), "in the way");
        let marker = s.run().unwrap();
        assert!(!marker.data);
        assert_eq!(text(&s.new.config.join("settings.json")), "old settings");
        assert_eq!(text(&s.new.local.join("parked.json")), "parked");

        fs::remove_file(s.new.config.join("sub")).unwrap();
        assert!(s.run().unwrap().data);
        assert_eq!(text(&s.new.config.join("sub").join("deep.json")), "deep");
    }

    #[test]
    fn a_failed_copy_leaves_no_temp_file() {
        let s = Scratch::new();
        put(&s.new.config.join("x"), "");
        let missing = s.old.config.join("gone.json");
        let mut tally = Tally::default();
        copy_file(&missing, &s.new.config.join("gone.json"), &mut tally);
        assert_eq!(tally.failed, 1);
        let names: Vec<_> = fs::read_dir(&s.new.config).unwrap().collect();
        assert_eq!(names.len(), 1);
    }

    #[test]
    fn the_old_folders_are_never_changed() {
        let s = Scratch::new().filled();
        put(&s.new.config.join("settings.json"), "new settings");
        let all = |s: &Scratch| [&s.old.config, &s.old.data, &s.old.local].map(|d| snapshot(d));
        let before = all(&s);
        s.run();
        s.run();
        assert_eq!(all(&s), before);
    }

    #[test]
    fn temp_files_are_skipped() {
        let s = Scratch::new();
        put(&s.old.config.join("settings.json.123.tmp"), "half");
        put(&s.old.config.join("ok.json"), "ok");
        s.run();
        assert!(s.new.config.join("ok.json").exists());
        assert!(!s.new.config.join("settings.json.123.tmp").exists());
    }

    #[cfg(unix)]
    #[test]
    fn links_are_skipped_not_followed() {
        let s = Scratch::new();
        put(&s.old.config.join("ok.json"), "ok");
        put(&s.root.join("outside").join("secret"), "x");
        std::os::unix::fs::symlink(s.root.join("outside"), s.old.config.join("dir")).unwrap();
        std::os::unix::fs::symlink(s.old.config.join("ok.json"), s.old.config.join("file"))
            .unwrap();
        s.run();
        assert!(s.new.config.join("ok.json").exists());
        assert!(s.new.config.join("dir").symlink_metadata().is_err());
        assert!(s.new.config.join("file").symlink_metadata().is_err());
    }

    /// A junction needs no special privilege, unlike a symlink on Windows.
    #[cfg(windows)]
    #[test]
    fn a_junction_is_skipped_not_followed() {
        let s = Scratch::new();
        put(&s.old.config.join("ok.json"), "ok");
        put(&s.root.join("outside").join("secret"), "x");
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(s.old.config.join("dir"))
            .arg(s.root.join("outside"))
            .output()
            .unwrap();
        assert!(made.status.success());
        assert!(s.old.config.join("dir").join("secret").exists());
        s.run();
        assert!(s.new.config.join("ok.json").exists());
        assert!(s.new.config.join("dir").symlink_metadata().is_err());
    }

    #[test]
    fn another_identifier_or_a_debug_build_copies_nothing() {
        let s = Scratch::new().filled();
        let mut ran = false;
        assert_eq!(
            carry_over(
                "com.joyahmed.velkrys.screen",
                true,
                &s.new,
                true,
                &mut || { ran = true }
            ),
            None
        );
        assert_eq!(
            carry_over(NEW_ID, false, &s.new, true, &mut || ran = true),
            None
        );
        assert!(!ran);
        assert!(!s.new.config.exists());
    }

    #[test]
    fn the_marker_records_data_and_a_broken_one_counts_as_nothing_done() {
        let s = Scratch::new().filled();
        s.run();
        assert_eq!(
            read_marker(&s.new.config),
            Marker {
                data: true,
                integration: false
            }
        );
        put(&s.new.config.join(MARKER_FILE), "not json");
        assert_eq!(read_marker(&s.new.config), Marker::default());
    }

    #[test]
    fn integration_runs_once_after_the_copy_with_the_backup_in_place() {
        let s = Scratch::new().filled();
        let backup = default_handler::BACKUP_FILE;
        put(&s.old.config.join(backup), "true state");
        let mut seen = Vec::new();
        let target = s.new.config.join(backup);
        let mut pass = || seen.push(text(&target));
        let marker = carry_over(NEW_ID, true, &s.new, true, &mut pass).unwrap();
        assert!(marker.data && marker.integration);
        carry_over(NEW_ID, true, &s.new, true, &mut pass).unwrap();
        assert_eq!(seen, ["true state"]);
    }

    #[test]
    fn integration_waits_while_a_copy_has_failed() {
        let s = Scratch::new().filled();
        put(&s.new.config.join("sub"), "in the way");
        let mut runs = 0;
        let marker = carry_over(NEW_ID, true, &s.new, true, &mut || runs += 1).unwrap();
        assert_eq!(runs, 0);
        assert!(!marker.integration);

        fs::remove_file(s.new.config.join("sub")).unwrap();
        let marker = carry_over(NEW_ID, true, &s.new, true, &mut || runs += 1).unwrap();
        assert_eq!(runs, 1);
        assert!(marker.data && marker.integration);
    }

    #[test]
    fn a_build_that_may_not_integrate_leaves_it_for_a_later_start() {
        let s = Scratch::new().filled();
        let mut runs = 0;
        let marker = carry_over(NEW_ID, true, &s.new, false, &mut || runs += 1).unwrap();
        assert_eq!(runs, 0);
        assert!(marker.data && !marker.integration);
        carry_over(NEW_ID, true, &s.new, true, &mut || runs += 1).unwrap();
        assert_eq!(runs, 1);
    }

    #[test]
    fn integration_needs_direct_release_and_a_stable_path() {
        let stable = Path::new("/opt/Velkrys/velkrys");
        assert!(may_integrate(true, true, stable));
        assert!(!may_integrate(false, true, stable));
        assert!(!may_integrate(true, false, stable));
        let built = Path::new("/w/src-tauri/target/release/velkrys");
        assert!(!may_integrate(true, true, built));
        assert!(!may_integrate(true, true, &std::env::temp_dir().join("v")));
    }

    /// The test binary is in `target/`, so even with the other two gates open it is refused.
    #[test]
    fn the_test_binary_itself_may_not_integrate() {
        let exe = std::env::current_exe().unwrap();
        assert!(!may_integrate(true, true, &exe));
    }

    #[test]
    fn the_old_folder_sits_beside_the_new_one() {
        let new = Folders {
            config: PathBuf::from("/c").join(NEW_ID),
            data: PathBuf::from("/d").join(NEW_ID),
            local: PathBuf::from("/l").join(NEW_ID),
        };
        let old = new.old().unwrap();
        assert_eq!(old.config, PathBuf::from("/c").join(OLD_ID));
        assert_eq!(old.data, PathBuf::from("/d").join(OLD_ID));
        assert_eq!(old.local, PathBuf::from("/l").join(OLD_ID));
    }
}
