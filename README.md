<p align="center">
  <a href="https://github.com/Agzes/AntiAFK-RBX"><img src="readme/antiafk-rbx-v3.png" alt="Go To AntiAFK-RBX for windows" style="width:33%;"></a>
  <img src="readme/-.png" alt="" style="width:32%;">
  <a href="#installation"><img src="readme/download-now.png" alt="Download Now" style="width:33%;"></a>
</p>

![AntiAFK-RBX-Sober by Agzes v.3.1](readme/antiafk-rbx.png)

<p align="center" width="100%">
  <a href="https://github.com/Agzes/AntiAFK-RBX-Sober/issues/new?template=feature_request.md"><img src="readme/got-ideas-to-add.png" alt="Got ideas to add?" style="width:32%; min-width:190px;"></a>
  <a href="https://boosty.to/agzes/donate"><img src="readme/want-to-support.png" alt="Want to support?" style="width:32%; min-width:190px;"></a>
  <a href="https://github.com/Agzes/AntiAFK-RBX-Sober/issues/new?template=bug_report.md"><img src="readme/bug-write-issue.png" alt="Bug? Write issue!" style="width:32%; min-width:190px;"></a>
</p>

---

Anti-AFK application specifically designed for **Sober** (Roblox on Linux). Currently focused on **Hyprland** environment (using "Swapper" mode).

## ✨ Features

- **🧠 Smart window management**: The program switches focus, performs an action, and returns the cursor to its original position.
- **⏰ AFK Actions**: Jump (Space), Walk (W/S), or Camera Zoom (I/O).
- **⏯️ Auto-Start/Stop**: Automatically enables when Sober is detected and disables once the game is closed.
- **🛡️ User-Safe**: Pauses actions if user activity (mouse movement or key presses) is detected.
- **🎯 FPS Capper**: Limits CPU resource consumption of background game processes via `systemd CPUQuota`. [BETA]
- **🔄 Auto Reconnect**: Automatically clicks the "Reconnect" button on disconnect (uses `grim` for pixel analysis). [BETA]
- **🥷 Stealth Mode**: Ability to run the game in "hidden" mode (Special Workspace in Hyprland) during action execution.
- **📥 Tray**: Convenient control and status indication via the tray (powered by `ksni`).

## 📥 Installation

<a id="installation"></a>

### AppImage (binary)
- **AppImage**: Download the latest version from the [Releases](https://github.com/Agzes/AntiAFK-RBX-Sober/releases) page. (run `chmod +x AntiAFK-RBX-Sober-vX.Y.Z.AppImage` and then `./AntiAFK-RBX-Sober-vX.Y.Z.AppImage` and see [Usage](#usage))

### AUR (auto-build)
- **[AUR](https://aur.archlinux.org/packages/antiafk-rbx-sober)**: 
```bash
yay -S antiafk-rbx-sober
``` 
or 
```bash
paru -S antiafk-rbx-sober 
```

<a id="usage"></a>

## 🚀 After install & usage

### 1. uinput Permissions
The program requires write access to `/dev/uinput` to simulate input. The application includes a **"FIX"** button in the Compatibility section that sets this up automatically (requires sudo password).

But you can unlock it manually using (resets after reboot):
```bash
sudo chmod 666 /dev/uinput
```
Or create a udev rule (recommended, will be applied after reboot):
```bash
echo 'KERNEL=="uinput", MODE="0666"' | sudo tee /etc/udev/rules.d/99-uinput-antiafk.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

### 2. Register Application (Desktop)
*don't need if you install via AUR*

To add the application to your system menu, run the binary with the `--install` flag:
```bash
./AntiAFK-RBX-Sober --install
```
This will automatically:
- Create a `.desktop` entry in `~/.local/share/applications/`.
- Install the application icon to `~/.local/share/icons/`.

### 3. Runtime Dependencies
- **General**: `pgrep`, `systemd` (for FPS Capper).
- **Hyprland**: `grim` (for "Auto Reconnect").
- **KDE Plasma**: `spectacle` (for "Auto Reconnect") and `qdbus`/`qdbus6` (for focus management, basically pre-installed on KDE).

## 📦 Build

### Prerequisites
You will need **Rust**, **GTK4**, and **libdbus** development libraries installed on your system.

### Building from Source
1. Install [Rust](https://www.rust-lang.org/tools/install).

2. Install build-time dependencies:
   - **Ubuntu/Debian**: `sudo apt install build-essential libgtk-4-dev libdbus-1-dev pkg-config`
   - **Arch Linux**: `sudo pacman -S --needed base-devel gtk4 pkg-config`
   - **Fedora**: `sudo dnf install gtk4-devel dbus-devel gcc pkg-config`

3. Clone the repository:
   ```bash
   git clone https://github.com/Agzes/AntiAFK-RBX-Sober.git
   cd AntiAFK-RBX-Sober
   ```

4. Build the project:
   ```bash
   cargo build --release
   ```

5. Run the binary:
   ```bash
   ./target/release/AntiAFK-RBX-Sober
   ```



The compiled binary will be located at `target/release/AntiAFK-RBX-Sober`.

## 🗑️ Correct uninstall 
### AppImage:
1. Remove `~/.local/share/applications/dev.agzes.antiafk-rbx-sober.desktop` 
2. Remove the AppImage file.

### AUR:
1. Run `yay -Rns antiafk-rbx-sober` or `paru -Rns antiafk-rbx-sober`

## ⚠️ Important Note
Currently program support only Hyprland and KDE Plasma 6 (with AntiAFK-RBX-Sober v.0.2.0). Support for other environments (GNOME, NIRI, X11) is planning.

---

<kbd>With</kbd> <kbd>❤️</kbd> <kbd>by</kbd> <kbd>Agzes</kbd><br>
<kbd>pls ⭐ project!</kbd>
