# Install Velkrys on a Mac

Velkrys for Mac is free. It is signed with a Developer ID and notarized by Apple (Zettabyte Technology Inc.), so it opens like any other app. There is nothing to allow in System Settings first.

## Install with Homebrew (easiest)

If you use [Homebrew](https://brew.sh), one command installs Velkrys, and `brew upgrade` keeps it up to date:

```sh
brew install --cask joyahmed/tap/velkrys
```

Then open **Velkrys** from Applications.

## Install from the download

1. Download `Velkrys_<version>_universal.dmg` from the releases page. It runs on Apple silicon and Intel Macs.
2. Open the downloaded file. A window shows the Velkrys icon and an Applications folder.
3. Drag **Velkrys** onto **Applications**. If you already have an older Velkrys there, quit Velkrys first, then choose **Replace**.
4. Open **Velkrys** from Applications. macOS may ask once whether you want to open an app downloaded from the internet. Click **Open**.

## What Velkrys asks for

macOS may show these questions as you use Velkrys. Click **Allow** or **OK** to let Velkrys work fully:

- **Documents, Desktop and Downloads folders:** so Velkrys can show the files in them.
- **Network drives and USB sticks or external drives:** so Velkrys can show the files on them.
- **Control Finder (Automation):** so Move to Trash and Put Back work the same as in Finder. It is asked the first time you delete a file.
- **Full Disk Access:** needed only to show the Trash. If Velkrys's Trash view says it needs access, open System Settings, then Privacy & Security, then Full Disk Access, and switch Velkrys on. Without it everything else still works.

After you install a new version, macOS may ask these questions again.

You can change any of these later in System Settings, under Privacy & Security.

## Uninstall

1. Quit Velkrys.
2. Drag **Velkrys** from Applications to the Trash.
3. Optional: to remove its saved settings and search index too, delete the folder `~/Library/Application Support/com.joyahmed.velkrys` (in Finder, choose Go, then Go to Folder, and paste that path).
