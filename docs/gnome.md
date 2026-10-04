# GNOME Shell Backend Guide

AntiAFK-RBX-Sober integrates with [GNOME Shell (Wayland & X11)](https://www.gnome.org) using a dedicated, lightweight GNOME Shell Extension (`antiafk-rbx-sober@agzes.github.io`) and `/dev/uinput`.

---

## Why a GNOME Extension?

On GNOME (Wayland), Mutter blocks unprivileged external processes from:

- Querying window lists and geometries across workspaces.
- Warping or setting pointer coordinates directly.
- Taking arbitrary background screenshots via `org.gnome.Shell.Screenshot` without interactive user prompts.
- Accessing `org.gnome.Shell.Eval`.

To provide seamless, background automation without portal popups or Mutter compositor crashes, AntiAFK includes a bundled, versioned GNOME Shell Extension exporting a dedicated D-Bus helper interface: `org.gnome.Shell.Extensions.AntiAFK` (bus name: `dev.agzes.antiafk`).

---

## Requirements

| Tool                | Purpose                                                 | Package                                       |
| :------------------ | :------------------------------------------------------ | :-------------------------------------------- |
| **GNOME Extension** | Window tracking, focusing, pointer warping, screenshots | Bundled (`antiafk-rbx-sober@agzes.github.io`) |
| `/dev/uinput`       | Virtual input emulation                                 | Built-in kernel module                        |

---

## Installation & Quick Setup

### Method 1: In the App (Recommended)

1. Launch **AntiAFK-RBX-Sober**.
2. Open the **Diagnostics** tab.
3. Click **Install & Enable** on the **GNOME Extension** row.
4. If this is your first install, log out and back into GNOME.

### Method 2: Via CLI

```bash
AntiAFK-RBX-Sober --install-gnome-helper
```

### Method 3: Manual Installation

```bash
UUID="antiafk-rbx-sober@agzes.github.io"
DEST="$HOME/.local/share/gnome-shell/extensions/$UUID"
mkdir -p "$DEST"
cp assets/gnome-extension/metadata.json assets/gnome-extension/extension.js "$DEST/"
gnome-extensions enable "$UUID"
```

Check status at any time with:

```bash
gnome-extensions info antiafk-rbx-sober@agzes.github.io
```

> **Important**: On GNOME (Wayland), **log out and log back in** after the initial installation so GNOME Shell can discover and initialize the new extension.

---

## Architecture & How It Works

### 1. D-Bus Extension Interface

The extension exports the `dev.agzes.antiafk.Backend` interface at `/dev/agzes/antiafk` with the following methods:

- `ListWindows()`: queries Mutter's `global.get_window_actors()`, returning JSON metadata for all windows (id, class, title, geometry, active state).
- `FocusWindow(id)`: activates the target window using Mutter's `activate(global.get_current_time())`.
- `GetCursorPosition()`: returns the exact global pointer coordinates `(x, y)`.
- `WarpPointer(x, y)`: sets the pointer position via Clutter's pointer actor.
- `SetMinimized(id, bool)`: minimizes or unminimizes windows.
- `CaptureWindowToFile(id, filepath)`: captures window pixels using native `Shell.Screenshot.screenshot_area` into a `Gio.FileStream`, bypassing deprecated external screenshot APIs.

### 2. Multi-Instance Support

- AntiAFK filters all windows returned by `ListWindows()` matching Sober metadata (`class` or `title` containing `sober` / `roblox`).
- When **Multi-Instance** is enabled, it cycles through all matching windows sequentially.

### 3. Cursor Preservation & User Safety

- Before switching windows, AntiAFK captures current pointer coordinates via `GetCursorPosition()`.
- After sending actions, the pointer is restored via `WarpPointer()`.
- **User Safety ("Don't Interrupt Me")**: Dual-check combining kernel `evdev` listening on `/dev/input/event*` with cursor delta tracking from the extension. Automation pauses for 3 seconds whenever input is detected.

### 4. Hide Game (Stealth Mode)

- Invokes `SetMinimized(id, true)` on Sober windows.
- Upon stopping or unhiding, restores windows via `SetMinimized(id, false)`.

### 5. Auto Reconnect

- Calls `CaptureWindowToFile(id, path)` to capture the window safely.
- Analyzes pixels for the dark-theme reconnect modal (`RGB(57, 59, 61)`).
- Warps cursor to the Reconnect button and clicks via `/dev/uinput`.

### 6. FPS Capper (CPU Throttling)

- Throttles Sober via systemd user scopes or `SIGSTOP`/`SIGCONT` duty-cycle fallback.
- **Unlock on Focus**: Checks window `active` property from `ListWindows()`, temporarily clearing the quota when you focus Sober.

---

## Diagnostics in the App

Under the **Diagnostics** tab in AntiAFK:

- **GNOME Extension**:
    - `Active & connected via D-Bus` (Ready)
    - `Not installed` (shows **Install & Enable** button)
    - `Installed & enabled. Please log out and back in` (awaiting session restart)
- **Screen Capture**: reports `GNOME Extension active`.
- **Input Emulation**: checks `/dev/uinput` permissions.

---

## Troubleshooting

- **"GNOME Extension: Installed but disabled"**:
  Enable the extension via `gnome-extensions enable antiafk-rbx-sober@agzes.github.io` or the Extensions app.
- **"GNOME Extension: Not active / D-Bus error"**:
  Make sure you logged out and back in after the initial installation.
- **Forced Backend Selection**:
    ```bash
    AntiAFK-RBX-Sober --force-desktop gnome
    ```
