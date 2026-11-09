//! The drive facts Windows has (plan 23.2): label, file system, read-only flag, sizes, the type
//! (`GetDriveTypeW` plus the storage bus for a "fixed" USB disk), renaming a volume, and the
//! shell's own Eject.
//!
//! The pure mapping (`type_of`, `bus_name`) lives in `core/drive.rs`, tested on every OS.

use std::io;
use std::os::windows::ffi::OsStrExt;

use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    SetVolumeLabelW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Ioctl::{
    PropertyStandardQuery, StorageDeviceProperty, IOCTL_STORAGE_QUERY_PROPERTY,
    STORAGE_DEVICE_DESCRIPTOR, STORAGE_PROPERTY_QUERY,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

use crate::core::drive::{bus_name, type_of, DriveDetails, DriveType};

/// `FILE_READ_ONLY_VOLUME` in `GetVolumeInformationW`'s flags.
const FILE_READ_ONLY_VOLUME: u32 = 0x0008_0000;

/// What one volume answers. `label` is `None` when it has none (the caller says "Local Disk").
pub struct Probe {
    pub label: Option<String>,
    pub file_system: Option<String>,
    pub read_only: bool,
    pub total_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub drive_type: DriveType,
    pub bus: Option<u32>,
}

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(Some(0))
        .collect()
}

/// `E:\` → `E:`; anything else has no letter.
pub fn letter_of(root: &str) -> Option<String> {
    let b = root.as_bytes();
    (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':')
        .then(|| format!("{}:", (b[0] as char).to_ascii_uppercase()))
}

