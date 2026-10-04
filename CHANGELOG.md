# Changelog

## [1.0.0] Universal Desktop & Native Integration - 2026-10-04

### Added

- **Multi-Desktop Wayland & X11 Support**:
    - **GNOME Shell (Wayland)**: Bundled and versioned GNOME Shell Extension (`antiafk-rbx-sober@agzes.github.io`) exporting a dedicated D-Bus backend for non-interactive window tracking, focus, pointer warping, and screenshots without portal prompts.
    - **Niri**: Scrollable-tiling integration via `$NIRI_SOCKET` IPC, `wtype` keyboard injection, and dedicated workspace hiding (`workspace "antiafk-rbx-sober-hidden"`).
    - **COSMIC Desktop**: Protocol integration via `cosmic-comp` toplevel management and calibrated absolute pointer emulation.
    - **Generic X11 & i3**: Window management via `wmctrl` and `xdotool`, native i3 IPC scratchpad hiding, and zero-permission XTest fallback when `/dev/uinput` is inaccessible.
- **Desktop Integration & App Lifecycle**:
    - One-click **Add to System** banner and diagnostics button installing `.desktop` launcher, hicolor icons, and local binary in `~/.local/bin`.
    - Automatic prompt to update the installed desktop copy when running a newly downloaded binary.
    - **AUR Detection**: Auto-detects AUR package installs via `pacman -Qo` and recommends updating or removing via your installed AUR helper (`yay` / `paru`).
    - **Full Uninstallation & Purge**: Options in the UI and CLI (`--uninstall`, `--purge`) to cleanly remove desktop launchers, icons, configuration files (`~/.config/antiafk-rbx-sober`), and GNOME extensions.
- **Complete UI Rewrite to Iced**:
    - Fully rewritten GUI built on **`iced` 0.14** with reactive Elm architecture, hardware-accelerated rendering (`wgpu` / `tiny-skia`), and custom fluid animations.
    - Replaced raster PNG icons with crisp, embedded vector SVGs sourced from Lucide Icons.
    - Native system theme detection and live accent color extraction from GTK 3/4 stylesheets (`~/.config/gtk-4.0/gtk.css`).
    - Interactive diagnostics dashboard with actionable preflight checks and one-click quick fixes.

### Changed

- Complete rewrite of the presentation layer from GTK/relm to `iced`.
- Refactored modular backend architecture isolating compositor-specific implementations under `src/inputs/`.
- Replaced polling with low-overhead process monitoring and kernel-level physical `evdev` activity detection.
- Upgraded dependencies (`notify-rust` 4.18.1, `iced` 0.14).

---

## [0.2.0] KDE Plasma 6 Support - 2026-03-28

### Added

- **KDE Plasma 6 (Wayland) Support**: Full support for KDE Plasma 6 environment.
    - New input method: **"Plasma (preview)"** using `qdbus6` and `KWin` for window management.
    - Non-interactive focus detection for Plasma using KWin scripting and journalctl.
    - Automatic desktop environment detection (Hyprland & Plasma).
- **UI Enhancements & Reliability**:
    - **Custom Icon Fallback**: New `get_safe_icon` system ensures icons display correctly even if standard symbolic icons are missing in the current theme.
    - **Theme fix & update**: Now fully using system gtk theme.
    - **Styles update**: Updated styling for a more nice look.

### Changed

- **Major Architectural Refactoring**:
    - Moved input simulation logic to a new module structure (`src/inputs/swapper.rs`, `src/inputs/plasma.rs`).
    - Decoupled `backend.rs` from specific input implementations for better maintainability.
- **Enhanced Process & Window Detection**:
    - Improved reliability of identifying Sober windows.

_And other minor fixes..._

---

## [0.1.0] Init Release - 2026-03-15

init release :D
