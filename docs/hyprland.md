# Hyprland Backend Guide

AntiAFK-RBX-Sober integrates natively with [Hyprland](https://hypr.land) via the `hyprctl` IPC interface and Linux `/dev/uinput`.

---

## Requirements

| Tool          | Purpose                         | Package                          |
| :------------ | :------------------------------ | :------------------------------- |
| `hyprctl`     | Window management & focus       | Part of Hyprland (pre-installed) |
| `grim`        | Screen capture (Auto Reconnect) | `grim`                           |
| `/dev/uinput` | Virtual keyboard and pointer    | Built-in kernel module           |

---

## Architecture & How It Works

### 1. Window Discovery & Focus

AntiAFK communicates directly with Hyprland through `hyprctl clients -j`:

- Scans all open windows across workspaces for matching Sober metadata (`class` containing `sober`, `org.vinegarhq.Sober`, or window title).
- Focuses the target window via:
    ```bash
    hyprctl dispatch focuswindow address:<addr>
    ```
- **Multi-Instance**: If multiple Sober instances are running, AntiAFK cycles through each window address sequentially with a 150 ms delay between activations.

### 2. Input Emulation & Cursor Preservation

- Inputs are delivered through virtual devices created via `/dev/uinput`.
- **Cursor Preservation**: AntiAFK queries your exact pointer coordinates before switching windows via `hyprctl cursorpos -j`. After dispatching actions (`Space`, `W`/`S`, `I`/`O`), it warps the cursor back to your original position via `warp_cursor`.
- **Restoring Active Window**: AntiAFK records the window address that had focus before automation triggered and returns focus to your active window.

### 3. User Safe ("Don't Interrupt Me")

AntiAFK protects your typing and manual gameplay with a dual-check mechanism:

1. **Physical Input Detection**: A kernel-level `evdev` listener monitors physical keyboards and mice (`/dev/input/event*`). Any physical keystroke or mouse movement immediately pauses automation for 3 seconds.
2. **Cursor Delta Check**: AntiAFK samples cursor position via `hyprctl cursorpos -j` over a 3-second window. Any movement greater than 2 pixels pauses automation.

### 4. Hide Game (Stealth Mode)

When **Hide Game** is enabled in Settings:

- AntiAFK moves the Sober window to Hyprland's special workspace without showing it on screen:
    ```bash
    hyprctl dispatch movetoworkspacesilent special:minimized,address:<addr>
    ```
- When unhiding, stopping automation, or quitting the app, windows are restored back to your active workspace via `movetoworkspace`.

### 5. Auto Reconnect

- When enabled, AntiAFK captures the Sober window using `grim`:
    ```bash
    grim -g "<x>,<y> <w>x<h>" /tmp/antiafk-sober-recon.png
    ```
- Inspects pixel data around Roblox's dark-theme modal (`RGB(57, 59, 61)` within tolerance and bright reconnect button text).
- Calculates exact Reconnect button coordinates relative to the window, moves the cursor, and clicks the button.

### 6. FPS Capper (CPU Throttling)

- Throttles Sober's systemd user cgroup scope via `systemctl --user set-property <scope> CPUQuota=X%` (or via precise `SIGSTOP`/`SIGCONT` duty cycle fallback).
- **Unlock on Focus**: When **Unlock limit when game is focused** is enabled, AntiAFK checks `hyprctl activewindow -j` and temporarily clears the throttle while you are actively playing.

---

## Diagnostics in the App

Under the **Diagnostics** tab in AntiAFK:

- **Desktop Session**: reports `Hyprland (Wayland)`.
- **hyprctl Utility**: verifies `hyprctl version` execution.
- **Screen Capture**: reports `grim utility detected`.
- **Input Emulation**: verifies `/dev/uinput` write permissions.

---

## Troubleshooting

- **"Access to /dev/uinput is denied"**:
  Open **Diagnostics** in the app and click **Fix**, or run:
    ```bash
    echo 'KERNEL=="uinput", MODE="0666"' | sudo tee /etc/udev/rules.d/99-uinput-antiafk.rules
    sudo udevadm control --reload-rules && sudo udevadm trigger
    ```
- **"grim utility missing"**:
  Install `grim` using your package manager (`sudo pacman -S grim` / `sudo dnf install grim`).
- **Forced Backend Selection**:
  If Hyprland is run in a nested or customized session, start with:
    ```bash
    AntiAFK-RBX-Sober --force-desktop hyprland
    ```
