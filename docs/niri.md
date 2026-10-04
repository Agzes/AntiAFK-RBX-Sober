# Niri Backend Guide

AntiAFK-RBX-Sober provides native integration for [Niri](https://github.com/YaLTeR/niri), the scrollable-tiling Wayland compositor, via Niri's JSON IPC socket (`$NIRI_SOCKET`), `wtype`, and `/dev/uinput`.

---

## Requirements

| Tool          | Purpose                           | Package                |
| :------------ | :-------------------------------- | :--------------------- |
| `niri`        | Window manager & IPC (`niri msg`) | Part of Niri session   |
| `wtype`       | Virtual keyboard emulation        | `wtype`                |
| `grim`        | Screen capture (Auto Reconnect)   | `grim`                 |
| `/dev/uinput` | Virtual pointer emulation         | Built-in kernel module |

> **Why `wtype` is required on Niri**:
> By design, Niri's `libinput` integration ignores virtual keyboard input sent through `/dev/uinput` to isolate sessions and prevent unauthorized keystroke injection. `wtype` injects keystrokes directly through Wayland's `zwp_virtual_keyboard_v1` protocol, ensuring keys reliably reach the game.

---

## Architecture & How It Works

### 1. Niri Socket Communication

AntiAFK connects to `$NIRI_SOCKET` and uses `niri msg --json`:

- `niri msg --json windows`: lists all open windows, workspaces, and window geometries.
- Identifies Sober by matching `app_id` (e.g. `org.vinegarhq.Sober` or `sober`) and window title.
- Focuses windows via:
    ```bash
    niri msg action focus-window --id <id>
    ```
- **Multi-Instance**: Cycles through all matching Sober window IDs sequentially.

### 2. Event-Stream Activity Monitoring

AntiAFK listens to Niri's event stream in real time:

```bash
niri msg --json event-stream
```

Detects window focus changes, workspace switches, and layout modifications to keep tracking synchronized.

### 3. User Safe ("Don't Interrupt Me")

Dual-layer activity detection:

1. Physical `evdev` event monitoring on `/dev/input/event*` for any keyboard or mouse input.
2. Niri event stream listener detecting user navigation.
   Automation immediately pauses for 3 seconds whenever activity is detected.

### 4. Hide Game (Dedicated Workspace)

Niri arranges windows in dynamic horizontal ribbons across named workspaces. To hide Sober cleanly without breaking your tiling column layout:

- AntiAFK moves the Sober window to a dedicated workspace:
    ```kdl
    workspace "antiafk-rbx-sober-hidden"
    ```
- **One-Click Workspace Creation**: Under the **Diagnostics** tab in AntiAFK, click **Create** next to the **Niri Hidden Workspace** row. AntiAFK will safely validate and append the workspace declaration to `~/.config/niri/config.kdl` using `niri validate`.
- When stopping or unhiding, Sober is restored back to your active workspace.

### 5. Auto Reconnect

- Captures output/window using `grim`.
- Analyzes pixels for Roblox's disconnected modal (`RGB(57, 59, 61)`).
- Warps cursor to the Reconnect button and clicks via `/dev/uinput`.

### 6. FPS Capper (CPU Throttling)

- Throttles Sober via systemd user scopes or `SIGSTOP`/`SIGCONT` duty cycle.
- **Unlock on Focus**: Queries `niri msg --json focused-window` and temporarily unthrottles when Sober is active.

---

## Diagnostics in the App

Under the **Diagnostics** tab in AntiAFK:

- **Niri Socket**: confirms `$NIRI_SOCKET` is connected.
- **wtype Utility**: verifies `wtype` is installed.
- **Niri Window Target**: shows count of detected Sober windows out of total open windows.
- **Niri Hidden Workspace**: displays workspace readiness with **Create** and **Remove** action buttons.
- **Screen Capture**: reports `grim utility detected`.
- **Input Emulation**: verifies `/dev/uinput` write access.

---

## Troubleshooting

- **"wtype utility: Missing"**:
  Install `wtype` using your package manager (`sudo pacman -S wtype` / `sudo dnf install wtype`).
- **"Niri Hidden Workspace not configured"**:
  Click **Create** in the app's Diagnostics tab to automatically configure your `config.kdl`.
- **Forced Backend Selection**:
    ```bash
    AntiAFK-RBX-Sober --force-desktop niri
    ```
