//! What a drive IS, the same on every OS (plan 23): its type, the rules for renaming and
//! ejecting it, and the pure readers the Mac and Linux code feeds with command output.
//!
//! ⛔ Nothing here touches the machine. The per-OS files (`drive_windows`, `drive_mac`,
//! `drive_linux`) ask the OS and hand the answers to the functions below, so every rule is
//! tested on every OS. `folder_sizes::DriveKind` is a different, Windows-only thing and stays.

// Each OS build uses only its own part of this shared file (Windows the type map, Mac the plist
// readers, Linux the udisks helpers), and every part is tested on every OS.
#![allow(dead_code)]

use std::collections::HashMap;

use serde::Serialize;

/// What kind of drive it is. `External` is a USB / FireWire / SD disk that Windows still calls
/// fixed (a USB hard disk), so it can be ejected like a stick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DriveType {
    Fixed,
    External,
    Removable,
    Optical,
    Network,
    Ram,
    Unknown,
}

/// Everything the Properties pane shows. ⛔ Unknown is `None`, never 0 or "" (plan 10 B.2.4).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DriveDetails {
    pub path: String,
    pub label: String,
    pub file_system: Option<String>,
    pub drive_type: DriveType,
    pub total_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    /// `E:` on Windows only.
    pub letter: Option<String>,
    /// Off Windows: where it is mounted.
    pub mount_point: Option<String>,
    /// `/dev/disk4s1`, `/dev/sdb1` (off Windows).
    pub device: Option<String>,
    /// How it is connected: `USB`, `SATA`, `NVMe` …
    pub bus: Option<String>,
    pub read_only: bool,
}

/// Can the drive be ejected: removable, external and optical drives, never `/`.
pub fn ejectable(drive_type: DriveType, path: &str) -> bool {
    matches!(
        drive_type,
        DriveType::External | DriveType::Removable | DriveType::Optical
    ) && path != "/"
}

/// Whether the Mac startup disk can be renamed (plan 23, Q2). `true` until the Mac probe
/// (23.0.3) shows `diskutil rename /` cannot work even with an administrator password.
pub const MAC_BOOT_RENAME: bool = true;

/// Is a Rename row offered at all. Network and optical drives never; Linux `/` never; the Mac
/// startup disk only while `MAC_BOOT_RENAME`. A drive whose file system cannot be renamed DOES
/// get the row: clicking it explains why (`check_label`).
pub fn can_rename(drive_type: DriveType, path: &str) -> bool {
    can_rename_on(std::env::consts::OS, drive_type, path)
}

