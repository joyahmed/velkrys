# Install Velkrys on Linux

Velkrys for Linux is free. It comes in three forms: a `.deb` package for Ubuntu and Debian, an `.rpm` package for Fedora and openSUSE, and an AppImage that runs on most other desktops. Pick one.

## Install the .deb (Ubuntu, Debian)

1. Download `Velkrys_<version>_amd64.deb` from the releases page.
2. Open a terminal in the folder where it was saved and run:

   ```
   sudo apt install ./Velkrys_<version>_amd64.deb
   ```

   Keep the `./` in front of the name. It tells apt to use the file you downloaded. apt also fetches the few libraries Velkrys needs (the GTK and WebKit libraries, and the tray icon library).

3. Open Velkrys from your applications menu.

## Install the .rpm (Fedora, openSUSE)

1. Download `Velkrys-<version>-1.x86_64.rpm` from the releases page.
2. Open a terminal in the folder where it was saved and run:

   ```
   sudo dnf install ./Velkrys-<version>-1.x86_64.rpm
   ```

   On openSUSE use `sudo zypper install ./Velkrys-<version>-1.x86_64.rpm`. The package manager also fetches the libraries Velkrys needs.

3. Open Velkrys from your applications menu.

## Use the AppImage (any distribution)

1. Download `Velkrys_<version>_amd64.AppImage` from the releases page.
2. Make it runnable, then start it:

   ```
   chmod +x Velkrys_<version>_amd64.AppImage
   ./Velkrys_<version>_amd64.AppImage
   ```

   You can also right-click the file, open **Properties**, and tick "Allow executing file as program", then double-click it.

3. If it will not start and the message mentions FUSE or `libfuse.so.2`, install the FUSE 2 library (on Ubuntu 22.04 and Debian: `sudo apt install libfuse2`; on Ubuntu 24.04 and newer: `sudo apt install libfuse2t64`). Or skip the library and run it unpacked:

   ```
   APPIMAGE_EXTRACT_AND_RUN=1 ./Velkrys_<version>_amd64.AppImage
   ```

The AppImage does not add Velkrys to your applications menu. Keep the file somewhere you can find it, such as `~/Applications`.

## Updates

Velkrys does not update itself on Linux yet. To update, download the new `.deb`, `.rpm` or AppImage and install it the same way as before. A new `.deb` or `.rpm` replaces the old one and keeps your settings.

## Open Velkrys from anywhere

Velkrys has a Spotlight search window. On Linux it opens with **Ctrl+Alt+Space**. This is not the same as on Windows and Mac (Alt+Space), because GNOME already uses Alt+Space for the window menu and would swallow it.

If another program already holds Ctrl+Alt+Space, Velkrys tells you once when it starts. You can pick a different shortcut in Settings.

## Use Velkrys as your default file manager (optional)

In Settings, turn on **Use as default file manager**. Velkrys then opens when another program asks to show a folder. Turn it off again and your previous file manager comes back. Velkrys saves the old choice first and puts exactly that back.

## Where Velkrys keeps its things

- Settings and your open windows: `~/.config/com.joyahmed.velkrys`
- Search index, icon cache and logs: `~/.local/share/com.joyahmed.velkrys`

## Uninstall

- **.deb:** `sudo apt remove velkrys`
- **.rpm:** `sudo dnf remove velkrys`
- **AppImage:** delete the file.
- Optional: to remove saved settings and the cache too, delete the two folders above.
- If you turned on "default file manager", turn it off in Settings first.
