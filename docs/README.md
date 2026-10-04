# AntiAFK-RBX-Sober documentation

AntiAFK-RBX-Sober is a native Linux utility designed to keep Sober (Roblox for Linux) sessions active across modern Wayland compositors and X11 window managers.

---

## Supported Desktop Environments

Detailed compositor-specific setup, requirements, and troubleshooting guides:

| Desktop                     | Session Type | Input Method                | Hide Game                                        | Auto Reconnect              | FPS Capper |
| :-------------------------- | :----------- | :-------------------------- | :----------------------------------------------- | :-------------------------- | :--------- |
| [**Hyprland**](hyprland.md) | Wayland      | `uinput`                    | Special workspace (`special:minimized`)          | `grim`                      | Yes        |
| [**KDE Plasma 6**](kde.md)  | Wayland      | `uinput` + KWin Scripting   | Window minimize                                  | `spectacle`                 | Yes        |
| [**GNOME Shell**](gnome.md) | Wayland      | `uinput` + Shell Extension  | Shell minimize                                   | Shell Extension             | Yes        |
| [**Niri**](niri.md)         | Wayland      | `uinput` + `wtype`          | Dedicated workspace (`antiafk-rbx-sober-hidden`) | `grim`                      | Yes        |
| [**COSMIC**](cosmic.md)     | Wayland      | `uinput` + absolute pointer | Toplevel minimize                                | `grim`                      | **No\***   |
| [**X11 (Generic)**](x11.md) | X11          | `uinput` / `xdotool`        | Window minimize                                  | `maim` / `import` / `scrot` | Yes        |
| [**i3**](i3.md)             | X11          | `uinput` / `xdotool`        | Scratchpad                                       | `maim` / `import` / `scrot` | Yes        |

\* _The FPS Capper is disabled on COSMIC because cgroup `CPUQuota` process pausing causes `cosmic-comp` to terminate Sober._

---

## Feature Overview & Architecture

### 1. Anti-AFK Actions & Intervals

AntiAFK simulates player activity to prevent Roblox's 20-minute idle disconnect:

- **Jump (`Space`)**: sends a 30 ms key tap.
- **Walk (`W`/`S`)**: presses `W` for 150 ms, pauses 50 ms, then presses `S` for 150 ms to return near the starting position.
- **Camera Zoom / Spin (`I`/`O`)**: sends zoom-in (`I`) and zoom-out (`O`) for 30 ms.
- **Interval Presets & Custom Interval**: choose from presets (4m, 6m, 9m, 19m) or input any custom duration in minutes.
- **Input Delivery**:
    - Wayland compositors: delivered via virtual kernel devices (`/dev/uinput`).
    - Niri: pointer clicks via `/dev/uinput`, keystrokes via `wtype` (bypassing compositor virtual keyboard restrictions).
    - X11 / i3: kernel `/dev/uinput` with automatic, zero-permission fallback to `xdotool` (XTest).

### 2. Multi-Instance Support

When **Multi-Instance** is enabled, AntiAFK scans for all active Sober windows or processes and cycles through each one during every action tick:

- Targets all Sober clients sequentially with a brief focus switch (150 ms).
- Warps cursor, performs configured actions, checks reconnect, and hides each instance.
- When disabled, AntiAFK restricts automation strictly to the primary Sober instance.

### 3. User-Safe ("Don't Interrupt Me")

AntiAFK protects your active work, typing, and manual gameplay:

- **Dual-Layer Activity Detection**:
    1. A low-overhead background thread listens to physical input devices via `/dev/input/event*` (`evdev`). Any physical key press or mouse motion immediately records user activity.
    2. Compositor-level cursor tracking measures cursor displacement before activating windows.
- When activity is detected, AntiAFK sets status to **Paused** and delays the anti-AFK tick for 3-5 seconds until input ceases.

### 4. Hide Game (Stealth Mode)

Keeps Sober off your screen while background automation runs:

- **Hyprland**: moves the client silently to `special:minimized`.
- **KDE Plasma 6**: minimizes windows using KWin scripting (`client.minimized = true`).
- **GNOME Shell**: minimizes windows via the extension D-Bus method `SetMinimized(id, true)`.
- **Niri**: stashes the window into a dedicated workspace `workspace "antiafk-rbx-sober-hidden"`.
- **COSMIC**: minimizes via `zcosmic_toplevel_handle_v1.set_minimized()`.
- **X11**: minimizes via `xdotool windowminimize`.
- **i3**: stashes windows into i3's scratchpad (`i3-msg [id=...] move scratchpad`).
- **Safe Restoration**: when the application is stopped, unhidden, or encounters an error, all hidden windows are immediately restored to the active workspace.