/// The storage bus of the disk behind a letter (`STORAGE_BUS_TYPE`): `\\.\E:` opened with no
/// access (it needs none, so no administrator), then `IOCTL_STORAGE_QUERY_PROPERTY`. `None` when
/// the drive will not say.
fn bus_of(letter: &str) -> Option<u32> {
    let device = wide(&format!("\\\\.\\{letter}"));
    // SAFETY: `device` is NUL-terminated and outlives the call; no security attributes, no template.
    let h = unsafe {
        CreateFileW(
            device.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return None;
    }
    // SAFETY: a zeroed query is a valid "standard query for the device descriptor".
    let mut query: STORAGE_PROPERTY_QUERY = unsafe { std::mem::zeroed() };
    query.PropertyId = StorageDeviceProperty;
    query.QueryType = PropertyStandardQuery;
    // u64s so the descriptor read below is aligned
    let mut out = [0u64; 128];
    let mut returned = 0u32;
    // SAFETY: both buffers are live locals of the lengths passed; `h` is ours until closed below.
    let ok = unsafe {
        DeviceIoControl(
            h,
            IOCTL_STORAGE_QUERY_PROPERTY,
            std::ptr::addr_of!(query).cast(),
            std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            out.as_mut_ptr().cast(),
            std::mem::size_of_val(&out) as u32,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    // SAFETY: closing the handle opened above, once.
    unsafe { windows_sys::Win32::Foundation::CloseHandle(h) };
    if ok == 0 || (returned as usize) < std::mem::size_of::<STORAGE_DEVICE_DESCRIPTOR>() {
        return None;
    }
    // SAFETY: the driver filled at least a descriptor's worth of bytes, and `out` is 8-aligned.
    let desc = unsafe { &*out.as_ptr().cast::<STORAGE_DEVICE_DESCRIPTOR>() };
    Some(desc.BusType as u32)
}

/// Label, file system, read-only flag, sizes and type of the volume at `root` (`E:\`). ⛔ Under
/// `QuietDriveErrors`: an empty card reader must not raise Windows' "no disk" box.
pub fn probe(root: &str) -> Probe {
    let _quiet = crate::core::shell::QuietDriveErrors::set();
    let root_w = wide(root);
    let mut name = [0u16; 261];
    let mut fs = [0u16; 261];
    let mut flags = 0u32;
    // SAFETY: every pointer is to a live local buffer of the stated length; the nullable
    // out-params we do not need are null, as the API allows.
    let ok = unsafe {
        GetVolumeInformationW(
            root_w.as_ptr(),
            name.as_mut_ptr(),
            name.len() as u32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut flags,
            fs.as_mut_ptr(),
            fs.len() as u32,
        )
    };
    let text = |buf: &[u16]| {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(0);
        Some(String::from_utf16_lossy(&buf[..end])).filter(|s| !s.is_empty())
    };
    let (label, file_system, read_only) = if ok != 0 {
        (text(&name), text(&fs), flags & FILE_READ_ONLY_VOLUME != 0)
    } else {
        (None, None, false)
    };
    let (mut free, mut total) = (0u64, 0u64);
    // SAFETY: out-params point to live u64s; the caller-free-bytes param may be null.
    let sized = unsafe {
        GetDiskFreeSpaceExW(root_w.as_ptr(), std::ptr::null_mut(), &mut total, &mut free)
    };
    let (total_bytes, free_bytes) = if sized == 0 {
        (None, None)
    } else {
        (Some(total), Some(free))
    };
    // SAFETY: a NUL-terminated root, live for the call.
    let raw_type = unsafe { GetDriveTypeW(root_w.as_ptr()) };
    // only a FIXED drive can be a USB disk in disguise; asking the others costs a device open
    let bus = if raw_type == 3 {
        letter_of(root).and_then(|l| bus_of(&l))
    } else {
        None
    };
    Probe {
        label,
        file_system,
        read_only,
        total_bytes,
        free_bytes,
        drive_type: type_of(raw_type, bus),
        bus,
    }
}

/// Everything the Properties pane shows for the drive at `root`.
pub fn volume_details(root: &str) -> DriveDetails {
    let p = probe(root);
    DriveDetails {
        path: root.to_string(),
        label: p.label.unwrap_or_else(|| "Local Disk".to_string()),
        file_system: p.file_system,
        drive_type: p.drive_type,
        total_bytes: p.total_bytes,
        free_bytes: p.free_bytes,
        letter: letter_of(root),
        mount_point: None,
        device: None,
        bus: p.bus.and_then(bus_name).map(str::to_string),
        read_only: p.read_only,
    }
}

/// Sets the volume's label. ⛔ Only ever called with a root the person chose from a drive row.
/// The `io::Error` is kept raw: access denied (5) is how the caller knows to elevate.
pub fn set_label(root: &str, label: &str) -> io::Result<()> {
    let root_w = wide(root);
    let label_w = wide(label);
    // SAFETY: both strings are NUL-terminated and outlive the call.
    if unsafe { SetVolumeLabelW(root_w.as_ptr(), label_w.as_ptr()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Is the letter still mounted.
fn mounted(letter: &str) -> bool {
    let Some(c) = letter.bytes().next() else {
        return false;
    };
    // SAFETY: no arguments; the bitmask is the return value.
    let mask = unsafe { GetLogicalDrives() };
    c.is_ascii_alphabetic() && mask & (1 << (c.to_ascii_uppercase() - b'A')) != 0
}

/// Asks Windows to eject the drive (the shell's own `eject` verb, so Windows shows its own
/// "safe to remove" toast or "problem ejecting" box), then waits up to 5 s for the letter to
/// go. `Ok(true)`: it is gone. `Ok(false)`: still there (Windows will have said why).
pub fn eject(root: &str) -> Result<bool, String> {
    let letter = letter_of(root).ok_or_else(|| "That is not a drive letter.".to_string())?;
    crate::core::shell_menu::invoke_verb(root, "eject", 0)?;
    for _ in 0..25 {
        if !mounted(&letter) {
            return Ok(true);
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    Ok(!mounted(&letter))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_letter_comes_from_a_drive_root() {
        assert_eq!(letter_of("e:\\"), Some("E:".into()));
        assert_eq!(letter_of("C:\\"), Some("C:".into()));
        assert_eq!(letter_of("\\\\server\\share"), None);
        assert_eq!(letter_of(""), None);
    }

    /// Read only: prints C: (and `VELKRYS_TEST_DRIVE` when set). Run by hand with
    /// `cargo test live_c_and_usb_details -- --ignored --nocapture`.
    #[test]
    #[ignore = "asks the real drives; prints what it sees"]
    fn live_c_and_usb_details() {
        let c = volume_details("C:\\");
        println!("C: {c:#?}");
        assert_eq!(c.letter.as_deref(), Some("C:"));
        assert!(c.total_bytes.is_some());
        if let Ok(root) = std::env::var("VELKRYS_TEST_DRIVE") {
            println!("{root} {:#?}", volume_details(&root));
        }
    }

    /// Renames `VELKRYS_TEST_DRIVE` and back. ⛔ Never C:.
    #[test]
    #[ignore = "renames the drive named by VELKRYS_TEST_DRIVE (a USB stick) and back"]
    fn live_set_label_round_trip() {
        let root =
            std::env::var("VELKRYS_TEST_DRIVE").expect("set VELKRYS_TEST_DRIVE to a USB stick");
        assert_ne!(
            letter_of(&root).as_deref(),
            Some("C:"),
            "never the system drive"
        );
        let before = volume_details(&root);
        assert!(
            matches!(
                before.drive_type,
                DriveType::Removable | DriveType::External
            ),
            "VELKRYS_TEST_DRIVE must be a removable drive, got {:?}",
            before.drive_type
        );
        set_label(&root, "VELKRYSTEST").expect("rename");
        assert_eq!(volume_details(&root).label, "VELKRYSTEST");
        set_label(&root, &before.label).expect("rename back");
        assert_eq!(volume_details(&root).label, before.label);
    }

    /// Ejects `VELKRYS_TEST_DRIVE`: the stick leaves the machine.
    #[test]
    #[ignore = "ejects the drive named by VELKRYS_TEST_DRIVE (a USB stick)"]
    fn live_eject_usb() {
        let root =
            std::env::var("VELKRYS_TEST_DRIVE").expect("set VELKRYS_TEST_DRIVE to a USB stick");
        assert_ne!(letter_of(&root).as_deref(), Some("C:"));
        let gone = eject(&root).expect("eject verb ran");
        println!("gone: {gone}");
    }
}
