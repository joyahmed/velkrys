# Install Velkrys on Windows

Velkrys for Windows is free. Pick one way. Windows 10 and 11, 64-bit.

## Microsoft Store (easiest)

Open [Velkrys in the Microsoft Store](https://apps.microsoft.com/detail/9PNW6SR97XWL) and click **Get**.
The Store version is signed by Microsoft, so Windows opens it with no warning, and the Store keeps it up to date.

Or from a terminal:

```powershell
winget install 9PNW6SR97XWL
```

## Scoop

If you use [Scoop](https://scoop.sh):

```powershell
scoop bucket add zettabyte https://github.com/joyahmed/scoop-bucket
scoop install zettabyte/velkrys
```

`scoop update velkrys` gets the next version.

## Download the installer

1. Download `Velkrys_<version>_x64-setup.exe` from the [releases page](https://github.com/joyahmed/velkrys/releases/latest).
2. Run it. This download isn't signed by a known publisher yet, so Windows may warn you: click **More info**, then **Run anyway**.
3. To update later, install the new version over the old one.

Each release has a `SHA256SUMS` file. To check your download: `Get-FileHash .\Velkrys_<version>_x64-setup.exe` and compare the hash.

## Coming from Trove

Velkrys used to be called Trove. Install Velkrys next to Trove: on first start it copies your settings, saved tabs and undo history. Your Trove folder is not touched.

## Uninstall

Settings, then Apps, then Installed apps, find **Velkrys**, then **Uninstall**.
