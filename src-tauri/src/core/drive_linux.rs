//! The Linux drive facts and actions (plan 23.4): the label from `/dev/disk/by-label`, the type
//! from sysfs, and rename / eject through udisks2 over `busctl` (polkit asks for a password on
//! its own with `--allow-interactive-authorization`).
//!
//! The pure helpers live in `core/drive.rs`; compiled on Linux and in tests.
// Allowed on Linux too until the drive menu commands call rename/eject (plan 23), like
// drive_mac.rs and drive_windows.rs; otherwise clippy -D warnings fails on Linux.
#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

use crate::core::drive::{
    busctl_object_path, check_label, decode_by_label, disk_of, linux_type, partitions_of_disk,
    udisks_block_path, udisks_error_sentence, DriveAction, DriveDetails, DriveType,
};

const BUSCTL: &str = "/usr/bin/busctl";
const UDISKS: &str = "org.freedesktop.UDisks2";

/// The label `/dev/disk/by-label` gives the device `source`, if any.
fn by_label(source: &str) -> Option<String> {
    let want = std::fs::canonicalize(source).ok()?;
    for e in std::fs::read_dir("/dev/disk/by-label").ok()?.flatten() {
        if std::fs::canonicalize(e.path()).ok().as_deref() == Some(want.as_path()) {
            return Some(decode_by_label(&e.file_name().to_string_lossy()));
        }
    }
    None
}

/// The type of the device `source` from sysfs: the disk's real path (is it on USB?) and its
/// `removable` flag.
fn type_from_sysfs(source: &str) -> DriveType {
    let disk = disk_of(source);
    let node = Path::new("/sys/class/block").join(&disk);
    let real = std::fs::canonicalize(&node)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let removable = std::fs::read_to_string(node.join("removable"))
        .map(|s| s.trim() == "1")
        .unwrap_or(false);
    linux_type(&real, removable, source)
}

/// `fuseblk` (ntfs-3g) and `ntfs3` are both shown as NTFS.
fn shown_fs(fstype: &str) -> String {
    match fstype {
        "fuseblk" | "ntfs3" | "ntfs-3g" => "NTFS".to_string(),
        other => other.to_string(),
    }
}

/// The facts of the drive mounted at `root`, from the mount table row (`source`, `fstype`).
pub fn details(
    root: &str,
    source: Option<&str>,
    fstype: Option<&str>,
    fallback_label: &str,
) -> DriveDetails {
    let label = if root == "/" {
        None
    } else {
        source.and_then(by_label)
    }
    .unwrap_or_else(|| fallback_label.to_string());
    let network = fstype
        .is_some_and(|f| matches!(f, "cifs" | "smb3" | "nfs" | "nfs4" | "sshfs" | "fuse.sshfs"));
    let dev = source.filter(|s| s.starts_with("/dev/"));
    let drive_type = if network {
        DriveType::Network
    } else {
        dev.map(type_from_sysfs).unwrap_or(DriveType::Fixed)
    };
    DriveDetails {
        path: root.to_string(),
        label,
        file_system: fstype.map(shown_fs),
        drive_type,
        total_bytes: None,
        free_bytes: None,
        letter: None,
        mount_point: Some(root.to_string()),
        device: dev.map(str::to_string),
        bus: None,
        read_only: false,
    }
}