/// `can_rename` for a named OS (`std::env::consts::OS`), so a test can ask for any of them.
pub fn can_rename_on(os: &str, drive_type: DriveType, path: &str) -> bool {
    if matches!(drive_type, DriveType::Network | DriveType::Optical) {
        return false;
    }
    if os == "linux" && path == "/" {
        return false;
    }
    if os == "macos" && path == "/" && !MAC_BOOT_RENAME {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------------------------
// Label rules
// ---------------------------------------------------------------------------------------------

/// The characters FAT will not take in a volume label.
const FAT_BAD: &[char] = &[
    '*', '?', '.', ',', ';', ':', '/', '\\', '|', '+', '=', '<', '>', '[', ']', '"',
];

enum Limit {
    Chars(usize),
    Bytes(usize),
}

/// `(display name, limit, bad characters)` for a file system name as Windows, `diskutil` or
/// `/proc/self/mounts` spell it. `None`: not renameable by Velkrys.
fn label_rule(fs: &str) -> Option<(&'static str, Limit, &'static [char])> {
    let f = fs.trim().to_ascii_lowercase();
    Some(match f.as_str() {
        "ntfs" | "ntfs3" | "ntfs-3g" | "refs" => ("NTFS", Limit::Chars(32), &[][..]),
        "fat" | "fat32" | "fat16" | "vfat" | "msdos" | "ms-dos fat32" | "ms-dos fat16"
        | "ms-dos" => ("FAT", Limit::Chars(11), FAT_BAD),
        "exfat" => ("exFAT", Limit::Chars(15), &[][..]),
        "ext2" => ("ext2", Limit::Bytes(16), &[][..]),
        "ext3" => ("ext3", Limit::Bytes(16), &[][..]),
        "ext4" => ("ext4", Limit::Bytes(16), &[][..]),
        "btrfs" => ("Btrfs", Limit::Bytes(255), &[][..]),
        "apfs" => ("APFS", Limit::Chars(255), &[][..]),
        "hfs" | "hfs+" | "hfsplus" | "mac os extended" => ("HFS+", Limit::Chars(255), &[][..]),
        _ => return None,
    })
}

/// A volume label a drive of file system `fs` will take, trimmed; or the one plain sentence that
/// says why not. The same sentence goes to the notice.
pub fn check_label(fs: &str, label: &str) -> Result<String, String> {
    let label = label.trim();
    let Some((name, limit, bad)) = label_rule(fs) else {
        return Err(format!("Renaming isn't supported for {fs} drives."));
    };
    if label.is_empty() {
        return Err("A drive name can't be empty.".into());
    }
    if let Some(c) = label.chars().find(|c| c.is_control()) {
        return Err(format!("A drive name can't contain {c:?}."));
    }
    if let Some(c) = label.chars().find(|c| bad.contains(c)) {
        return Err(format!("A {name} drive name can't contain {c}."));
    }
    match limit {
        Limit::Chars(n) if label.chars().count() > n => Err(format!(
            "A {name} drive name can be at most {n} characters."
        )),
        Limit::Bytes(n) if label.len() > n => {
            Err(format!("A {name} drive name can be at most {n} bytes."))
        }
        _ => Ok(label.to_string()),
    }
}

// ---------------------------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------------------------

/// The type from `GetDriveTypeW` and, for a fixed drive, the storage bus. A USB, FireWire, SD or
/// MMC disk Windows calls fixed is `External`.
pub fn type_of(drive_type: u32, bus: Option<u32>) -> DriveType {
    match drive_type {
        2 => DriveType::Removable,
        3 => match bus {
            Some(7 | 4 | 0xC | 0xD) => DriveType::External,
            _ => DriveType::Fixed,
        },
        4 => DriveType::Network,
        5 => DriveType::Optical,
        6 => DriveType::Ram,
        _ => DriveType::Unknown,
    }
}

/// A `STORAGE_BUS_TYPE` as a word for the Properties pane.
pub fn bus_name(bus: u32) -> Option<&'static str> {
    Some(match bus {
        1 => "SCSI",
        2 => "ATAPI",
        3 => "ATA",
        4 => "FireWire",
        7 => "USB",
        8 => "RAID",
        0xA => "SAS",
        0xB => "SATA",
        0xC => "SD",
        0xD => "MMC",
        0x11 => "NVMe",
        _ => return None,
    })
}

// ---------------------------------------------------------------------------------------------
// macOS: `diskutil info -plist`
// ---------------------------------------------------------------------------------------------

/// One top-level value of a flat plist.
#[derive(Debug, Clone, PartialEq)]
pub enum PlistValue {
    Str(String),
    Int(i64),
    Bool(bool),
}

impl PlistValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            PlistValue::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PlistValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_int(&self) -> Option<i64> {
        match self {
            PlistValue::Int(n) => Some(*n),
            _ => None,
        }
    }
}

