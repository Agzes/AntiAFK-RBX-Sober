<div align="center">
  <p>
    <a href="https://github.com/Agzes/AntiAFK-RBX-Sober/releases"><kbd>Latest release</kbd></a>
    &nbsp;·&nbsp;
    <a href="https://aur.archlinux.org/packages/antiafk-rbx-sober"><kbd>AUR package</kbd></a>
    &nbsp;·&nbsp;
    <a href="#-build-from-source"><kbd>Build from source</kbd></a>
    &nbsp;·&nbsp;
    <a href="https://github.com/Agzes/AntiAFK-RBX-Sober/issues/new?template=bug_report.yml"><kbd>Report a bug</kbd></a>
    &nbsp;·&nbsp;
    <a href="https://github.com/Agzes/AntiAFK-RBX-Sober/issues/new?template=feature_request.yml"><kbd>Request a feature</kbd></a>
  </p>
  <h1><img height="24" alt="AntiAFK-RBX-Sober logo" src="https://raw.githubusercontent.com/Agzes/AntiAFK-RBX-Sober/refs/heads/main/assets/logo.png"/> AntiAFK-RBX-Sober <sup><kbd>v.1.0</kbd></sup> </h1>
  <p>
    A native Linux desktop utility for keeping Sober sessions active. <br>
    <strong>Compatibility:</strong> <a href="docs/hyprland.md">Hyprland</a>, <a href="docs/kde.md">KDE Plasma 6</a>, <a href="docs/gnome.md">GNOME</a>, <a href="docs/niri.md">Niri</a>, <a href="docs/cosmic.md">COSMIC</a>, <a href="docs/x11.md">X11</a>, and <a href="docs/i3.md">i3</a>.
  </p>
</div>

## > Features

- **Window management** - switches focus, performs an action, and restores the cursor position.
- **Configurable actions** - jump (`Space`), walk (`W`/`S`), or camera zoom (`I`/`O`).
- **Automatic lifecycle** - starts when Sober is detected and stops when Sober exits.
- **User-safe operation** - pauses actions while mouse movement or keyboard input is detected.
- **Hide Game** - moves Sober out of the way while automation runs, and restores it when the app stops.
- **CPU limiting** - applies `systemd --user` `CPUQuota` to Sober scopes. _(beta)_
- **Automatic reconnect** - finds Sober's reconnect prompt in a screenshot and presses it, reporting progress in the status line. _(beta)_
- **Tray integration** - provides status and controls through the StatusNotifierItem tray (`ksni`).

## > Supported desktops