fn busctl(args: &[&str]) -> Result<String, String> {
    let out = Command::new(BUSCTL)
        .args(args)
        .output()
        .map_err(|e| format!("Could not run busctl: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

/// Give the drive on `source` the name `label`. The file system's own rules are checked first.
pub fn rename(source: &str, fstype: &str, label: &str) -> Result<(), String> {
    let label = check_label(&shown_fs(fstype), label)?;
    let object = udisks_block_path(source);
    busctl(&[
        "--allow-interactive-authorization",
        "call",
        "--system",
        UDISKS,
        &object,
        "org.freedesktop.UDisks2.Filesystem",
        "SetLabel",
        "sa{sv}",
        &label,
        "0",
    ])
    .map(|_| ())
    .map_err(|e| udisks_error_sentence(&e, &label, DriveAction::Rename))
}

/// Unmount every mounted partition of the disk behind `source`, then power the disk off (an
/// optical drive is ejected instead). `root` is the mount point, used for the name in messages.
pub fn eject(root: &str, source: &str) -> Result<(), String> {
    let name = root.rsplit('/').next().unwrap_or(root).to_string();
    let mounts = std::fs::read_to_string("/proc/self/mounts").unwrap_or_default();
    let mut parts = partitions_of_disk(&mounts, source);
    if parts.is_empty() {
        parts.push(source.to_string());
    }
    for p in &parts {
        busctl(&[
            "--allow-interactive-authorization",
            "call",
            "--system",
            UDISKS,
            &udisks_block_path(p),
            "org.freedesktop.UDisks2.Filesystem",
            "Unmount",
            "a{sv}",
            "0",
        ])
        .map_err(|e| udisks_error_sentence(&e, &name, DriveAction::Eject))?;
    }
    let block = udisks_block_path(source);
    let out = busctl(&[
        "--system",
        "get-property",
        UDISKS,
        &block,
        "org.freedesktop.UDisks2.Block",
        "Drive",
    ])
    .map_err(|e| udisks_error_sentence(&e, &name, DriveAction::Eject))?;
    let drive = busctl_object_path(&out).ok_or_else(|| format!("{name} can't be ejected."))?;
    let method = if source.starts_with("/dev/sr") {
        "Eject"
    } else {
        "PowerOff"
    };
    busctl(&[
        "--allow-interactive-authorization",
        "call",
        "--system",
        UDISKS,
        &drive,
        "org.freedesktop.UDisks2.Drive",
        method,
        "a{sv}",
        "0",
    ])
    .map(|_| ())
    .map_err(|e| udisks_error_sentence(&e, &name, DriveAction::Eject))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntfs_drivers_are_shown_as_ntfs() {
        assert_eq!(shown_fs("fuseblk"), "NTFS");
        assert_eq!(shown_fs("ntfs3"), "NTFS");
        assert_eq!(shown_fs("ext4"), "ext4");
    }

    #[test]
    fn a_network_mount_is_a_network_drive() {
        let d = details("/mnt/share", Some("//host/share"), Some("cifs"), "share");
        assert_eq!(d.drive_type, DriveType::Network);
        assert_eq!(d.label, "share");
        assert_eq!(d.device, None);
    }

    #[test]
    fn root_keeps_the_fallback_label() {
        let d = details("/", Some("/dev/nonexistent1"), Some("ext4"), "File System");
        assert_eq!(d.label, "File System");
        assert_eq!(d.file_system.as_deref(), Some("ext4"));
    }

    /// Needs a mounted USB stick: `VELKRYS_LIVE_MOUNT` and `VELKRYS_LIVE_DEV`.
    #[test]
    #[ignore = "live: needs Linux and a USB stick"]
    fn live_linux_details() {
        let mount = std::env::var("VELKRYS_LIVE_MOUNT").expect("set VELKRYS_LIVE_MOUNT");
        let dev = std::env::var("VELKRYS_LIVE_DEV").expect("set VELKRYS_LIVE_DEV");
        let d = details(&mount, Some(&dev), Some("vfat"), "x");
        assert_ne!(d.drive_type, DriveType::Fixed, "{d:?}");
    }

    /// Renames the stick and back; needs udisks2 and polkit.
    #[test]
    #[ignore = "live: needs Linux, udisks2 and a USB stick"]
    fn live_linux_rename_round_trip() {
        let dev = std::env::var("VELKRYS_LIVE_DEV").expect("set VELKRYS_LIVE_DEV");
        let fs = std::env::var("VELKRYS_LIVE_FS").unwrap_or_else(|_| "vfat".into());
        rename(&dev, &fs, "VELKRYSTMP").unwrap();
        rename(&dev, &fs, "VELKRYSOLD").unwrap();
    }

    /// Unmounts and powers off the stick.
    #[test]
    #[ignore = "live: needs Linux, udisks2 and a USB stick"]
    fn live_linux_eject() {
        let mount = std::env::var("VELKRYS_LIVE_MOUNT").expect("set VELKRYS_LIVE_MOUNT");
        let dev = std::env::var("VELKRYS_LIVE_DEV").expect("set VELKRYS_LIVE_DEV");
        eject(&mount, &dev).unwrap();
    }
}
