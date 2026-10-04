# KDE Plasma 6 Backend Guide

AntiAFK-RBX-Sober integrates natively with [KDE Plasma 6 (Wayland)](https://kde.org/plasma-desktop/) using KWin's D-Bus Scripting API (`org.kde.KWin /Scripting`) and `/dev/uinput`.

---

## Requirements

| Tool               | Purpose                              | Package                            |
| :----------------- | :----------------------------------- | :--------------------------------- |
| `qdbus6` / `qdbus` | D-Bus communication with KWin        | `qt6-tools` / `qttools5-dev-tools` |
| `journalctl`       | Reading KWin script execution output | Part of systemd                    |
| `spectacle`        | Window screenshot for Auto Reconnect | `spectacle`                        |
| `/dev/uinput`      | Virtual input emulation              | Built-in kernel module             |

---

## Architecture & How It Works

### 1. KWin Scripting Bridge

Under Wayland, unprivileged applications cannot query window lists or geometries directly. AntiAFK uploads and runs ephemeral JavaScript scripts into KWin via D-Bus:

```bash
qdbus6 org.kde.KWin /Scripting loadScript "/path/to/script.js" "antiafk_query"
qdbus6 org.kde.KWin /Scripting/Script<id> run
```

- The script inspects `workspace.windowList()` for Sober clients (`client.caption`, `client.resourceClass`, or `client.resourceName`).
- AntiAFK reads script output from `journalctl` using unique random marker tokens to avoid race conditions.
- Target windows are focused with `workspace.activeWindow = client`.

### 2. Multi-Instance Support

- AntiAFK queries `get_target_window_count` to discover how many Sober windows exist.
- When **Multi-Instance** is enabled, it cycles through all instances by index (`minimize_window_by_index`), activates each window sequentially, executes anti-AFK actions, and checks reconnect status.

### 3. Cursor Preservation & User Safety

- **Cursor Tracking**: Queries `workspace.cursorPos` through KWin scripting before switching windows, and restores cursor position immediately after actions finish.
- **User Safety ("Don't Interrupt Me")**: Dual-layer detection combining:
    1. Low-level physical `evdev` event listener on `/dev/input/event*` for any physical keystroke or mouse movement.
    2. KWin script cursor polling over a 3-second window. Any activity delays automation by 3 seconds.

### 4. Hide Game (Stealth Mode)

- AntiAFK sets `client.minimized = true` on Sober windows via an inline KWin script.
- When stopping automation, unhiding, or closing the application, windows are restored with `client.minimized = false`.

### 5. Auto Reconnect

- Captures the active Sober window using `spectacle` in background mode:
    ```bash
    spectacle -b -n -o /tmp/antiafk-sober-kde.png
    ```
- AntiAFK analyzes pixel data to detect Roblox's disconnected modal (`RGB(57, 59, 61)`).
- Emits button click events via `/dev/uinput`.

### 6. FPS Capper (CPU Throttling)

- Throttles Sober via `systemctl --user set-property <scope> CPUQuota=X%` or `SIGSTOP`/`SIGCONT` duty-cycle fallback.
- **Unlock on Focus**: When **Unlock limit when game is focused** is enabled, AntiAFK checks `workspace.activeWindow` via KWin script and temporarily clears the throttle while you are playing in Sober.

---

## Diagnostics in the App

Under the **Diagnostics** tab in AntiAFK:

- **KDE D-Bus (qdbus)**: checks availability of `qdbus6` or `qdbus`.
- **KWin Script Bridge**: runs a test script and verifies journal output.
    - If output is silenced, an **Enable Logging** button appears in Diagnostics to enable KWin debug logging automatically!
- **Screen Capture**: checks for `spectacle`.
- **Input Emulation**: verifies `/dev/uinput` write access.

---

## Troubleshooting

- **"KWin Script Bridge: KWin does not forward script output to the journal"**:
  By default, Plasma may silence KWin script console logging. To fix:
    1. Click **Enable Logging** under the KWin Script Bridge row in Diagnostics, or run:
        ```bash
        systemctl --user set-environment QT_LOGGING_RULES=kwin_*.debug=true
        ```
    2. Log out and back into your Plasma session to apply.
- **"spectacle command is missing"**:
  Install `spectacle` via your package manager (`sudo pacman -S spectacle` / `sudo dnf install spectacle`).
- **Forced Backend Selection**:
    ```bash
    AntiAFK-RBX-Sober --force-desktop kde
    ```
