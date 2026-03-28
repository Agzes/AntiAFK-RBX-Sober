# Changelog

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

*And other minor fixes...*

---

## [0.1.0] Init Release - 2026-03-15
init release :D