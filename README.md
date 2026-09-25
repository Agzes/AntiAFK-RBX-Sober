<div align="center">
  <p>
    <a href="https://github.com/Agzes/AntiAFK-RBX-Sober/releases"><kbd>Latest release</kbd></a>
    &nbsp;·&nbsp;
    <a href="https://aur.archlinux.org/packages/antiafk-rbx-sober"><kbd>AUR package</kbd></a>
    &nbsp;·&nbsp;
    <a href="#build-from-source"><kbd>Build from source</kbd></a>
    &nbsp;·&nbsp;
    <a href="https://github.com/Agzes/AntiAFK-RBX-Sober/issues/new?template=bug_report.md"><kbd>Report a bug</kbd></a>
    &nbsp;·&nbsp;
    <a href="https://github.com/Agzes/AntiAFK-RBX-Sober/issues/new?template=feature_request.md"><kbd>Request a feature</kbd></a>
  </p>
  <h1><img height="24" alt="AntiAFK-RBX-Sober logo" src="https://raw.githubusercontent.com/Agzes/AntiAFK-RBX-Sober/refs/heads/main/assets/logo.png"/> AntiAFK-RBX-Sober <sup><kbd>v.0.2</kbd></sup> </h1>
  <p>
    A native Linux desktop utility for keeping Sober sessions active. <br>
    <strong>Compatibility:</strong> Hyprland and KDE Plasma 6.
  </p>
</div>

## > Features

- **Window management** - switches focus, performs an action, and restores the cursor position.
- **Configurable actions** - jump (`Space`), walk (`W`/`S`), or camera zoom (`I`/`O`).
- **Automatic lifecycle** - starts when Sober is detected and stops when Sober exits.
- **User-safe operation** - pauses actions while mouse movement is detected.
- **CPU limiting** - applies `systemd --user` `CPUQuota` to Sober scopes. _(beta)_
- **Automatic reconnect** - detects a disconnected state and selects the reconnect action using `grim` or `spectacle`. _(beta)_
- **Tray integration** - provides status and controls through the StatusNotifierItem tray (`ksni`).

## > Installation

### AppImage

Download the latest AppImage from the [Releases page](https://github.com/Agzes/AntiAFK-RBX-Sober/releases), make it executable, and run it:

```bash
chmod +x AntiAFK-RBX-Sober-vX.Y.Z.AppImage
./AntiAFK-RBX-Sober-vX.Y.Z.AppImage
```

### Arch Linux

The package is available from the AUR and can be installed with an AUR helper:

```bash
yay -S antiafk-rbx-sober
```

Alternatively:

```bash
paru -S antiafk-rbx-sober
```

## > First run

### Permissions

The application needs write access to `/dev/uinput`. The **Diagnostics → Fix** action creates a persistent udev rule (sudo required). For temporary access until reboot:

```bash
sudo chmod 666 /dev/uinput
```

Persistent rule:

```bash
echo 'KERNEL=="uinput", MODE="0666"' | sudo tee /etc/udev/rules.d/99-uinput-antiafk.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

### Desktop entry and launch

AUR installs the desktop entry automatically. For AppImage or source builds:

```bash
./AntiAFK-RBX-Sober --install
./AntiAFK-RBX-Sober
```

Use `--tray` instead of the last command to start with the window hidden. The install command also creates the desktop entry, icon, and local executable.

Required tools: `pgrep` and `systemd --user`; Hyprland also needs `hyprctl` and `grim`; KDE Plasma 6 Wayland needs `qdbus6`/`qdbus`, `journalctl`, and `spectacle`.

## > Build from source

Requires Rust, GTK4, `libdbus`, `pkg-config`, and a C/C++ toolchain.

- Debian / Ubuntu: `sudo apt install build-essential libgtk-4-dev libdbus-1-dev pkg-config`
- Arch Linux: `sudo pacman -S --needed base-devel gtk4 pkg-config`
- Fedora: `sudo dnf install gtk4-devel dbus-devel gcc pkg-config`

```bash
git clone https://github.com/Agzes/AntiAFK-RBX-Sober.git
cd AntiAFK-RBX-Sober
cargo build --release
./target/release/AntiAFK-RBX-Sober
```

## > Uninstall

Local installation:

```bash
rm -f ~/.local/share/applications/dev.agzes.antiafk-rbx-sober.desktop
rm -f ~/.local/share/icons/dev.agzes.antiafk-rbx-sober.png
rm -f ~/.local/bin/antiafk-rbx-sober
```

AUR: `yay -Rns antiafk-rbx-sober` or `paru -Rns antiafk-rbx-sober`.

## > Support and license

Supported: Hyprland and KDE Plasma 6 on Wayland. Other environments soon.

<kbd>With</kbd> <kbd>❤️</kbd> <kbd>by</kbd> <kbd>Agzes</kbd><br>
<kbd>pls ⭐ project!</kbd>
