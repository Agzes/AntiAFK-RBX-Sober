use std::env;
use std::sync::RwLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    Hyprland = 0,
    Kde = 1,
    Niri = 2,
    Cosmic = 3,
    X11 = 4,
    Gnome = 5,
    I3 = 6,
    Unsupported = 7,
}

impl InputMode {
    pub const fn from_legacy(value: usize) -> Self {
        match value {
            0 => Self::Hyprland,
            1 => Self::Kde,
            2 => Self::Niri,
            3 => Self::Cosmic,
            4 => Self::X11,
            5 => Self::Gnome,
            6 => Self::I3,
            _ => Self::Unsupported,
        }
    }

    pub const fn as_legacy(self) -> usize {
        self as usize
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Hyprland => "Hyprland",
            Self::Kde => "KDE Plasma 6 (Wayland)",
            Self::Niri => "NIRI",
            Self::Cosmic => "COSMIC",
            Self::X11 => "X11",
            Self::Gnome => "GNOME (Wayland)",
            Self::I3 => "i3 (X11)",
            Self::Unsupported => "Unsupported",
        }
    }

    pub const fn is_supported(self) -> bool {
        !matches!(self, Self::Unsupported)
    }

    pub const fn uses_uinput(self) -> bool {
        !matches!(self, Self::X11 | Self::I3)
    }

    pub fn parse_force_name(value: &str) -> Result<Option<Self>, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(None),
            "hyprland" => Ok(Some(Self::Hyprland)),
            "kde" | "plasma" => Ok(Some(Self::Kde)),
            "niri" => Ok(Some(Self::Niri)),
            "cosmic" => Ok(Some(Self::Cosmic)),
            "x11" => Ok(Some(Self::X11)),
            "gnome" => Ok(Some(Self::Gnome)),
            "i3" => Ok(Some(Self::I3)),
            _ => Err(format!("unsupported desktop override: {value}")),
        }
    }

    pub fn display_label(self) -> String {
        if forced_desktop().is_some() {
            format!("{} (forced)", self.label())
        } else {
            self.label().to_string()
        }
    }
}

static FORCED_DESKTOP: RwLock<Option<InputMode>> = RwLock::new(None);

pub fn force_desktop(mode: Option<InputMode>) {
    *FORCED_DESKTOP
        .write()
        .expect("desktop override lock poisoned") = mode;
}

pub fn forced_desktop() -> Option<InputMode> {
    *FORCED_DESKTOP
        .read()
        .expect("desktop override lock poisoned")
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EnvSnapshot {
    pub current_desktop: Option<String>,
    pub session_desktop: Option<String>,
    pub session_type: Option<String>,
    pub wayland_display: bool,
    pub display: bool,
    pub hyprland_instance: bool,
    pub niri_socket: bool,
    pub kde_full_session: bool,
    pub gnome_session: bool,
    pub i3_socket: bool,
}

impl EnvSnapshot {
    pub fn capture() -> Self {
        Self {
            current_desktop: env::var("XDG_CURRENT_DESKTOP").ok(),
            session_desktop: env::var("XDG_SESSION_DESKTOP").ok(),
            session_type: env::var("XDG_SESSION_TYPE").ok(),
            wayland_display: env::var_os("WAYLAND_DISPLAY").is_some(),
            display: env::var_os("DISPLAY").is_some(),
            hyprland_instance: env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some(),
            niri_socket: env::var_os("NIRI_SOCKET").is_some(),
            kde_full_session: env::var_os("KDE_FULL_SESSION").is_some(),
            gnome_session: env::var_os("GNOME_DESKTOP_SESSION_ID").is_some(),
            i3_socket: env::var_os("I3SOCK").is_some(),
        }
    }

    fn desktop_tokens(&self) -> impl Iterator<Item = String> + '_ {
        [
            self.current_desktop.as_deref(),
            self.session_desktop.as_deref(),
        ]
        .into_iter()
        .flatten()
        .flat_map(|value| value.split(':'))
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| !token.is_empty())
    }

    fn has_desktop_token(&self, expected: &str) -> bool {
        self.desktop_tokens().any(|token| token == expected)
    }

    fn session_is_wayland(&self) -> bool {
        self.session_type
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case("wayland"))
            .unwrap_or(self.wayland_display)
    }

    fn session_is_x11(&self) -> bool {
        self.session_type
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case("x11"))
            .unwrap_or(self.display && !self.session_is_wayland())
    }
}