Detailed setup and troubleshooting guides are available in the [**docs/**](docs/README.md) directory:

| Desktop                          | Session Type | Input Method                | Hide Game        | Auto Reconnect        | FPS Capper |
| :------------------------------- | :----------- | :-------------------------- | :--------------- | :-------------------- | :--------- |
| [**Hyprland**](docs/hyprland.md) | Wayland      | `uinput`                    | yes              | yes (`grim`)          | yes        |
| [**KDE Plasma 6**](docs/kde.md)  | Wayland      | `uinput`                    | yes              | yes (`spectacle`)     | yes        |
| [**GNOME Shell**](docs/gnome.md) | Wayland      | `uinput` + Shell Extension  | yes              | yes (Extension)       | yes        |
| [**Niri**](docs/niri.md)         | Wayland      | `uinput` + `wtype`          | yes              | yes (`grim`)          | yes        |
| [**COSMIC**](docs/cosmic.md)     | Wayland      | `uinput` + absolute pointer | yes              | yes (`grim`)          | **no\***   |
| [**X11 (Generic)**](docs/x11.md) | X11          | `uinput` / `xdotool`        | yes              | yes (`maim`/`import`) | yes        |
| [**i3**](docs/i3.md)             | X11          | `uinput` / `xdotool`        | yes (scratchpad) | yes (`maim`/`import`) | yes        |

When a feature is unsupported or a required tool is missing in your environment, its toggle is safely disabled with an explanatory tooltip, and full status is reported in the **Diagnostics** tab.

\* - The FPS Capper is intentionally unavailable on COSMIC: the `CPUQuota` scope limit can terminate Sober there, so the switch is disabled and preflight refuses to enable it.

### GNOME helper extension

Modern GNOME (Wayland) restricts unprivileged processes from querying window positions, moving the pointer, or capturing window screenshots without interactive portal prompts. AntiAFK includes a bundled, versioned GNOME Shell Extension exporting a dedicated D-Bus helper interface (`antiafk-rbx-sober@agzes.github.io`).

**Installation methods:**

1. **Via UI (Recommended)**: Launch the app, switch to the **Diagnostics** tab, and click **Install & Enable** on the GNOME Extension row.
2. **Via CLI**:
    ```bash
    AntiAFK-RBX-Sober --install-gnome-helper
    ```
3. **Manual**: Copy the extension files into `~/.local/share/gnome-shell/extensions/`, see [assets/gnome-extension/README.md](assets/gnome-extension/README.md) for step-by-step instructions.

> **Note**: On Wayland, log out and back in after the initial installation so GNOME Shell can load the new extension. If needed, ensure **AntiAFK-RBX-Sober GNOME backend** is enabled in the Extensions app.

### Desktop override

On tty or nested sessions the desktop cannot always be detected automatically. Pick the backend explicitly:

```bash
AntiAFK-RBX-Sober --force-desktop cosmic
```

The same works through `ANTIAFK_FORCE_DESKTOP=cosmic`. Accepted values are `auto`, `hyprland`, `kde`, `niri`, `cosmic`, `gnome`, `x11` and `i3`.

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

The application needs write access to `/dev/uinput`. The **Diagnostics -> Fix** action creates a persistent udev rule (sudo required). For temporary access until reboot:

```bash
sudo chmod 666 /dev/uinput
```

Persistent rule:

```bash
echo 'KERNEL=="uinput", MODE="0666"' | sudo tee /etc/udev/rules.d/99-uinput-antiafk.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

### Desktop entry and launch

AUR installs the desktop entry automatically. For standalone binaries or source builds:

- **Via UI**: A banner will appear on first launch offering **Add to System**. Alternatively, go to **Diagnostics -> Cleanup & Integration -> Reinstall / Add to System**.
- **Via CLI**:

```bash
./AntiAFK-RBX-Sober --install
./AntiAFK-RBX-Sober
```

This installs:

- `.desktop` file: `~/.local/share/applications/dev.agzes.antiafk-rbx-sober.desktop`
- Application icons: `~/.local/share/icons/hicolor/128x128/apps/` and `~/.local/share/pixmaps/`
- Binary: `~/.local/bin/antiafk-rbx-sober`

When you launch a newly downloaded binary while an older version is already installed in `~/.local/bin`, the application prompts you with a modal dialog to update the installed copy.

Use `--tray` to start with the window hidden.

Required tools: `pgrep` and `systemd --user`. Depending on your desktop: Hyprland (`hyprctl`, `grim`), KDE (`qdbus`, `journalctl`, `spectacle`), GNOME (AntiAFK Shell Extension), Niri (`wtype`, `grim`), X11/i3 (`xdotool`, `i3-msg`, `maim`/`scrot`). See [docs/](docs/README.md) for complete details.

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

## > Updates

- **AUR**: If installed via AUR, AntiAFK detects package management and suggests updating with your AUR helper (`yay -S antiafk-rbx-sober` or `paru -S antiafk-rbx-sober`).
- **Manual / Standalone**: When launching a newer binary, the app offers to replace the outdated version installed in `~/.local/bin`.

## > Uninstall & Data Cleanup

### Removing Desktop Integration

- **Via UI**: Open **Diagnostics -> Cleanup & Integration -> Remove**.
- **Via CLI**:

```bash
./AntiAFK-RBX-Sober --uninstall
```

### Full Purge (Configs & Traces)

To completely remove all application configurations, desktop integration files, and GNOME extensions:

- **Via UI**: Open **Diagnostics -> Cleanup & Integration -> Purge & Exit**.
- **Via CLI**:

```bash
./AntiAFK-RBX-Sober --purge
```

For AUR installations, remove the package with your package manager after purging: `yay -R antiafk-rbx-sober` (or `paru -R antiafk-rbx-sober`).

## > Support and license

Supported: Hyprland, KDE Plasma 6, GNOME Shell, Niri, COSMIC, X11, and i3.

<kbd>With</kbd> <kbd>❤️</kbd> <kbd>by</kbd> <kbd>Agzes</kbd><br>
<kbd>pls ⭐ project!</kbd>
