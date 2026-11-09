//! The drive menu's three commands (plan 23.5): Properties, Rename, Eject. Thin: the rules are in
//! `core/drive.rs`, the asking and doing in `core/drive_windows|mac|linux.rs`.
//!
//! Counts, never paths, in the log (A.4, `commands/shell.rs`).

use crate::core::drive::{check_label, DriveDetails};
use crate::core::shell::{self, DriveInfo};

const GONE: &str = "That drive is not there any more.";
#[cfg(windows)]
const NOT_GIVEN: &str = "Drive not renamed: administrator permission was not given.";

/// Everything the Properties pane shows for the drive at `path`.
#[tauri::command(async)]
pub fn drive_details(path: String) -> Result<DriveDetails, String> {
    let info = shell::drive_info(&path).ok_or_else(|| GONE.to_string())?;
    let mut d = details_of(&path, &info);
    // one source for the sizes, the same as the drive row
    d.total_bytes = info.total_bytes;
    d.free_bytes = info.free_bytes;
    log::info!(
        "drive_details: {:?}, sizes known: {}",
        d.drive_type,
        d.total_bytes.is_some()
    );
    Ok(d)
}

#[cfg(windows)]
fn details_of(path: &str, _info: &DriveInfo) -> DriveDetails {
    crate::core::drive_windows::volume_details(path)
}

#[cfg(target_os = "macos")]
fn details_of(path: &str, info: &DriveInfo) -> DriveDetails {
    let mut d = crate::core::drive_mac::info(path, info.file_system.as_deref(), &info.label);
    d.mount_point = Some(path.to_string());
    d
}

#[cfg(target_os = "linux")]
fn details_of(path: &str, info: &DriveInfo) -> DriveDetails {
    let root = shell::drive_root(path);
    let mut d = crate::core::drive_linux::details(
        path,
        root.as_ref().and_then(|r| r.source.as_deref()),
        root.as_ref().and_then(|r| r.fstype.as_deref()),
        &info.label,
    );
    d.mount_point = Some(path.to_string());
    d
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn details_of(path: &str, info: &DriveInfo) -> DriveDetails {
    DriveDetails {
        path: path.to_string(),
        label: info.label.clone(),
        file_system: info.file_system.clone(),
        drive_type: info.drive_type,
        total_bytes: info.total_bytes,
        free_bytes: info.free_bytes,
        letter: None,
        mount_point: Some(path.to_string()),
        device: None,
        bus: None,
        read_only: false,
    }
}

/// Renames the drive at `path` and answers its new row. The file system's own label rules are
/// checked FIRST, so a bad name never reaches the drive.
#[tauri::command(async)]
pub fn rename_drive(path: String, label: String) -> Result<DriveInfo, String> {
    let before = shell::drive_info(&path).ok_or_else(|| GONE.to_string())?;
    let fs = before.file_system.as_deref().unwrap_or("unknown");
    let label = check_label(fs, &label)?;
    let new_path = rename_on_os(&path, &label)?;
    let after = shell::drive_info(&new_path)
        .or_else(|| source_row(&path))
        .ok_or_else(|| {
            log::warn!("rename_drive: renamed, but the drive row is gone");
            GONE.to_string()
        })?;
    log::info!("rename_drive: ok, {} chars", label.chars().count());
    Ok(after)
}

/// Linux: the row of the same device, when the new label moved the mount point.
#[cfg(target_os = "linux")]
fn source_row(path: &str) -> Option<DriveInfo> {
    let source = shell::drive_root(path)?.source?;
    shell::drive_info_by_source(&source)
}

#[cfg(not(target_os = "linux"))]
fn source_row(_path: &str) -> Option<DriveInfo> {
    None
}

/// Does the rename; answers the path the drive has afterwards.
#[cfg(windows)]
fn rename_on_os(path: &str, label: &str) -> Result<String, String> {
    use crate::core::elevate::{run_elevated, ElevatedOp, CANCELLED};
    match crate::core::drive_windows::set_label(path, label) {
        Ok(()) => Ok(path.to_string()),
        // access denied: Explorer asks for administrator here, so Velkrys does too
        Err(e) if e.raw_os_error() == Some(5) => {
            let op = ElevatedOp::SetVolumeLabel {
                root: path.to_string(),
                label: label.to_string(),
            };
            match run_elevated(&op) {
                Ok(_) => Ok(path.to_string()),
                Err(e) if e == CANCELLED => Err(NOT_GIVEN.to_string()),
                Err(e) => Err(format!("Drive not renamed: {e}")),
            }
        }
        Err(e) if e.raw_os_error() == Some(1223) => Err(NOT_GIVEN.to_string()),
        Err(e) => Err(format!("Drive not renamed: {e}")),
    }
}

#[cfg(target_os = "macos")]
fn rename_on_os(path: &str, label: &str) -> Result<String, String> {
    crate::core::drive_mac::rename(path, label)
}

#[cfg(target_os = "linux")]
fn rename_on_os(path: &str, label: &str) -> Result<String, String> {
    let root = shell::drive_root(path).ok_or_else(|| GONE.to_string())?;
    let source = root
        .source
        .ok_or_else(|| "Drive not renamed: Linux did not say which device it is.".to_string())?;
    let fstype = root.fstype.unwrap_or_default();
    crate::core::drive_linux::rename(&source, &fstype, label)?;
    Ok(path.to_string())
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn rename_on_os(_path: &str, _label: &str) -> Result<String, String> {
    Err("Renaming a drive is not supported on this system.".into())
}

/// Ejects the drive at `path`. `Ok(true)`: it is gone. `Ok(false)`: still there (Windows has
/// shown its own reason).
#[tauri::command(async)]
pub fn eject_drive(path: String) -> Result<bool, String> {
    let gone = eject_on_os(&path)?;
    log::info!("eject_drive: gone: {gone}");
    Ok(gone)
}

#[cfg(windows)]
fn eject_on_os(path: &str) -> Result<bool, String> {
    crate::core::drive_windows::eject(path)
}

#[cfg(target_os = "macos")]
fn eject_on_os(path: &str) -> Result<bool, String> {
    crate::core::drive_mac::eject(path).map(|()| true)
}

#[cfg(target_os = "linux")]
fn eject_on_os(path: &str) -> Result<bool, String> {
    let source = shell::drive_root(path)
        .and_then(|r| r.source)
        .ok_or_else(|| "Linux did not say which device that is.".to_string())?;
    crate::core::drive_linux::eject(path, &source).map(|()| true)
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn eject_on_os(_path: &str) -> Result<bool, String> {
    Err("Ejecting is not supported on this system.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A label the file system forbids is refused with the rule's own sentence, before any OS
    /// call: the drive used is a real one (the first listed) and is never touched.
    #[test]
    fn rename_drive_refuses_a_bad_label_before_touching_the_drive() {
        let Some(first) = shell::drives()
            .into_iter()
            .find(|d| d.file_system.is_some())
        else {
            return; // a machine that lists no drive with a known file system
        };
        let too_long = "A".repeat(600);
        for bad in ["", "   ", too_long.as_str()] {
            let err = rename_drive(first.path.clone(), bad.to_string()).unwrap_err();
            assert!(
                err.contains("drive name") || err.contains("isn't supported"),
                "{err}"
            );
        }
    }

    #[test]
    fn a_drive_that_is_not_there_is_said_so() {
        let err = rename_drive("Q:\\no\\such\\drive".into(), "X".into()).unwrap_err();
        assert_eq!(err, GONE);
        assert_eq!(
            drive_details("Q:\\no\\such\\drive".into()).unwrap_err(),
            GONE
        );
    }
}