pub fn detect_mode() -> InputMode {
    if let Some(forced) = forced_desktop() {
        return forced;
    }
    detect_mode_from(&EnvSnapshot::capture())
}

pub fn is_hyprland() -> bool {
    env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

#[allow(dead_code)]
pub fn is_kde() -> bool {
    env::var_os("KDE_FULL_SESSION").is_some()
        || ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP"]
            .iter()
            .any(|key| {
                env::var(key).is_ok_and(|value| {
                    let value = value.to_ascii_lowercase();
                    value.contains("kde") || value.contains("plasma")
                })
            })
}

pub fn detect_mode_from(snapshot: &EnvSnapshot) -> InputMode {
    if snapshot.hyprland_instance || snapshot.has_desktop_token("hyprland") {
        return InputMode::Hyprland;
    }

    if (snapshot.niri_socket || snapshot.has_desktop_token("niri")) && snapshot.session_is_wayland()
    {
        return InputMode::Niri;
    }

    if snapshot.has_desktop_token("cosmic") {
        return if snapshot.session_is_wayland() {
            InputMode::Cosmic
        } else {
            InputMode::X11
        };
    }

    if snapshot.kde_full_session
        || snapshot.has_desktop_token("kde")
        || snapshot.has_desktop_token("plasma")
    {
        return if snapshot.session_is_wayland() {
            InputMode::Kde
        } else {
            InputMode::X11
        };
    }

    if snapshot.gnome_session || snapshot.has_desktop_token("gnome") {
        return if snapshot.session_is_wayland() {
            InputMode::Gnome
        } else {
            InputMode::X11
        };
    }

    if snapshot.i3_socket || snapshot.has_desktop_token("i3") {
        return InputMode::I3;
    }

    if snapshot.session_is_x11() {
        InputMode::X11
    } else {
        InputMode::Unsupported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wayland(desktop: Option<&str>) -> EnvSnapshot {
        EnvSnapshot {
            current_desktop: desktop.map(ToOwned::to_owned),
            session_type: Some("wayland".to_string()),
            wayland_display: true,
            ..EnvSnapshot::default()
        }
    }

    #[test]
    fn parses_desktop_override_names() {
        assert_eq!(InputMode::parse_force_name("auto").unwrap(), None);
        assert_eq!(
            InputMode::parse_force_name("COSMIC").unwrap(),
            Some(InputMode::Cosmic)
        );
        assert_eq!(
            InputMode::parse_force_name("i3").unwrap(),
            Some(InputMode::I3)
        );
        assert!(InputMode::parse_force_name("unknown").is_err());
    }

    #[test]
    fn legacy_values_are_stable() {
        assert_eq!(InputMode::from_legacy(0), InputMode::Hyprland);
        assert_eq!(InputMode::from_legacy(1), InputMode::Kde);
        assert_eq!(InputMode::from_legacy(99), InputMode::Unsupported);
    }

    #[test]
    fn detects_wayland_backends() {
        assert_eq!(
            detect_mode_from(&EnvSnapshot {
                hyprland_instance: true,
                ..wayland(None)
            }),
            InputMode::Hyprland
        );
        assert_eq!(detect_mode_from(&wayland(Some("NIRI"))), InputMode::Niri);
        assert_eq!(
            detect_mode_from(&wayland(Some("COSMIC"))),
            InputMode::Cosmic
        );
        assert_eq!(detect_mode_from(&wayland(Some("GNOME"))), InputMode::Gnome);
    }

    #[test]
    fn does_not_mistake_xwayland_display_for_x11() {
        let snapshot = EnvSnapshot {
            current_desktop: Some("GNOME".to_string()),
            session_type: Some("wayland".to_string()),
            wayland_display: true,
            display: true,
            ..EnvSnapshot::default()
        };
        assert_eq!(detect_mode_from(&snapshot), InputMode::Gnome);
    }

    #[test]
    fn detects_x11_and_i3_without_uinput() {
        let x11 = EnvSnapshot {
            session_type: Some("x11".to_string()),
            display: true,
            ..EnvSnapshot::default()
        };
        assert_eq!(detect_mode_from(&x11), InputMode::X11);
        assert!(!InputMode::X11.uses_uinput());
        assert!(!InputMode::I3.uses_uinput());

        let i3 = EnvSnapshot {
            current_desktop: Some("i3".to_string()),
            session_type: Some("x11".to_string()),
            display: true,
            ..EnvSnapshot::default()
        };
        assert_eq!(detect_mode_from(&i3), InputMode::I3);
    }
}