fn unescape_xml(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// The keys of the plist's top-level `<dict>` whose value is a string, integer, true or false.
/// Nested dicts and arrays, reals and dates are skipped (we use none). No XML crate: the shape
/// `diskutil` writes is fixed and flat.
pub fn read_flat_plist(xml: &str) -> HashMap<String, PlistValue> {
    let mut out = HashMap::new();
    let Some(start) = xml.find("<dict>") else {
        return out;
    };
    let mut rest = &xml[start + "<dict>".len()..];
    // depth of nested containers we are skipping inside a value
    loop {
        rest = rest.trim_start();
        let Some(after) = rest.strip_prefix("<key>") else {
            break; // `</dict>` or the end
        };
        let Some(end) = after.find("</key>") else {
            break;
        };
        let key = unescape_xml(&after[..end]);
        rest = after[end + "</key>".len()..].trim_start();
        let (value, tail) = take_value(rest);
        if let Some(v) = value {
            out.insert(key, v);
        }
        rest = tail;
    }
    out
}

/// One value element off the front of `s`: its value (when it is a kind we keep) and what is
/// left after it. Containers are skipped whole.
fn take_value(s: &str) -> (Option<PlistValue>, &str) {
    for (tag, make) in [
        (
            "string",
            (|t: &str| Some(PlistValue::Str(unescape_xml(t)))) as fn(&str) -> Option<PlistValue>,
        ),
        ("integer", |t: &str| {
            t.trim().parse().ok().map(PlistValue::Int)
        }),
    ] {
        if let Some(body) = s.strip_prefix(&format!("<{tag}>")) {
            if let Some(end) = body.find(&format!("</{tag}>")) {
                return (make(&body[..end]), &body[end + tag.len() + 3..]);
            }
        }
        if let Some(tail) = s.strip_prefix(&format!("<{tag}/>")) {
            return (make(""), tail);
        }
    }
    if let Some(tail) = s.strip_prefix("<true/>") {
        return (Some(PlistValue::Bool(true)), tail);
    }
    if let Some(tail) = s.strip_prefix("<false/>") {
        return (Some(PlistValue::Bool(false)), tail);
    }
    // a container (<dict>, <array>) or an element we do not keep: skip it balanced
    skip_element(s)
}

/// Skips one element, nested same-name elements included.
fn skip_element(s: &str) -> (Option<PlistValue>, &str) {
    let Some(open_end) = s.find('>') else {
        return (None, "");
    };
    let tag_text = &s[1..open_end];
    if tag_text.ends_with('/') {
        return (None, &s[open_end + 1..]);
    }
    let name = tag_text.split_whitespace().next().unwrap_or("");
    let (open, close) = (format!("<{name}>"), format!("</{name}>"));
    let mut depth = 1usize;
    let mut at = open_end + 1;
    while depth > 0 {
        let next_open = s[at..].find(&open).map(|i| i + at);
        let Some(next_close) = s[at..].find(&close).map(|i| i + at) else {
            return (None, "");
        };
        match next_open {
            Some(o) if o < next_close => {
                depth += 1;
                at = o + open.len();
            }
            _ => {
                depth -= 1;
                at = next_close + close.len();
            }
        }
    }
    (None, &s[at..])
}

/// The Mac type from `diskutil info -plist`: a network mount; an optical disc
/// (`OpticalMediaType`); `Internal == false` (external); a removable medium; else fixed.
pub fn mac_type(info: &HashMap<String, PlistValue>, network: bool) -> DriveType {
    let flag = |k: &str| info.get(k).and_then(PlistValue::as_bool);
    if network {
        DriveType::Network
    } else if info.contains_key("OpticalMediaType") || info.contains_key("OpticalDeviceType") {
        DriveType::Optical
    } else if flag("Internal") == Some(false) {
        DriveType::External
    } else if flag("RemovableMedia") == Some(true) || flag("Removable") == Some(true) {
        DriveType::Removable
    } else {
        DriveType::Fixed
    }
}

/// The Properties facts of a Mac volume from its `diskutil info -plist` answer. Sizes are left
/// `None`: the caller has the measured ones.
pub fn mac_details(
    info: &HashMap<String, PlistValue>,
    mount: &str,
    label: &str,
    network: bool,
) -> DriveDetails {
    let text = |k: &str| {
        info.get(k)
            .and_then(PlistValue::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    DriveDetails {
        path: mount.to_string(),
        label: text("VolumeName").unwrap_or_else(|| label.to_string()),
        file_system: text("FilesystemName"),
        drive_type: mac_type(info, network),
        total_bytes: None,
        free_bytes: None,
        letter: None,
        mount_point: Some(mount.to_string()),
        device: text("DeviceNode"),
        bus: text("BusProtocol"),
        read_only: info
            .get("WritableVolume")
            .and_then(PlistValue::as_bool)
            .is_some_and(|w| !w),
    }
}

/// The app that refused to let go, from `diskutil eject`'s complaint
/// (`… dissented by PID 412 (/Applications/Notes.app/Contents/MacOS/Notes)`): its path.
pub fn mac_dissenter(stderr: &str) -> Option<String> {
    let at = stderr.find("dissented by PID")?;
    let rest = &stderr[at..];
    let open = rest.find('(')?;
    let close = rest[open..].find(')')? + open;
    let path = rest[open + 1..close].trim();
    (!path.is_empty()).then(|| path.to_string())
}

/// `/Applications/Notes.app/Contents/MacOS/Notes` → `Notes`: the `.app` folder's name, else the
/// last component.
pub fn app_name(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let part = parts
        .iter()
        .find(|p| p.ends_with(".app"))
        .or(parts.last())
        .copied()
        .unwrap_or(path);
    part.strip_suffix(".app").unwrap_or(part).to_string()
}

/// The two `osascript` arguments-lists that rename the startup disk with the system's password
/// box. ⛔ The label is a trailing argument read as `item 1 of argv`, never written into the
/// script text, so no character in it can break out of the AppleScript or the shell
/// (`quoted form of` quotes it for the shell).
pub fn osascript_rename_args(label: &str) -> Vec<String> {
    vec![
        "-e".into(),
        "on run argv".into(),
        "-e".into(),
        "do shell script \"/usr/sbin/diskutil rename / \" & quoted form of (item 1 of argv) with administrator privileges".into(),
        "-e".into(),
        "end run".into(),
        label.to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// Linux: sysfs, udisks2 over busctl
// ---------------------------------------------------------------------------------------------

/// The udisks2 object path of a block device (`sdb1`, `/dev/nvme0n1p1`, `dm-0`): every byte that
/// is not a letter or digit becomes `_` and its two hex digits (`-` is `_2d`).
pub fn udisks_block_path(dev: &str) -> String {
    let name = dev.strip_prefix("/dev/").unwrap_or(dev);
    let mut out = String::from("/org/freedesktop/UDisks2/block_devices/");
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() {
            out.push(b as char);
        } else {
            out.push_str(&format!("_{b:02x}"));
        }
    }
    out
}

/// The type of a Linux block device: `/dev/sr*` optical; a removable disk removable; one whose
/// sysfs path runs through a USB or FireWire host external; else fixed.
pub fn linux_type(sysfs_path: &str, removable: bool, dev: &str) -> DriveType {
    let name = dev.strip_prefix("/dev/").unwrap_or(dev);
    if name.starts_with("sr") {
        DriveType::Optical
    } else if removable {
        DriveType::Removable
    } else if sysfs_path.contains("/usb") || sysfs_path.contains("/firewire") {
        DriveType::External
    } else {
        DriveType::Fixed
    }
}

/// The whole disk a partition belongs to: `sdb1` → `sdb`, `nvme0n1p2` → `nvme0n1`,
/// `mmcblk0p1` → `mmcblk0`. A name that is not a partition comes back as it is.
pub fn disk_of(dev: &str) -> String {
    let name = dev.strip_prefix("/dev/").unwrap_or(dev);
    let digits_off = |s: &str| s.trim_end_matches(|c: char| c.is_ascii_digit()).to_string();
    if name.starts_with("nvme") || name.starts_with("mmcblk") || name.starts_with("loop") {
        if let Some(p) = name.rfind('p') {
            let (head, tail) = name.split_at(p);
            if tail.len() > 1 && tail[1..].bytes().all(|b| b.is_ascii_digit()) && !head.is_empty() {
                return head.to_string();
            }
        }
        name.to_string()
    } else if ["sd", "hd", "vd", "xvd"]
        .iter()
        .any(|p| name.starts_with(p))
    {
        digits_off(name)
    } else {
        name.to_string()
    }
}

/// The devices from a mount table that are partitions of the same disk as `dev`, in table
/// order, each once.
pub fn partitions_of_disk(mounts: &str, dev: &str) -> Vec<String> {
    let disk = disk_of(dev);
    let mut out: Vec<String> = Vec::new();
    for line in mounts.lines() {
        let Some(source) = line.split_whitespace().next() else {
            continue;
        };
        if source.starts_with("/dev/")
            && disk_of(source) == disk
            && !out.iter().any(|s| s == source)
        {
            out.push(source.to_string());
        }
    }
    out
}

/// `/dev/disk/by-label` names escape every unsafe byte as `\xNN` (a space is `\x20`).
pub fn decode_by_label(name: &str) -> String {
    let b = name.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && b.get(i + 1) == Some(&b'x') && i + 4 <= b.len() {
            let hex = std::str::from_utf8(&b[i + 2..i + 4]).ok();
            if let Some(n) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(n);
                i += 4;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// What the person was doing when udisks said no.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveAction {
    Rename,
    Eject,
}

/// One plain sentence for a failed `busctl` call to udisks2. `label` is the drive's name.
pub fn udisks_error_sentence(stderr: &str, label: &str, action: DriveAction) -> String {
    let lower = stderr.to_ascii_lowercase();
    let did = match action {
        DriveAction::Rename => "Drive not renamed".to_string(),
        DriveAction::Eject => format!("{label} not ejected"),
    };
    if stderr.contains("NotAuthorized") {
        format!("{did}: permission was not given.")
    } else if stderr.contains("DeviceBusy") || lower.contains("target is busy") {
        format!("{label} is in use. Close any files on it and try again.")
    } else if lower.contains("unmount") && action == DriveAction::Rename {
        format!("Unmount {label} first to rename this drive.")
    } else if stderr.contains("NotSupported") {
        match action {
            DriveAction::Rename => "Renaming isn't supported for this drive's file system.".into(),
            DriveAction::Eject => format!("{label} can't be ejected."),
        }
    } else if stderr.contains("ServiceUnknown")
        || lower.contains("failed to connect")
        || lower.contains("no such file or directory")
    {
        "Eject and Rename need the udisks2 service (Ubuntu has it by default).".into()
    } else {
        let line = stderr.lines().map(str::trim).find(|l| !l.is_empty());
        match line {
            Some(l) => format!("{did}: {l}"),
            None => format!("{did}."),
        }
    }
}

/// The object path in `busctl get-property` output: `o "/org/freedesktop/UDisks2/drives/X"`.
pub fn busctl_object_path(out: &str) -> Option<String> {
    let start = out.find('"')? + 1;
    let end = out[start..].find('"')? + start;
    let p = &out[start..end];
    (p.starts_with('/') && p != "/").then(|| p.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ejectable_only_for_removable_external_optical_and_never_root() {
        for t in [
            DriveType::Removable,
            DriveType::External,
            DriveType::Optical,
        ] {
            assert!(ejectable(t, "E:\\"), "{t:?}");
            assert!(!ejectable(t, "/"), "{t:?} on /");
        }
        for t in [
            DriveType::Fixed,
            DriveType::Network,
            DriveType::Ram,
            DriveType::Unknown,
        ] {
            assert!(!ejectable(t, "D:\\"), "{t:?}");
        }
    }

    #[test]
    fn network_optical_and_linux_root_offer_no_rename() {
        assert!(!can_rename_on("windows", DriveType::Network, "Z:\\"));
        assert!(!can_rename_on("macos", DriveType::Optical, "/Volumes/DVD"));
        assert!(!can_rename_on("linux", DriveType::Fixed, "/"));
        assert!(can_rename_on("linux", DriveType::Fixed, "/mnt/data"));
        assert!(can_rename_on("windows", DriveType::Fixed, "C:\\"));
        assert!(can_rename_on("windows", DriveType::Removable, "E:\\"));
        // the Mac startup disk follows the Q2 switch
        assert_eq!(
            can_rename_on("macos", DriveType::Fixed, "/"),
            MAC_BOOT_RENAME
        );
        // an unknown file system does not hide the row
        assert!(can_rename_on("linux", DriveType::Removable, "/media/u/X"));
    }

    #[test]
    fn label_rules_per_file_system() {
        // trimmed
        assert_eq!(check_label("NTFS", "  Photos  "), Ok("Photos".into()));
        // NTFS 32
        assert!(check_label("NTFS", &"a".repeat(32)).is_ok());
        assert_eq!(
            check_label("NTFS", &"a".repeat(33)),
            Err("A NTFS drive name can be at most 32 characters.".into())
        );
        // FAT 11 and its bad characters
        assert!(check_label("FAT32", "USB STICK").is_ok());
        assert_eq!(
            check_label("vfat", "TWELVE CHARS"),
            Err("A FAT drive name can be at most 11 characters.".into())
        );
        for c in "*?.,;:/\\|+=<>[]\"".chars() {
            let err = check_label("FAT", &format!("A{c}B")).unwrap_err();
            assert!(err.contains("can't contain"), "{c}: {err}");
        }
        assert!(check_label("msdos", "A-B_C").is_ok());
        // exFAT 15
        assert!(check_label("exFAT", &"a".repeat(15)).is_ok());
        assert!(check_label("exfat", &"a".repeat(16)).is_err());
        assert!(check_label("exfat", "a.b").is_ok());
        // ext 16 bytes, so a multibyte name is over sooner than it looks
        assert!(check_label("ext4", &"a".repeat(16)).is_ok());
        assert!(check_label("ext4", &"a".repeat(17)).is_err());
        let multibyte = "ü".repeat(9); // 9 chars, 18 bytes
        assert_eq!(
            check_label("ext4", &multibyte),
            Err("A ext4 drive name can be at most 16 bytes.".into())
        );
        assert!(check_label("ext3", &"ü".repeat(8)).is_ok());
        // btrfs 255 bytes; APFS and HFS+ 255
        assert!(check_label("btrfs", &"a".repeat(255)).is_ok());
        assert!(check_label("btrfs", &"a".repeat(256)).is_err());
        assert!(check_label("APFS", &"a".repeat(255)).is_ok());
        assert!(check_label("APFS", &"a".repeat(256)).is_err());
        assert!(check_label("Mac OS Extended", "Backup").is_ok());
        // anything else
        assert_eq!(
            check_label("xfs", "X"),
            Err("Renaming isn't supported for xfs drives.".into())
        );
        assert_eq!(
            check_label("iso9660", "X"),
            Err("Renaming isn't supported for iso9660 drives.".into())
        );
        // empty and control characters
        assert!(check_label("NTFS", "   ").is_err());
        assert!(check_label("NTFS", "a\nb").is_err());
    }

    #[test]
    fn windows_drive_type_and_bus_map_to_type() {
        assert_eq!(type_of(2, None), DriveType::Removable);
        assert_eq!(type_of(3, None), DriveType::Fixed);
        assert_eq!(type_of(3, Some(0xB)), DriveType::Fixed); // SATA
        assert_eq!(type_of(3, Some(0x11)), DriveType::Fixed); // NVMe
        assert_eq!(type_of(3, Some(7)), DriveType::External); // USB
        assert_eq!(type_of(3, Some(4)), DriveType::External); // 1394
        assert_eq!(type_of(3, Some(0xC)), DriveType::External); // SD
        assert_eq!(type_of(3, Some(0xD)), DriveType::External); // MMC
        assert_eq!(type_of(4, None), DriveType::Network);
        assert_eq!(type_of(5, None), DriveType::Optical);
        assert_eq!(type_of(6, None), DriveType::Ram);
        assert_eq!(type_of(0, None), DriveType::Unknown);
        assert_eq!(type_of(1, None), DriveType::Unknown);
        // the bus is only asked for a fixed drive: a removable one stays removable
        assert_eq!(type_of(2, Some(7)), DriveType::Removable);
        assert_eq!(bus_name(7), Some("USB"));
        assert_eq!(bus_name(0xB), Some("SATA"));
        assert_eq!(bus_name(0x99), None);
    }

    /// (assumed) shaped like `diskutil info -plist` output; replaced by the Mac probe's real
    /// files (plan 23.0.3) in `fixtures/diskutil-info-*.plist`.
    const USB_PLIST: &str = include_str!("fixtures/diskutil-info-usb.plist");
    const BOOT_PLIST: &str = include_str!("fixtures/diskutil-info-boot.plist");

    #[test]
    fn diskutil_plist_reader_reads_the_keys_we_use() {
        let usb = read_flat_plist(USB_PLIST);
        assert_eq!(usb["VolumeName"].as_str(), Some("USB STICK"));
        assert_eq!(usb["FilesystemName"].as_str(), Some("MS-DOS FAT32"));
        assert_eq!(usb["DeviceNode"].as_str(), Some("/dev/disk4s1"));
        assert_eq!(usb["BusProtocol"].as_str(), Some("USB"));
        assert_eq!(usb["Internal"].as_bool(), Some(false));
        assert_eq!(usb["RemovableMedia"].as_bool(), Some(true));
        assert_eq!(usb["TotalSize"].as_int(), Some(31_914_983_424));
        // a nested dict is skipped whole, and the keys after it are still read
        assert!(!usb.contains_key("NestedKey"));
        assert_eq!(
            usb["VolumeUUID"].as_str(),
            Some("1C2D3E4F-0000-4000-8000-ABCDEF012345")
        );
        // entities are decoded
        assert_eq!(usb["MountPoint"].as_str(), Some("/Volumes/USB STICK"));
        assert_eq!(usb["Note"].as_str(), Some("R&D <1>"));
        let boot = read_flat_plist(BOOT_PLIST);
        assert_eq!(boot["FilesystemName"].as_str(), Some("APFS"));
        assert_eq!(boot["Internal"].as_bool(), Some(true));
        assert_eq!(boot["MountPoint"].as_str(), Some("/"));
        assert!(read_flat_plist("not a plist").is_empty());
    }

    #[test]
    fn mac_type_from_diskutil() {
        let usb = read_flat_plist(USB_PLIST);
        let boot = read_flat_plist(BOOT_PLIST);
        assert_eq!(mac_type(&usb, false), DriveType::External);
        assert_eq!(mac_type(&boot, false), DriveType::Fixed);
        assert_eq!(mac_type(&usb, true), DriveType::Network);
        let mut sd = boot.clone();
        sd.insert("RemovableMedia".into(), PlistValue::Bool(true));
        assert_eq!(mac_type(&sd, false), DriveType::Removable);
        let mut dvd = boot.clone();
        dvd.insert("OpticalMediaType".into(), PlistValue::Str("DVD-ROM".into()));
        assert_eq!(mac_type(&dvd, false), DriveType::Optical);
        let d = mac_details(&usb, "/Volumes/USB STICK", "x", false);
        assert_eq!(d.label, "USB STICK");
        assert_eq!(d.file_system.as_deref(), Some("MS-DOS FAT32"));
        assert_eq!(d.device.as_deref(), Some("/dev/disk4s1"));
        assert_eq!(d.bus.as_deref(), Some("USB"));
        assert_eq!(d.mount_point.as_deref(), Some("/Volumes/USB STICK"));
        assert_eq!((d.total_bytes, d.letter), (None, None));
    }

    #[test]
    fn diskutil_dissenter_names_the_app() {
        let err = "Volume USB STICK on disk4s1 failed to unmount: dissented by PID 412 \
                   (/System/Applications/TextEdit.app/Contents/MacOS/TextEdit)\n";
        let path = mac_dissenter(err).unwrap();
        assert_eq!(
            path,
            "/System/Applications/TextEdit.app/Contents/MacOS/TextEdit"
        );
        assert_eq!(app_name(&path), "TextEdit");
        assert_eq!(app_name("/usr/bin/vim"), "vim");
        assert_eq!(mac_dissenter("Unmount failed for /Volumes/X"), None);
    }

    #[test]
    fn the_osascript_line_quotes_the_label() {
        let label = r#"It's "mine" $(rm -rf ~) \ `x`"#;
        let args = osascript_rename_args(label);
        // the label is only ever the last argument, verbatim
        assert_eq!(args.last().map(String::as_str), Some(label));
        let script = args[..args.len() - 1].join("\n");
        assert!(!script.contains("rm -rf"), "{script}");
        assert!(!script.contains("mine"), "{script}");
        assert!(script.contains("quoted form of (item 1 of argv)"));
        assert!(script.contains("with administrator privileges"));
        assert!(script.contains("/usr/sbin/diskutil rename / "));
    }

    #[test]
    fn udisks_object_path_escapes_device_names() {
        let base = "/org/freedesktop/UDisks2/block_devices/";
        assert_eq!(udisks_block_path("/dev/sdb1"), format!("{base}sdb1"));
        assert_eq!(
            udisks_block_path("/dev/nvme0n1p1"),
            format!("{base}nvme0n1p1")
        );
        assert_eq!(udisks_block_path("mmcblk0p1"), format!("{base}mmcblk0p1"));
        assert_eq!(udisks_block_path("/dev/dm-0"), format!("{base}dm_2d0"));
    }

    #[test]
    fn linux_type_from_sysfs() {
        let usb = "/sys/devices/pci0000:00/0000:00:14.0/usb2/2-1/2-1:1.0/host6/target6:0:0/6:0:0:0/block/sdb";
        let sata = "/sys/devices/pci0000:00/0000:00:17.0/ata1/host0/target0:0:0/0:0:0:0/block/sda";
        assert_eq!(linux_type(usb, true, "/dev/sdb1"), DriveType::Removable);
        assert_eq!(linux_type(usb, false, "/dev/sdb1"), DriveType::External);
        assert_eq!(linux_type(sata, false, "/dev/sda2"), DriveType::Fixed);
        assert_eq!(linux_type("", false, "/dev/sr0"), DriveType::Optical);
        assert_eq!(
            linux_type("/sys/x/firewire/fw0", false, "sdc1"),
            DriveType::External
        );
        assert_eq!(disk_of("/dev/sdb1"), "sdb");
        assert_eq!(disk_of("/dev/nvme0n1p2"), "nvme0n1");
        assert_eq!(disk_of("mmcblk0p1"), "mmcblk0");
        assert_eq!(disk_of("/dev/sr0"), "sr0");
        assert_eq!(disk_of("/dev/dm-0"), "dm-0");
    }

    #[test]
    fn by_label_names_are_decoded() {
        assert_eq!(decode_by_label("My\\x20Disk"), "My Disk");
        assert_eq!(decode_by_label("PLAIN"), "PLAIN");
        assert_eq!(decode_by_label("caf\\xc3\\xa9"), "café");
        assert_eq!(decode_by_label("odd\\xZZ"), "odd\\xZZ");
        assert_eq!(decode_by_label("end\\x2"), "end\\x2");
    }

    #[test]
    fn udisks_errors_become_sentences() {
        let dismissed = "Call failed: GDBus.Error:org.freedesktop.UDisks2.Error.NotAuthorizedDismissed: Not authorized";
        assert_eq!(
            udisks_error_sentence(dismissed, "STICK", DriveAction::Rename),
            "Drive not renamed: permission was not given."
        );
        assert_eq!(
            udisks_error_sentence(dismissed, "STICK", DriveAction::Eject),
            "STICK not ejected: permission was not given."
        );
        let busy = "org.freedesktop.UDisks2.Error.DeviceBusy: target is busy";
        assert_eq!(
            udisks_error_sentence(busy, "STICK", DriveAction::Eject),
            "STICK is in use. Close any files on it and try again."
        );
        assert_eq!(
            udisks_error_sentence("...Error.NotSupported: nope", "S", DriveAction::Rename),
            "Renaming isn't supported for this drive's file system."
        );
        assert_eq!(
            udisks_error_sentence("Failed to connect to bus", "S", DriveAction::Rename),
            "Eject and Rename need the udisks2 service (Ubuntu has it by default)."
        );
        assert_eq!(
            udisks_error_sentence("weird\n", "S", DriveAction::Rename),
            "Drive not renamed: weird"
        );
        assert_eq!(
            udisks_error_sentence("", "S", DriveAction::Eject),
            "S not ejected."
        );
        assert_eq!(
            busctl_object_path("o \"/org/freedesktop/UDisks2/drives/Vendor_Disk\"\n"),
            Some("/org/freedesktop/UDisks2/drives/Vendor_Disk".into())
        );
        assert_eq!(busctl_object_path("o \"/\"\n"), None);
    }

    #[test]
    fn the_partitions_of_one_disk_are_found_in_a_mount_table() {
        let table = "\
/dev/sda2 / ext4 rw 0 0
/dev/sdb1 /media/u/ONE vfat rw 0 0
/dev/sdb2 /media/u/TWO ext4 rw 0 0
/dev/sdb1 /mnt/again vfat rw 0 0
/dev/sdc1 /media/u/OTHER exfat rw 0 0
/dev/nvme0n1p3 /home ext4 rw 0 0
tmpfs /run tmpfs rw 0 0
";
        assert_eq!(
            partitions_of_disk(table, "/dev/sdb1"),
            ["/dev/sdb1", "/dev/sdb2"]
        );
        assert_eq!(partitions_of_disk(table, "/dev/sdc1"), ["/dev/sdc1"]);
        assert_eq!(
            partitions_of_disk(table, "/dev/nvme0n1p1"),
            ["/dev/nvme0n1p3"]
        );
    }

    #[test]
    fn details_serialise_camel_case_with_nulls_for_unknowns() {
        let d = DriveDetails {
            path: "E:\\".into(),
            label: "USB STICK".into(),
            file_system: Some("FAT32".into()),
            drive_type: DriveType::Removable,
            total_bytes: None,
            free_bytes: None,
            letter: Some("E:".into()),
            mount_point: None,
            device: None,
            bus: None,
            read_only: false,
        };
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["driveType"], "removable");
        assert_eq!(v["fileSystem"], "FAT32");
        assert!(v["totalBytes"].is_null() && v["mountPoint"].is_null());
        assert_eq!(v["readOnly"], false);
    }
}