### 5. Automatic Reconnect

Detects Roblox's disconnection modal and reconnects automatically:

1. **Window Capture**: takes a clean, unprompted screenshot of the Sober window (via `grim`, `spectacle`, the GNOME extension, or `maim`/`import`).
2. **Modal Recognition**: analyzes pixel buffer around expected dialog coordinates matching Roblox's dark theme modal (`RGB(57, 59, 61)` within tolerance and bright button text).
3. **Button Interaction**: calculates exact Reconnect button coordinates, moves the pointer, and performs a multi-click sequence.
4. **Status Feedback**: reports `ReconnectChecking` and `Reconnecting` in the UI.

### 6. FPS Capper (CPU Quota Throttling)

Lowers CPU usage and power consumption:

- **Systemd Cgroups**: applies `systemctl --user set-property <scope> CPUQuota=X%` to Sober's systemd user scopes.
- **Process Duty-Cycle Fallback**: if Sober does not run in an isolated systemd scope, AntiAFK employs a precision `SIGSTOP`/`SIGCONT` duty cycle to enforce the target limit.
- **Unlock on Focus**: when **Unlock limit when game is focused** is enabled, AntiAFK tracks the active window and removes the throttle whenever you switch back to Sober.

### 7. Process Lifecycle & Auto-Start

- **Low-Overhead Process Monitoring**: AntiAFK scans `/proc` once per second without external process spawning overhead to identify Sober PIDs.
- **Auto-Start**: when enabled, AntiAFK automatically starts automation within 1 second of Sober opening, and enters a clean waiting state when Sober closes.

### 8. System Tray Integration (`ksni`)

- StatusNotifierItem tray icon displays current status (Stopped, Running).
- Tray menu allows hiding/showing Sober, opening settings, or quitting.
- Start directly into the system tray using `--tray`.

### 9. Theme & Accent Color Synchronization

- **System Accent Sync**: automatically reads system accent colors from GTK 4 / GTK 3 stylesheets (`~/.config/gtk-4.0/gtk.css`).
- **Dark / Light Modes**: supports manual dark/light switching or automatic system-level color synchronization.

### 10. Desktop Integration, Updates & Lifecycle Management

- **Desktop Entry & Icon**: Install `.desktop` launchers, hicolor application icons, and binary to `~/.local/bin` directly via the UI banner, Diagnostics page, or `--install` flag.
- **Smart Update Detection**:
    - Automatically identifies whether the running binary is packaged via AUR (`pacman -Qo`).
    - Prompts to update via AUR helper (`yay -S antiafk-rbx-sober` or `paru`) when a new version is released.
    - Detects standalone newer versions launched alongside an older installed binary and prompts to update the installed copy in `~/.local/bin`.
- **Clean Uninstallation & Purge**:
    - **Uninstall**: Cleanly removes `.desktop` files, icons, and binary from `~/.local/`, refreshing desktop databases.
    - **Purge All Traces**: Completely erases `~/.config/antiafk-rbx-sober`, desktop integration files, and GNOME Shell extensions, then safely exits.

---

## CLI Reference & Diagnostics

```bash
AntiAFK-RBX-Sober [OPTIONS]
```

| Argument                    | Description                                                                           |
| :-------------------------- | :------------------------------------------------------------------------------------ |
| `-t`, `--tray`              | Start application minimized directly to the system tray.                              |
| `--install`                 | Install desktop entry, application icon, and binary to `~/.local/`.                   |
| `--uninstall`               | Remove desktop entry, application icons, and binary from `~/.local/`.                 |
| `--purge`                   | Delete all config files, desktop integration, and GNOME extension, then exit.         |
| `--install-gnome-helper`    | Install and enable the bundled GNOME Shell Extension.                                 |
| `--force-desktop <DESKTOP>` | Force a specific backend (`hyprland`, `kde`, `gnome`, `niri`, `cosmic`, `x11`, `i3`). |
| `--diagnose-cosmic`         | Run low-level Wayland protocol diagnostics for COSMIC.                                |
| `-h`, `--help`              | Print help information.                                                               |
| `-V`, `--version`           | Print version information.                                                            |

### Environment Variables

- `ANTIAFK_FORCE_DESKTOP=<name>`: overrides automatic desktop detection.
- `ANTIAFK_ACCENT_COLOR=<hex>`: overrides UI accent color (e.g. `#7aa2f7`).
