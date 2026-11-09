//! The macOS drive facts and actions (plan 23.3): `diskutil info -plist` for the type, file
//! system and device, `diskutil rename` and `diskutil eject` for the two actions.
//!
//! The pure readers live in `core/drive.rs`; this file only runs the commands and hands the
//! output to them. Compiled on macOS and in tests, so it must build on every OS.
// Allowed on macOS too until the drive menu commands call rename/eject (plan 23), like
// drive_windows.rs; otherwise clippy -D warnings fails on a Mac.
#![allow(dead_code)]

use std::process::Command;

use crate::core::drive::{
    app_name, check_label, mac_details, mac_dissenter, osascript_rename_args, read_flat_plist,
    DriveDetails, DriveType,
};

const DISKUTIL: &str = "/usr/sbin/diskutil";
const OSASCRIPT: &str = "/usr/bin/osascript";

fn run(program: &str, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("Could not run {program}: {e}"))
}

fn bare(mount: &str, label: &str, fstype: Option<&str>, drive_type: DriveType) -> DriveDetails {
    DriveDetails {
        path: mount.to_string(),
        label: label.to_string(),
        file_system: fstype.map(str::to_string),
        drive_type,
        total_bytes: None,
        free_bytes: None,
        letter: None,
        mount_point: Some(mount.to_string()),
        device: None,
        bus: None,
        read_only: false,
    }
}

/// The facts of the volume mounted at `mount`. `/` is never asked (no `diskutil`): it is the
/// startup disk, so fixed, with the file system the mount table named.
pub fn info(mount: &str, fstype: Option<&str>, fallback_label: &str) -> DriveDetails {
    let network = fstype.is_some_and(|f| matches!(f, "smbfs" | "nfs" | "afpfs" | "webdav"));
    if mount == "/" {
        return bare(mount, fallback_label, fstype, DriveType::Fixed);
    }
    let mut d = match run(DISKUTIL, &["info", "-plist", mount]) {
        Ok(out) if out.status.success() => {
            let xml = String::from_utf8_lossy(&out.stdout);
            mac_details(&read_flat_plist(&xml), mount, fallback_label, network)
        }
        _ => {
            let t = if network {
                DriveType::Network
            } else {
                DriveType::Fixed
            };
            bare(mount, fallback_label, None, t)
        }
    };
    if d.file_system.is_none() {
        d.file_system = fstype.map(str::to_string);
    }
    d
}

/// The mount point of the volume now called `label`.
fn mount_of_label(label: &str) -> Option<String> {
    let out = run(DISKUTIL, &["info", "-plist", &format!("/Volumes/{label}")]).ok()?;
    if !out.status.success() {
        return None;
    }
    let map = read_flat_plist(&String::from_utf8_lossy(&out.stdout));
    map.get("MountPoint")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

/// Rename the volume at `mount` and return its new mount point. `/` goes through `osascript`
/// so macOS asks for the administrator password itself; the label travels as an argument.
pub fn rename(mount: &str, label: &str) -> Result<String, String> {
    let d = info(mount, None, "");
    let fs = d.file_system.as_deref().unwrap_or("APFS");
    let label = check_label(fs, label)?;
    if mount == "/" {
        let args = osascript_rename_args(&label);
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = run(OSASCRIPT, &refs)?;
        if out.status.success() {
            return Ok("/".into());
        }
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(if err.contains("-128") {
            "Drive not renamed: administrator permission was not given.".into()
        } else {
            format!(
                "Drive not renamed: {}",
                err.lines().next().unwrap_or("unknown error").trim()
            )
        });
    }
    let out = run(DISKUTIL, &["rename", mount, &label])?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let line = err.lines().map(str::trim).find(|l| !l.is_empty());
        return Err(format!(
            "Drive not renamed: {}",
            line.unwrap_or("diskutil said no")
        ));
    }
    Ok(mount_of_label(&label).unwrap_or_else(|| format!("/Volumes/{label}")))
}

/// Eject the volume at `mount`. A refusal names the app that holds it open.
pub fn eject(mount: &str) -> Result<(), String> {
    let label = info(mount, None, "").label.trim().to_string();
    let name = if label.is_empty() {
        mount.rsplit('/').next().unwrap_or(mount).to_string()
    } else {
        label
    };
    let out = run(DISKUTIL, &["eject", mount])?;
    if out.status.success() {
        return Ok(());
    }
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    match mac_dissenter(&text) {
        Some(path) => Err(format!(
            "{name} is in use by {}. Quit it and try again.",
            app_name(&path)
        )),
        None => Err(format!("{name} could not be ejected.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_is_never_asked_to_diskutil() {
        let d = info("/", Some("apfs"), "Startup");
        assert_eq!(d.drive_type, DriveType::Fixed);
        assert_eq!(d.file_system.as_deref(), Some("apfs"));
        assert_eq!(d.label, "Startup");
    }

    /// Needs a Mac; `VELKRYS_LIVE_MOUNT` names a mounted volume (default `/`).
    #[test]
    #[ignore = "live: needs a Mac"]
    fn live_mac_details() {
        let mount = std::env::var("VELKRYS_LIVE_MOUNT").unwrap_or_else(|_| "/".into());
        let d = info(&mount, None, "x");
        assert!(d.file_system.is_some(), "{d:?}");
    }

    /// Renames a scratch volume at `VELKRYS_LIVE_MOUNT` and back.
    #[test]
    #[ignore = "live: needs a Mac and a scratch volume"]
    fn live_mac_rename_round_trip() {
        let mount = std::env::var("VELKRYS_LIVE_MOUNT").expect("set VELKRYS_LIVE_MOUNT");
        let old = info(&mount, None, "").label;
        let moved = rename(&mount, "VelkrysTmp").unwrap();
        let back = rename(&moved, &old).unwrap();
        assert!(back.ends_with(&old));
    }

    /// Ejects the volume at `VELKRYS_LIVE_MOUNT`.
    #[test]
    #[ignore = "live: needs a Mac and a scratch volume"]
    fn live_mac_eject() {
        let mount = std::env::var("VELKRYS_LIVE_MOUNT").expect("set VELKRYS_LIVE_MOUNT");
        eject(&mount).unwrap();
    }
}
