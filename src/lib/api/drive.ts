// The only place `invoke` appears for drive properties, rename and eject. Mirrors
// src-tauri/src/commands/drive.rs. Errors are plain sentences, shown as notices.
import { invoke } from '@tauri-apps/api/core';
import type { DriveDetails, DriveInfo } from '$lib/types/shell';

/** The volume's own numbers and names. Never walks the drive. */
export const driveDetails = (path: string) => invoke<DriveDetails>('drive_details', { path });
/** Set the volume label. The returned row has the drive's new path (it can change on macOS). */
export const renameDrive = (path: string, label: string) =>
  invoke<DriveInfo>('rename_drive', { path, label });
/** Eject or power off a removable drive. `true` when it is safe to unplug. */
export const ejectDrive = (path: string) => invoke<boolean>('eject_drive', { path });
