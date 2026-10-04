# COSMIC Desktop Backend Guide

AntiAFK-RBX-Sober provides native integration for the [COSMIC Desktop](https://system76.com/cosmic) (System76 / Pop!_OS) using native Wayland protocols (`cosmic-comp`) and `/dev/uinput`.

---

## Requirements

| Tool              | Purpose                         | Package                |
| :---------------- | :------------------------------ | :--------------------- |
| `cosmic-comp`     | Wayland compositor session      | Part of COSMIC Desktop |
| `grim` / `import` | Screen capture (Auto Reconnect) | `grim`                 |
| `/dev/uinput`     | Virtual pointer & keyboard      | Built-in kernel module |

---

## Architecture & How It Works

### 1. Native Wayland Protocol Integration

AntiAFK connects directly to the COSMIC Wayland display via `wayland-client`:

- **Window Enumeration & Activation**: Uses `zcosmic_toplevel_manager_v1` and `ext_foreign_toplevel_list_v1` protocols to monitor toplevel windows and activate Sober using `zcosmic_toplevel_handle_v1.activate`.
- **Hide Game (Stealth)**: Uses `zcosmic_toplevel_handle_v1.set_minimized()` to minimize and restore windows cleanly.

### 2. Absolute Pointer Emulation

Because Wayland compositors manage cursor positioning strictly within the compositor without exposing arbitrary cursor warping APIs to clients, AntiAFK sets up a virtual absolute mouse device (`ABSOLUTE_MOUSE_DEVICE`) over `/dev/uinput`:

- Calibrated to absolute coordinates `(0..32767)` mapped across display dimensions.
- Allows precise clicks on Sober windows and dialog buttons.

### 3. User Safe ("Don't Interrupt Me")

- Low-level physical `evdev` listening on `/dev/input/event*` for any keyboard or mouse input.
- Compositor focus change detection to detect user switching windows.
- Automatically pauses automation for 3 seconds whenever user activity is detected.

### 4. Auto Reconnect

- Captures display output via `grim`.
- Analyzes pixels for the dark-theme reconnect modal (`RGB(57, 59, 61)`).
- Emits clicks on the Reconnect button using the virtual absolute pointer.

### 5. Why FPS Capper is Disabled on COSMIC

The FPS Capper switch is intentionally disabled on COSMIC:

- Applying `systemd --user` `CPUQuota` or process freezer signals (`SIGSTOP`) to processes running inside `cosmic-comp` causes the compositor's Wayland event queue to detect a broken pipe, crashing the Sober client.
- To protect session stability, the FPS Capper toggle is safely disabled with an explanatory tooltip, and preflight prevents enabling it.

---

## Diagnostics in the App

Under the **Diagnostics** tab in AntiAFK:

- **Desktop Session**: reports `COSMIC (Wayland)`.
- **COSMIC Protocols**: reports protocol binding status (`Ready` when `zcosmic_toplevel_manager_v1` is available).
- **Screen Capture**: reports `grim utility detected`.
- **Input Emulation**: verifies `/dev/uinput` permissions.

---

## Troubleshooting

- **"Access to /dev/uinput is denied"**:
  Open **Diagnostics** in the app and click **Fix**, or add the uinput udev rule manually.
- **Low-Level Protocol Diagnostics**:
  To inspect COSMIC Wayland protocol advertisements, run:
    ```bash
    AntiAFK-RBX-Sober --diagnose-cosmic
    ```
- **Forced Backend Selection**:
    ```bash
    AntiAFK-RBX-Sober --force-desktop cosmic
    ```
