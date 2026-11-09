// Mirrors src-tauri/src/core/shell.rs (serde camelCase). Change both or neither.

export type KnownFolderKey =
  'desktop' | 'downloads' | 'documents' | 'pictures' | 'music' | 'videos';

export interface KnownFolder {
  key: KnownFolderKey;
  label: string;
  path: string;
}

export interface DriveInfo {
  /** `C:\` on Windows; `/` or `/Volumes/<name>` on macOS; `/`, `/media/…` or `/mnt/<name>` on Linux */
  path: string;
  label: string;
  /** null when the OS would not say: unknown is not zero */
  totalBytes: number | null;
  freeBytes: number | null;
  /**
   * A network share (SMB, AFP, NFS, WebDAV, sshfs ...). Its sizes are always null, and nothing
   * walks it on its own - a search from This PC or the palette's every-drive pass leaves it out.
   * Touching one is what makes macOS ask "Velkrys would like to access files on a network volume"
   * (`core/mounts.rs`). Missing or false on Windows.
   */
  network?: boolean;
  /**
   * What kind of drive it is (plan 23). ⚠️ On Windows `network` stays false for a mapped letter
   * while this says "network": key on `driveType`, not on `network`.
   */
  driveType?: DriveType;
  /** `NTFS`, `exFAT`, `ext4` ...; null when the OS would not say. */
  fileSystem?: string | null;
  /** Offer a Rename row. */
  canRename?: boolean;
  /** Offer an Eject row. */
  ejectable?: boolean;
}

export type DriveType =
  'fixed' | 'external' | 'removable' | 'optical' | 'network' | 'ram' | 'unknown';

/** One drive's own numbers and names for the Details pane (`drive_details`, plan 23). Every value
 * the OS did not give is `null`: unknown is not zero. Mirrors `DriveDetails` in `core/drive.rs`. */
export interface DriveDetails {
  path: string;
  label: string;
  fileSystem: string | null;
  driveType: DriveType;
  totalBytes: number | null;
  freeBytes: number | null;
  /** `E:`; Windows only */
  letter: string | null;
  /** `/Volumes/Backup`; macOS and Linux */
  mountPoint: string | null;
  /** `/dev/disk4s1`; macOS and Linux */
  device: string | null;
  /** how it is plugged in, `USB` */
  bus: string | null;
  readOnly: boolean;
}

/** The room left on the volume that really holds a folder, and where that volume is mounted
 * (`folder_space`, src-tauri `core/volume.rs`). Mirrors `FolderSpace` there. */
export interface FolderSpace {
  freeBytes: number;
  mountRoot: string;
}
