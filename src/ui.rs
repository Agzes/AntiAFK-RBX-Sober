use crate::DesktopIntegrationStatus;
use crate::GNOME_EXTENSION_UUID;
use crate::environment::InputMode;
use crate::state::APP_TITLE;
use crate::state::{AppState, RuntimeStatus, SharedState, set_runtime_error, set_runtime_status};
use crate::tray::{TrayBridge, TrayCommand};
use iced::alignment::{Horizontal, Vertical};
use iced::theme::Mode as ThemeMode;
use iced::widget::tooltip::Position as TooltipPosition;
use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, stack, svg, text, text_input,
    tooltip,
};
use iced::{
    Background, Border, Color, Element, Fill, Font, Length, Padding, Shadow, Size, Subscription,
    Task, Theme, font, time, widget::row::Row, window,
};
use std::fmt;
use std::process::Command;
use std::time::Duration;

const CURRENT_VERSION: &str = "0.2.0";
const REPOSITORY_URL: &str = "https://github.com/Agzes/AntiAFK-RBX-Sober";
const RELEASES_URL: &str = "https://github.com/Agzes/AntiAFK-RBX-Sober/releases";
const DONATE_URL: &str = "https://agzes.github.io/donate";
const DISCORD_URL: &str = "https://agzes.github.io/go/to/discord";
const UINPUT_RULE: &str = "/etc/udev/rules.d/99-uinput-antiafk.rules";
const BETA_NOTE: &str = "Beta feature: may be unstable. Please check how it behaves on your own PC and report problems.";
const TICK: Duration = Duration::from_millis(250);
const FRAME: Duration = Duration::from_millis(16);

const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

const SEMIBOLD: Font = Font {
    weight: font::Weight::Semibold,
    ..Font::DEFAULT
};

const RBX_RED: Color = Color::from_rgb(0.886, 0.137, 0.102);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Icon {
    Interval,
    Action,
    Autostart,
    Mouse,
    Multi,
    Hide,
    Reconnect,
    Performance,
    Cpu,
    Focus,
    Settings,
    Info,
    Check,
    Warning,
    Repository,
    Heart,
    Chat,
    Palette,
    Moon,
    Layers,
    Beta,
    Close,
}

impl Icon {
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Interval => include_bytes!("../assets/icons/interval.svg"),
            Self::Action => include_bytes!("../assets/icons/action.svg"),
            Self::Autostart => include_bytes!("../assets/icons/autostart.svg"),
            Self::Mouse => include_bytes!("../assets/icons/mouse.svg"),
            Self::Multi => include_bytes!("../assets/icons/multi.svg"),
            Self::Hide => include_bytes!("../assets/icons/hide.svg"),
            Self::Reconnect => include_bytes!("../assets/icons/reconnect.svg"),
            Self::Performance => include_bytes!("../assets/icons/performance.svg"),
            Self::Cpu => include_bytes!("../assets/icons/cpu.svg"),
            Self::Focus => include_bytes!("../assets/icons/focus.svg"),
            Self::Settings => include_bytes!("../assets/icons/settings.svg"),
            Self::Info => include_bytes!("../assets/icons/info.svg"),
            Self::Check => include_bytes!("../assets/icons/check.svg"),
            Self::Warning => include_bytes!("../assets/icons/warning.svg"),
            Self::Repository => include_bytes!("../assets/icons/repository.svg"),
            Self::Heart => include_bytes!("../assets/icons/heart.svg"),
            Self::Chat => include_bytes!("../assets/icons/chat.svg"),
            Self::Palette => include_bytes!("../assets/icons/palette.svg"),
            Self::Moon => include_bytes!("../assets/icons/moon.svg"),
            Self::Layers => include_bytes!("../assets/icons/layers.svg"),
            Self::Beta => include_bytes!("../assets/icons/beta.svg"),
            Self::Close => include_bytes!("../assets/icons/close.svg"),
        }
    }
}

fn icon<'a>(which: Icon, size: f32, color: Color) -> Element<'a, Message> {
    let handle = svg::Handle::from_memory(which.bytes());
    svg(handle)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_theme, _status| svg::Style { color: Some(color) })
        .into()
}

#[derive(Clone, Copy)]
struct Palette {
    fg: Color,
    muted: Color,
    faint: Color,
    accent: Color,
    on_accent: Color,
    success: Color,
    warning: Color,
    danger: Color,
    surface: Color,
    surface_hover: Color,
    card: Color,
    strip: Color,
    hairline: Color,
    border: Color,
}

impl Palette {
    fn of(theme: &Theme) -> Self {
        let base = theme.palette();
        let extended = theme.extended_palette();
        let fg = base.text;
        let card = if extended.is_dark {
            Color::from_rgb8(0x1C, 0x1E, 0x24)
        } else {
            Color::WHITE
        };
        Self {
            fg,
            muted: alpha(fg, 0.62),
            faint: alpha(fg, 0.38),
            accent: base.primary,
            on_accent: extended.primary.base.text,
            success: base.success,
            warning: base.warning,
            danger: base.danger,
            surface: alpha(fg, 0.05),
            surface_hover: alpha(fg, 0.11),
            card,
            strip: mix(base.background, card, 0.5),
            hairline: alpha(fg, 0.08),
            border: alpha(fg, 0.14),
        }
    }
}

fn select_style(palette: Palette) -> impl Fn(&Theme, pick_list::Status) -> pick_list::Style {
    move |_theme, status| pick_list::Style {
        text_color: palette.fg,
        placeholder_color: palette.muted,
        handle_color: palette.muted,
        background: Background::Color(match status {
            pick_list::Status::Hovered => palette.surface_hover,
            _ => palette.surface,
        }),
        border: Border {
            color: palette.border,
            width: 1.0,
            radius: 9.0.into(),
        },
    }
}

fn input_style(palette: Palette) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_theme, _status| text_input::Style {
        background: Background::Color(palette.surface),
        border: Border {
            color: palette.border,
            width: 1.0,
            radius: 9.0.into(),
        },
        icon: palette.muted,
        placeholder: palette.faint,
        value: palette.fg,
        selection: palette.accent,
    }
}

fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

const SUCCESS_DARK: Color = Color::from_rgb8(0x4A, 0xDE, 0x80);
const SUCCESS_LIGHT: Color = Color::from_rgb8(0x15, 0x80, 0x3D);
const DANGER_DARK: Color = Color::from_rgb8(0xF8, 0x71, 0x71);
const DANGER_LIGHT: Color = Color::from_rgb8(0xDC, 0x26, 0x26);

fn custom_theme(dark: bool, accent: Option<Color>, system: Option<SystemColors>) -> Theme {
    let mut palette = if dark {
        iced::theme::Palette {
            background: Color::from_rgb8(0x15, 0x16, 0x1A),
            text: Color::from_rgb8(0xE7, 0xE9, 0xEF),
            primary: accent.unwrap_or(Color::from_rgb8(0x8B, 0x9B, 0xFF)),
            success: SUCCESS_DARK,
            warning: Color::from_rgb8(0xFB, 0xBF, 0x24),
            danger: DANGER_DARK,
        }
    } else {
        iced::theme::Palette {
            background: Color::from_rgb8(0xF4, 0xF5, 0xF8),
            text: Color::from_rgb8(0x1A, 0x1C, 0x21),
            primary: accent.unwrap_or(Color::from_rgb8(0x4F, 0x46, 0xE5)),
            success: SUCCESS_LIGHT,
            warning: Color::from_rgb8(0xB4, 0x5D, 0x09),
            danger: DANGER_LIGHT,
        }
    };

    if let Some(system) = system {
        palette.background = system.background;
        palette.text = system.text;
        palette.success = system.success;
        palette.danger = system.danger;
    }

    Theme::custom(
        if dark {
            "AntiAFK Dark"
        } else {
            "AntiAFK Light"
        },
        palette,
    )
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

const INTERVALS: [Interval; 5] = [
    Interval::Min4,
    Interval::Min6,
    Interval::Min9,
    Interval::Min19,
    Interval::Custom,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Interval {
    Min4,
    Min6,
    Min9,
    Min19,
    Custom,
}

impl Interval {
    fn seconds(self) -> Option<u64> {
        match self {
            Self::Min4 => Some(240),
            Self::Min6 => Some(360),
            Self::Min9 => Some(540),
            Self::Min19 => Some(1140),
            Self::Custom => None,
        }
    }

    fn from_seconds(seconds: u64) -> Self {
        match seconds {
            240 => Self::Min4,
            360 => Self::Min6,
            540 => Self::Min9,
            1140 => Self::Min19,
            _ => Self::Custom,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Min4 => "4 min",
            Self::Min6 => "6 min",
            Self::Min9 => "9 min",
            Self::Min19 => "19 min",
            Self::Custom => "Custom",
        }
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

const ACTIONS: [IdleAction; 3] = [IdleAction::Jump, IdleAction::Walk, IdleAction::Camera];

const FLAGS: [Flag; Flag::COUNT] = [
    Flag::MultiInstance,
    Flag::AutoStart,
    Flag::UserSafe,
    Flag::Stealth,
    Flag::AutoReconnect,
    Flag::FpsCapper,
    Flag::StopLimitOnFocus,
    Flag::ThemeSync,
    Flag::ThemeDark,
    Flag::SystemColors,
];

fn flag_value(state: &AppState, flag: Flag) -> bool {
    match flag {
        Flag::MultiInstance => state.multi_instance,
        Flag::AutoStart => state.auto_start,
        Flag::UserSafe => state.user_safe,
        Flag::Stealth => state.stealth,
        Flag::AutoReconnect => state.auto_reconnect,
        Flag::FpsCapper => state.fps_capper,
        Flag::StopLimitOnFocus => state.stop_limit_on_focus,
        Flag::ThemeSync => state.theme_sync,
        Flag::ThemeDark => state.theme_dark,
        Flag::SystemColors => state.system_colors,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IdleAction {
    Jump,
    Walk,
    Camera,
}

impl IdleAction {
    fn label(self) -> &'static str {
        match self {
            Self::Jump => "Jump",
            Self::Walk => "Walk",
            Self::Camera => "Camera",
        }
    }

    fn from_state(state: &AppState) -> Self {
        if state.walk {
            Self::Walk
        } else if state.spin_jiggle {
            Self::Camera
        } else {
            Self::Jump
        }
    }

    fn apply(self, state: &mut AppState) {
        state.jump = self == Self::Jump;
        state.walk = self == Self::Walk;
        state.spin_jiggle = self == Self::Camera;
    }
}

impl fmt::Display for IdleAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

fn format_minutes(seconds: u64) -> String {
    let minutes = seconds as f64 / 60.0;
    let text = format!("{minutes:.1}");
    text.strip_suffix(".0").map(str::to_string).unwrap_or(text)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flag {
    MultiInstance,
    AutoStart,
    UserSafe,
    Stealth,
    AutoReconnect,
    FpsCapper,
    StopLimitOnFocus,
    ThemeSync,
    ThemeDark,
    SystemColors,
}

impl Flag {
    const COUNT: usize = 10;

    fn index(self) -> usize {
        self as usize
    }

    fn apply(self, state: &mut AppState, value: bool) {
        match self {
            Self::MultiInstance => state.multi_instance = value,
            Self::AutoStart => {
                if value {
                    state.manually_stopped = false;
                }
                state.auto_start = value;
            }
            Self::UserSafe => state.user_safe = value,
            Self::Stealth => state.stealth = value,
            Self::AutoReconnect => state.auto_reconnect = value,
            Self::FpsCapper => state.fps_capper = value,
            Self::StopLimitOnFocus => state.stop_limit_on_focus = value,
            Self::ThemeSync => state.theme_sync = value,
            Self::ThemeDark => state.theme_dark = value,
            Self::SystemColors => state.system_colors = value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Settings,
    Diagnostics,
}

#[derive(Clone, Debug)]
enum Message {
    Noop,
    Tick,
    Frame,
    ThemeModeChanged(ThemeMode),
    WindowOpened(window::Id),
    CloseRequested(window::Id),
    DismissWelcome,
    DismissGnomeModal,
    ToggleRunning,
    OpenDiagnostics,
    ToggleDiagnostics,
    OpenUrl(&'static str),
    SelectInterval(Interval),
    CustomMinutesChanged(String),
    ApplyCustomMinutes,
    SelectAction(IdleAction),
    SetFlag(Flag, bool),
    FpsLimitChanged(String),
    RefreshDiagnostics,
    CheckVersion,
    VersionChecked(Result<(bool, String), String>),
    RunUinputCommand(bool),
    UinputCommandFinished,
    GnomeHelperInstalled(Result<String, String>),
    RunGnomeQuickSetup,
    EnableKWinLogging,
    CreateNiriWorkspace,
    RemoveNiriWorkspace,
    InstallDesktopEntry,
    DesktopEntryInstalled(Result<(), String>),
    UninstallDesktopEntry,
    DesktopEntryUninstalled(Result<(), String>),
    DismissUpdateInstalledModal,
    DismissAurUpdateModal,
    ShowAurUpdateModal,
    ShowPurgeModal,
    DismissPurgeModal,
    ConfirmPurgeAndExit,
    ShowAurUninstallModal,
    DismissAurUninstallModal,
    DismissInstallBanner,
    DismissAurUpdateBanner,
}

#[derive(Clone, Copy)]
struct Caps {
    enumeration: bool,
    hide: bool,
    capture: bool,
    quota: bool,
}

impl Caps {
    fn of(mode: InputMode) -> Self {
        Self {
            enumeration: crate::backend::supports_enumeration(mode),
            hide: crate::backend::supports_hide(mode),
            capture: crate::backend::supports_capture(mode),
            quota: crate::backend::cpu_quota_supported(mode),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiagStatus {
    Ok,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug)]
enum DiagAction {
    Fix,
    RemoveRule,
    CheckVersion,
    RunGnomeQuickSetup,
    EnableKWinLogging,
    CreateNiriWorkspace,
    RemoveNiriWorkspace,
    InstallDesktop,
    UpdateDesktop,
    UninstallDesktop,
    ShowAurUpdate,
    ShowAurUninstall,
    OpenReleases,
}

struct DiagRow {
    icon: Icon,
    title: String,
    detail: Option<String>,
    status: DiagStatus,
    action: Option<DiagAction>,
    action_disabled: bool,
    label: Option<String>,
}

const INLINE_DETAIL_MAX: usize = 32;

impl DiagRow {
    fn inline_detail(&self) -> Option<&str> {
        self.detail
            .as_deref()
            .filter(|value| value.chars().count() <= INLINE_DETAIL_MAX)
    }

    fn subtitle(&self) -> Option<&str> {
        self.detail
            .as_deref()
            .filter(|value| value.chars().count() > INLINE_DETAIL_MAX)
    }
}

#[derive(Default, Clone)]
struct VersionState {
    checking: bool,
    result: Option<Result<(bool, String), String>>,
}

pub struct App {
    shared: SharedState,
    tray: Option<TrayBridge>,
    page: Page,
    show_welcome: bool,
    dark: bool,
    system_mode: ThemeMode,
    accent: Option<Color>,
    system_colors: Option<SystemColors>,
    window: Option<window::Id>,
    window_open: bool,
    snapshot: AppState,
    mode: InputMode,
    caps: Caps,
    interval: Interval,
    custom_minutes: String,
    action: IdleAction,
    fps_limit: String,
    version: VersionState,
    diagnostics: Vec<DiagRow>,
    switches: [f32; Flag::COUNT],
    button_anim: f32,
    show_gnome_relogin_modal: bool,
    desktop_status: DesktopIntegrationStatus,
    show_update_installed_modal: bool,
    show_aur_update_modal: bool,
    show_purge_modal: bool,
    show_aur_uninstall_modal: bool,
    dismissed_install_banner: bool,
    dismissed_aur_update_banner: bool,
}

impl App {
    fn new(shared: SharedState, start_hidden: bool) -> (Self, Task<Message>) {
        let snapshot = { shared.lock().unwrap().clone() };
        let mode = snapshot.input_mode();
        let switches = std::array::from_fn(|index| {
            if flag_value(&snapshot, FLAGS[index]) {
                1.0
            } else {
                0.0
            }
        });
        let button_anim = if snapshot.running { 1.0 } else { 0.0 };
        let system_dark = system_dark_preference();
        let system_mode = if system_dark.unwrap_or(false) {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
        let dark = if snapshot.theme_sync {
            system_dark.unwrap_or(false)
        } else {
            snapshot.theme_dark
        };
        let accent = snapshot.theme_sync.then(|| system_accent(dark)).flatten();
        let system_colors = snapshot
            .system_colors
            .then(|| system_palette(dark))
            .flatten();
        let desktop_status = crate::check_desktop_integration();
        let show_update_installed_modal = matches!(
            desktop_status,
            DesktopIntegrationStatus::Installed { up_to_date: false }
        );
        let mut app = Self {
            shared,
            tray: None,
            page: Page::Settings,
            show_welcome: !snapshot.shown_warning,
            dark,
            system_mode,
            accent,
            system_colors,
            window: None,
            window_open: false,
            interval: Interval::from_seconds(snapshot.interval_seq),
            custom_minutes: format_minutes(snapshot.interval_seq),
            action: IdleAction::from_state(&snapshot),
            fps_limit: snapshot.fps_limit.to_string(),
            version: VersionState {
                checking: true,
                result: None,
            },
            diagnostics: Vec::new(),
            mode,
            caps: Caps::of(mode),
            snapshot,
            switches,
            button_anim,
            show_gnome_relogin_modal: false,
            desktop_status,
            show_update_installed_modal,
            show_aur_update_modal: false,
            show_purge_modal: false,
            show_aur_uninstall_modal: false,
            dismissed_install_banner: false,
            dismissed_aur_update_banner: false,
        };
        app.sync();
        app.refresh_diagnostics();
        app.tray = TrayBridge::spawn(app.shared.clone());

        if let Err(error) = crate::backend::preflight(&app.snapshot) {
            set_runtime_error(&app.shared, error);
            app.sync();
        }

        let mut tasks = vec![
            iced::system::theme().map(Message::ThemeModeChanged),
            Task::perform(check_latest_version(), Message::VersionChecked),
        ];
        if !start_hidden {
            tasks.push(app.show_window());
        }
        (app, Task::batch(tasks))
    }

    fn show_window(&mut self) -> Task<Message> {
        let (id, open) = window::open(window_settings());
        self.window = Some(id);
        self.window_open = true;
        open.map(Message::WindowOpened)
    }

    fn theme(&self) -> Theme {
        custom_theme(self.dark, self.accent, self.system_colors)
    }

    fn refresh_theme(&mut self) {
        self.dark = if self.snapshot.theme_sync {
            system_dark_preference().unwrap_or(self.system_mode == ThemeMode::Dark)
        } else {
            self.snapshot.theme_dark
        };
        let dark = self.dark;
        self.accent = self
            .snapshot
            .theme_sync
            .then(|| system_accent(dark))
            .flatten();
        self.system_colors = self
            .snapshot
            .system_colors
            .then(|| system_palette(dark))
            .flatten();
    }

    fn sync(&mut self) {
        let mut state = self.shared.lock().unwrap();
        let mode = state.input_mode();
        let mut changed = false;
        if state.stealth && !crate::backend::supports_hide(mode) {
            state.stealth = false;
            changed = true;
        }
        if state.auto_reconnect && !crate::backend::supports_capture(mode) {
            state.auto_reconnect = false;
            changed = true;
        }
        if state.fps_capper && !crate::backend::cpu_quota_supported(mode) {
            state.fps_capper = false;
            changed = true;
        }
        if changed {
            state.save();
        }
        self.snapshot = state.clone();
        drop(state);

        self.mode = mode;
        self.caps = Caps::of(mode);
    }

    fn mutate(&mut self, f: impl FnOnce(&mut AppState)) {
        {
            let mut state = self.shared.lock().unwrap();
            f(&mut state);
            state.save();
        }
        self.sync();
    }

    fn toggle_running(&mut self) -> Task<Message> {
        if self.page == Page::Diagnostics {
            self.page = Page::Settings;
            let settings = self.snapshot.clone();
            match crate::backend::preflight(&settings) {
                Ok(()) => {
                    if self.snapshot.runtime_status == RuntimeStatus::Error {
                        let mut state = self.shared.lock().unwrap();
                        state.runtime_status = RuntimeStatus::Stopped;
                        state.error_message = None;
                    }
                }
                Err(error) => set_runtime_error(&self.shared, error),
            }
            self.sync();
            return Task::none();
        }

        if self.snapshot.running {
            {
                let mut state = self.shared.lock().unwrap();
                state.running = false;
                state.manually_stopped = true;
            }
            crate::backend::restore_hidden_windows();
            set_runtime_status(&self.shared, RuntimeStatus::Stopped);
        } else {
            let settings = self.snapshot.clone();
            match crate::backend::preflight(&settings) {
                Ok(()) => {
                    {
                        let mut state = self.shared.lock().unwrap();
                        state.running = true;
                        state.manually_stopped = false;
                    }
                    set_runtime_status(&self.shared, RuntimeStatus::WaitingForSober);
                }
                Err(error) => set_runtime_error(&self.shared, error),
            }
        }
        self.sync();
        Task::none()
    }

    fn apply_custom_minutes(&mut self) {
        let Ok(minutes) = self.custom_minutes.trim().parse::<f64>() else {
            self.custom_minutes = format_minutes(self.snapshot.interval_seq);
            return;
        };
        let minutes = minutes.clamp(0.5, 20.0);
        let seconds = (minutes * 60.0).round() as u64;
        self.interval = Interval::from_seconds(seconds);
        if self.interval == Interval::Custom {
            self.custom_minutes = format_minutes(seconds);
        }
        self.mutate(|state| state.interval_seq = seconds);
    }

    fn refresh_diagnostics(&mut self) {
        self.desktop_status = crate::check_desktop_integration();
        let settings = { self.shared.lock().unwrap().clone() };
        let rows = build_diagnostics(&settings, &self.version, &self.desktop_status);
        self.diagnostics = rows;
    }

    fn on_tick(&mut self) -> Task<Message> {
        let mut tasks = self.pump_tray();
        self.sync();
        if let Some(tray) = &mut self.tray {
            tray.refresh(self.snapshot.running);
        }
        if tasks.is_empty() {
            Task::none()
        } else {
            tasks.push(Task::none());
            Task::batch(tasks)
        }
    }

    fn on_frame(&mut self) {
        for (index, flag) in FLAGS.iter().enumerate() {
            let target = if flag_value(&self.snapshot, *flag) {
                1.0
            } else {
                0.0
            };
            let current = self.switches[index];
            let next = current + (target - current) * 0.3;
            self.switches[index] = if (target - next).abs() < 0.002 {
                target
            } else {
                next
            };
        }

        let button_target = if self.snapshot.running { 1.0 } else { 0.0 };
        let button_next = self.button_anim + (button_target - self.button_anim) * 0.25;
        self.button_anim = if (button_target - button_next).abs() < 0.002 {
            button_target
        } else {
            button_next
        };
    }

    fn animations_active(&self) -> bool {
        (self.button_anim - if self.snapshot.running { 1.0 } else { 0.0 }).abs() > 0.002
            || FLAGS.iter().enumerate().any(|(index, flag)| {
                let target = if flag_value(&self.snapshot, *flag) {
                    1.0
                } else {
                    0.0
                };
                self.switches[index] != target
            })
    }

    fn switch_el<'a>(&self, flag: Flag, enabled: bool, palette: Palette) -> Element<'a, Message> {
        switch(
            flag_value(&self.snapshot, flag),
            enabled,
            self.switches[flag.index()],
            palette,
            move |value| Message::SetFlag(flag, value),
        )
    }

    fn shutdown(&mut self) {
        {
            let mut state = self.shared.lock().unwrap();
            state.running = false;
        }
        crate::backend::restore_hidden_windows();
    }

    fn pump_tray(&mut self) -> Vec<Task<Message>> {
        let commands = match &self.tray {
            Some(tray) => tray.drain(),
            None => return Vec::new(),
        };

        let mut tasks = Vec::new();
        for command in commands {
            match command {
                TrayCommand::Show => {
                    if !self.window_open {
                        tasks.push(self.show_window());
                    }
                }
                TrayCommand::ShowSober => {
                    if let Err(error) = crate::backend::show_sober(self.mode) {
                        eprintln!("Cannot show Sober: {error}");
                    }
                }
                TrayCommand::HideSober => {
                    if let Err(error) = crate::backend::hide_sober(self.mode) {
                        eprintln!("Cannot hide Sober: {error}");
                    }
                }
                TrayCommand::Quit => {
                    self.shutdown();
                    tasks.push(iced::exit());
                }
            }
        }
        tasks
    }

    fn toggle_diagnostics(&mut self) {
        self.page = match self.page {
            Page::Settings => {
                self.refresh_diagnostics();
                Page::Diagnostics
            }
            Page::Diagnostics => Page::Settings,
        };
    }
}

fn window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(452.0, 604.0),
        min_size: Some(Size::new(380.0, 460.0)),
        resizable: true,
        exit_on_close_request: false,
        ..window::Settings::default()
    }
}

pub fn run(shared: SharedState, start_hidden: bool) -> iced::Result {
    let boot = {
        let shared = shared.clone();
        move || App::new(shared.clone(), start_hidden)
    };

    iced::daemon::daemon(boot, update, view)
        .title(APP_TITLE)
        .theme(|app: &App, _window: window::Id| app.theme())
        .style(|_app: &App, theme: &Theme| iced::theme::Style {
            background_color: theme.palette().background,
            text_color: theme.palette().text,
        })
        .subscription(subscription)
        .run()
}

fn subscription(app: &App) -> Subscription<Message> {
    let mut subscriptions = vec![
        time::every(TICK).map(|_| Message::Tick),
        iced::system::theme_changes().map(Message::ThemeModeChanged),
        window::close_requests().map(Message::CloseRequested),
    ];
    if app.animations_active() {
        subscriptions.push(time::every(FRAME).map(|_| Message::Frame));
    }
    Subscription::batch(subscriptions)
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Noop => Task::none(),
        Message::Tick => app.on_tick(),
        Message::Frame => {
            app.on_frame();
            Task::none()
        }
        Message::ThemeModeChanged(mode) => {
            app.system_mode = mode;
            app.refresh_theme();
            Task::none()
        }
        Message::WindowOpened(id) => {
            app.window = Some(id);
            app.window_open = true;
            Task::none()
        }
        Message::CloseRequested(id) => {
            app.window_open = false;
            window::close(id)
        }
        Message::DismissWelcome => {
            app.show_welcome = false;
            app.mutate(|state| state.shown_warning = true);
            Task::none()
        }
        Message::DismissGnomeModal => {
            app.show_gnome_relogin_modal = false;
            Task::none()
        }
        Message::ToggleRunning => app.toggle_running(),
        Message::OpenDiagnostics => {
            app.page = Page::Diagnostics;
            app.refresh_diagnostics();
            Task::none()
        }
        Message::ToggleDiagnostics => {
            app.toggle_diagnostics();
            Task::none()
        }
        Message::OpenUrl(url) => {
            let _ = Command::new("xdg-open").arg(url).spawn();
            Task::none()
        }
        Message::SelectInterval(value) => {
            app.interval = value;
            if let Some(seconds) = value.seconds() {
                app.mutate(|state| state.interval_seq = seconds);
            }
            Task::none()
        }
        Message::CustomMinutesChanged(value) => {
            app.custom_minutes = value;
            Task::none()
        }
        Message::ApplyCustomMinutes => {
            app.apply_custom_minutes();
            Task::none()
        }
        Message::SelectAction(value) => {
            app.action = value;
            app.mutate(|state| value.apply(state));
            Task::none()
        }
        Message::SetFlag(flag, value) => {
            app.mutate(|state| flag.apply(state, value));
            if matches!(flag, Flag::ThemeSync | Flag::ThemeDark | Flag::SystemColors) {
                app.refresh_theme();
            }
            Task::none()
        }
        Message::FpsLimitChanged(value) => {
            app.fps_limit = value;
            if let Ok(limit) = app.fps_limit.trim().parse::<u32>()
                && (3..=99).contains(&limit)
            {
                app.mutate(|state| state.fps_limit = limit);
            }
            Task::none()
        }
        Message::RefreshDiagnostics => {
            app.refresh_diagnostics();
            Task::none()
        }
        Message::CheckVersion => {
            if app.version.checking {
                return Task::none();
            }
            app.version.checking = true;
            app.version.result = None;
            app.refresh_diagnostics();
            Task::perform(check_latest_version(), Message::VersionChecked)
        }
        Message::VersionChecked(result) => {
            app.version.checking = false;
            app.version.result = Some(result);
            app.refresh_diagnostics();
            Task::none()
        }
        Message::RunUinputCommand(install) => {
            let command = if install {
                format!(
                    "echo 'KERNEL==\"uinput\", MODE=\"0666\"' > {UINPUT_RULE} && udevadm control --reload-rules && udevadm trigger"
                )
            } else {
                format!("rm -f {UINPUT_RULE} && udevadm control --reload-rules && udevadm trigger")
            };
            Task::perform(run_uinput_command(command), |_| {
                Message::UinputCommandFinished
            })
        }
        Message::UinputCommandFinished => {
            app.refresh_diagnostics();
            Task::none()
        }
        Message::GnomeHelperInstalled(result) => {
            match result {
                Ok(_) => {
                    app.show_gnome_relogin_modal = true;
                }
                Err(error) => {
                    set_runtime_error(&app.shared, error);
                }
            }
            app.sync();
            app.refresh_diagnostics();
            Task::none()
        }
        Message::RunGnomeQuickSetup => Task::perform(run_gnome_quick_setup(), |result| {
            Message::GnomeHelperInstalled(result)
        }),
        Message::EnableKWinLogging => {
            match crate::inputs::kde::enable_script_logging() {
                Ok(()) => set_runtime_error(
                    &app.shared,
                    "KWin logging enabled. Log out and back in, then refresh diagnostics."
                        .to_string(),
                ),
                Err(error) => set_runtime_error(&app.shared, error),
            }
            app.sync();
            app.refresh_diagnostics();
            Task::none()
        }
        Message::CreateNiriWorkspace => {
            match add_niri_hidden_workspace() {
                Ok(()) => {
                    let _ = Command::new("niri")
                        .args(["msg", "action", "load-config-file"])
                        .output();
                    app.sync();
                    app.refresh_diagnostics();
                }
                Err(err) => {
                    set_runtime_error(&app.shared, err);
                    app.sync();
                    app.refresh_diagnostics();
                }
            }
            Task::none()
        }
        Message::RemoveNiriWorkspace => {
            match remove_niri_hidden_workspace() {
                Ok(()) => {
                    let _ = Command::new("niri")
                        .args(["msg", "action", "load-config-file"])
                        .output();
                    app.sync();
                    app.refresh_diagnostics();
                }
                Err(err) => {
                    set_runtime_error(&app.shared, err);
                    app.sync();
                    app.refresh_diagnostics();
                }
            }
            Task::none()
        }
        Message::InstallDesktopEntry => Task::perform(
            async { crate::install_desktop_entry() },
            Message::DesktopEntryInstalled,
        ),
        Message::DesktopEntryInstalled(result) => {
            app.show_update_installed_modal = false;
            match result {
                Ok(()) => {
                    app.desktop_status = crate::check_desktop_integration();
                }
                Err(error) => {
                    set_runtime_error(&app.shared, error);
                }
            }
            app.sync();
            app.refresh_diagnostics();
            Task::none()
        }
        Message::DismissUpdateInstalledModal => {
            app.show_update_installed_modal = false;
            Task::none()
        }
        Message::DismissAurUpdateModal => {
            app.show_aur_update_modal = false;
            Task::none()
        }
        Message::ShowAurUpdateModal => {
            app.show_aur_update_modal = true;
            Task::none()
        }
        Message::UninstallDesktopEntry => Task::perform(
            async { crate::uninstall_desktop_entry() },
            Message::DesktopEntryUninstalled,
        ),
        Message::DesktopEntryUninstalled(result) => {
            match result {
                Ok(()) => {
                    app.desktop_status = crate::check_desktop_integration();
                    app.dismissed_install_banner = false;
                }
                Err(error) => {
                    set_runtime_error(&app.shared, error);
                }
            }
            app.sync();
            app.refresh_diagnostics();
            Task::none()
        }
        Message::ShowPurgeModal => {
            app.show_purge_modal = true;
            Task::none()
        }
        Message::DismissPurgeModal => {
            app.show_purge_modal = false;
            Task::none()
        }
        Message::ConfirmPurgeAndExit => {
            let _ = crate::purge_all_data();
            std::process::exit(0);
        }
        Message::ShowAurUninstallModal => {
            app.show_aur_uninstall_modal = true;
            Task::none()
        }
        Message::DismissAurUninstallModal => {
            app.show_aur_uninstall_modal = false;
            Task::none()
        }
        Message::DismissInstallBanner => {
            app.dismissed_install_banner = true;
            Task::none()
        }
        Message::DismissAurUpdateBanner => {
            app.dismissed_aur_update_banner = true;
            Task::none()
        }
    }
}

fn view(app: &App, _window: window::Id) -> Element<'_, Message> {
    let theme = app.theme();
    let mut palette = Palette::of(&theme);
    if let Some(system) = app.system_colors {
        palette.card = system.card;
        palette.strip = mix(theme.palette().background, system.card, 0.5);
    }

    if app.show_welcome {
        return container(welcome_page(palette))
            .width(Fill)
            .height(Fill)
            .padding(24.0)
            .into();
    }

    let page = match app.page {
        Page::Settings => settings_page(app, palette),
        Page::Diagnostics => diagnostics_page(app, palette),
    };

    let mut content = column![
        header(palette),
        divider(palette),
        scrollable(container(page).width(Fill).padding(Padding {
            top: 16.0,
            bottom: 16.0,
            left: 16.0,
            right: 16.0,
        }))
        .width(Fill)
        .height(Fill),
    ];

    if let Some(strip) = status_strip(app, palette) {
        content = content.push(strip);
    }

    content = content.push(bottom_bar(app, palette));

    let main_view = container(content.width(Fill).height(Fill))
        .width(Fill)
        .height(Fill);

    let mut stacked = stack![main_view];
    if app.show_update_installed_modal {
        stacked = stacked.push(update_installed_modal(palette));
    }
    if app.show_gnome_relogin_modal {
        stacked = stacked.push(gnome_relogin_modal(palette));
    }
    if app.show_aur_update_modal
        && let DesktopIntegrationStatus::Aur { package, helper } = &app.desktop_status
    {
        let latest_str = match &app.version.result {
            Some(Ok((false, latest))) => latest.as_str(),
            _ => "",
        };
        stacked = stacked.push(aur_update_modal(package, helper, latest_str, palette));
    }
    if app.show_aur_uninstall_modal
        && let DesktopIntegrationStatus::Aur { package, helper } = &app.desktop_status
    {
        stacked = stacked.push(aur_uninstall_modal(package, helper, palette));
    }
    if app.show_purge_modal {
        let (is_aur, aur_pkg, aur_helper) = match &app.desktop_status {
            DesktopIntegrationStatus::Aur { package, helper } => {
                (true, Some(package.as_str()), Some(helper.as_str()))
            }
            _ => (false, None, None),
        };
        stacked = stacked.push(purge_modal(is_aur, aur_pkg, aur_helper, palette));
    }

    stacked.width(Fill).height(Fill).into()
}

fn status_strip(app: &App, palette: Palette) -> Option<Element<'_, Message>> {
    let (status, color) = status_text(app, palette);
    if status.is_empty() {
        return None;
    }

    Some(
        container(text(status).size(11.5).color(color))
            .width(Fill)
            .height(28.0)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(palette.strip)),
                border: Border::default(),
                ..container::Style::default()
            })
            .into(),
    )
}

fn tip<'a>(
    content: impl Into<Element<'a, Message>>,
    label: impl Into<Element<'a, Message>>,
    position: TooltipPosition,
    palette: Palette,
) -> Element<'a, Message> {
    tooltip(content, label, position)
        .gap(6.0)
        .padding(6.0)
        .style(move |_theme| tooltip_surface(palette))
        .into()
}

fn tooltip_surface(palette: Palette) -> container::Style {
    container::Style {
        text_color: Some(palette.fg),
        background: Some(Background::Color(palette.card)),
        border: Border {
            width: 1.0,
            color: palette.border,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

fn title_row<'a>(size: f32, palette: Palette) -> Row<'a, Message> {
    let (head, tail) = APP_TITLE.split_once("RBX").unwrap_or((APP_TITLE, ""));
    row![
        text(head.to_string()).size(size).font(BOLD),
        text("RBX").size(size).font(BOLD).color(RBX_RED),
        text(tail.to_string())
            .size(size)
            .font(BOLD)
            .color(palette.accent),
    ]
    .spacing(0.0)
    .align_y(Vertical::Center)
}

fn header(palette: Palette) -> Element<'static, Message> {
    let title = title_row(14.0, palette);

    let buttons = row![
        tip(
            icon_button(Icon::Heart, palette, Message::OpenUrl(DONATE_URL)),
            text("Donate").size(11.0),
            TooltipPosition::Bottom,
            palette,
        ),
        tip(
            icon_button(Icon::Chat, palette, Message::OpenUrl(DISCORD_URL)),
            text("Discord").size(11.0),
            TooltipPosition::Bottom,
            palette,
        ),
        tip(
            icon_button(Icon::Repository, palette, Message::OpenUrl(REPOSITORY_URL),),
            text("Open GitHub repository").size(11.0),
            TooltipPosition::Bottom,
            palette,
        ),
        tip(
            icon_button(Icon::Settings, palette, Message::ToggleDiagnostics),
            text("Diagnostics").size(11.0),
            TooltipPosition::Bottom,
            palette,
        ),
    ]
    .spacing(4.0)
    .align_y(Vertical::Center);

    let content = row![title, Space::new().width(Fill), buttons]
        .align_y(Vertical::Center)
        .width(Fill);

    container(content)
        .width(Fill)
        .padding(Padding {
            top: 7.0,
            bottom: 7.0,
            left: 16.0,
            right: 14.0,
        })
        .into()
}

fn welcome_page(palette: Palette) -> Element<'static, Message> {
    let title = title_row(24.0, palette);

    let note = container(
        column![
            text("Desktop Environment Notice")
                .size(13.5)
                .font(SEMIBOLD)
                .align_x(Horizontal::Center)
                .width(Fill),
            text(
                "Linux desktop environments (Hyprland, KDE, GNOME, NIRI, COSMIC, X11, i3) handle inputs and window management differently. Some features might not behave identically across all systems. Please check how the app works on your desktop in Diagnostics, and report any bugs or suggestions on GitHub or Discord."
            )
            .size(12.0)
            .color(palette.muted)
            .align_x(Horizontal::Center)
            .width(Fill),
        ]
        .spacing(8.0)
        .align_x(Horizontal::Center)
        .width(Fill),
    )
    .padding(Padding {
        top: 14.0,
        bottom: 14.0,
        left: 14.0,
        right: 14.0,
    })
    .width(Fill)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(palette.surface)),
        border: Border {
            color: palette.border,
            width: 1.0,
            radius: 12.0.into(),
        },
        ..container::Style::default()
    });

    let links = row![
        button(
            row![
                icon(Icon::Repository, 14.0, palette.fg),
                text("Report on GitHub").size(12.0).font(SEMIBOLD),
            ]
            .spacing(6.0)
            .align_y(Vertical::Center),
        )
        .on_press(Message::OpenUrl(REPOSITORY_URL))
        .style(styled(ButtonKind::Subtle, palette)),
        button(
            row![
                icon(Icon::Chat, 14.0, palette.fg),
                text("Discord Community").size(12.0).font(SEMIBOLD),
            ]
            .spacing(6.0)
            .align_y(Vertical::Center),
        )
        .on_press(Message::OpenUrl(DISCORD_URL))
        .style(styled(ButtonKind::Subtle, palette)),
    ]
    .spacing(10.0)
    .align_y(Vertical::Center);

    let content = column![
        Space::new().height(Fill),
        icon(Icon::Warning, 48.0, palette.warning),
        title,
        text("Sober Edition")
            .size(14.0)
            .font(BOLD)
            .color(palette.accent),
        Space::new().height(14.0),
        note,
        Space::new().height(10.0),
        links,
        Space::new().height(Fill),
        button(
            container(text("Get Started").size(14.0).font(SEMIBOLD))
                .width(Fill)
                .height(Fill)
                .center_x(Fill)
                .center_y(Fill),
        )
        .width(Fill)
        .height(42.0)
        .padding(0.0)
        .on_press(Message::DismissWelcome)
        .style(styled(ButtonKind::Primary, palette)),
    ]
    .spacing(6.0)
    .align_x(Horizontal::Center)
    .width(Fill)
    .height(Fill);

    content.into()
}

fn settings_page(app: &App, palette: Palette) -> Element<'_, Message> {
    let locked = app.snapshot.running || app.snapshot.action_active;
    let allow = |supported: bool| supported && !locked;
    let mode = app.mode;

    let interval = pick_list(&INTERVALS[..], Some(app.interval), move |value| {
        if locked {
            Message::Noop
        } else {
            Message::SelectInterval(value)
        }
    })
    .text_size(13.0)
    .padding(Padding {
        top: 6.0,
        bottom: 6.0,
        left: 10.0,
        right: 8.0,
    })
    .width(Length::Fixed(112.0))
    .style(select_style(palette));

    let action = pick_list(&ACTIONS[..], Some(app.action), move |value| {
        if locked {
            Message::Noop
        } else {
            Message::SelectAction(value)
        }
    })
    .text_size(13.0)
    .padding(Padding {
        top: 6.0,
        bottom: 6.0,
        left: 10.0,
        right: 8.0,
    })
    .width(Length::Fixed(104.0))
    .style(select_style(palette));

    let mut rows: Vec<Element<'_, Message>> = Vec::new();

    rows.push(settings_row(
        palette,
        Icon::Interval,
        "Interval",
        None,
        interval.into(),
        None,
    ));

    if app.interval == Interval::Custom {
        let custom = row![
            text_input("minutes", app.custom_minutes.as_str())
                .on_input_maybe((!locked).then_some(Message::CustomMinutesChanged))
                .on_submit(Message::ApplyCustomMinutes)
                .size(13.0)
                .padding(Padding {
                    top: 6.0,
                    bottom: 6.0,
                    left: 8.0,
                    right: 8.0,
                })
                .width(Length::Fixed(68.0))
                .style(input_style(palette)),
            text("min").size(12.0).color(palette.muted),
            button(text("Apply").size(11.0))
                .padding(Padding {
                    top: 5.0,
                    bottom: 5.0,
                    left: 10.0,
                    right: 10.0,
                })
                .style(styled(ButtonKind::Subtle, palette))
                .on_press_maybe((!locked).then_some(Message::ApplyCustomMinutes)),
        ]
        .spacing(6.0)
        .align_y(Vertical::Center);

        rows.push(settings_sub_row(
            palette,
            Icon::Interval,
            "Custom interval",
            None,
            custom.into(),
            None,
        ));
    }

    rows.push(settings_row(
        palette,
        Icon::Action,
        "Idle action",
        None,
        action.into(),
        (mode == InputMode::Niri).then(|| {
            InfoTip::plain(
                "Niri does not report cursor position. The cursor will restore to the previously focused window instead of the exact coordinate.",
            )
        }),
    ));

    rows.push(settings_row(
        palette,
        Icon::Multi,
        "Multi-Instance",
        None,
        app.switch_el(Flag::MultiInstance, allow(app.caps.enumeration), palette),
        (!app.caps.enumeration)
            .then(|| InfoTip::plain("Window management is unavailable on this desktop.")),
    ));

    rows.push(settings_row(
        palette,
        Icon::Autostart,
        "Auto-Start",
        None,
        app.switch_el(Flag::AutoStart, !locked, palette),
        None,
    ));

    rows.push(settings_row(
        palette,
        Icon::Mouse,
        "Don't Interrupt Me",
        None,
        app.switch_el(Flag::UserSafe, !locked, palette),
        Some(InfoTip::plain(
            "Pause when mouse or keyboard activity is detected.",
        )),
    ));

    let hide_info = if !app.caps.hide {
        Some(InfoTip::plain(
            "Hide Game is not available on this desktop.",
        ))
    } else if app.mode == InputMode::Hyprland {
        Some(InfoTip {
            icon: Icon::Beta,
            text: BETA_NOTE,
        })
    } else {
        None
    };

    rows.push(settings_row(
        palette,
        Icon::Hide,
        "Hide Game",
        None,
        app.switch_el(Flag::Stealth, allow(app.caps.hide), palette),
        hide_info,
    ));

    let reconnect_tip = if !app.caps.capture {
        Some(InfoTip::plain(
            "Screen capture is required for Auto Reconnect and is not available here.",
        ))
    } else {
        Some(InfoTip {
            icon: Icon::Beta,
            text: "Auto Reconnect is currently in beta: in some cases, the reconnect element may not be detected.",
        })
    };

    rows.push(settings_row(
        palette,
        Icon::Reconnect,
        "Auto Reconnect",
        None,
        app.switch_el(Flag::AutoReconnect, allow(app.caps.capture), palette),
        reconnect_tip,
    ));

    let quota_info = if mode == InputMode::Cosmic {
        Some(InfoTip::plain(
            "FPS Capper is disabled on COSMIC because process pausing causes Roblox to crash.",
        ))
    } else if app.caps.quota {
        Some(InfoTip {
            icon: Icon::Beta,
            text: BETA_NOTE,
        })
    } else {
        None
    };

    rows.push(settings_row(
        palette,
        Icon::Performance,
        "FPS Capper",
        None,
        app.switch_el(Flag::FpsCapper, allow(app.caps.quota), palette),
        quota_info,
    ));

    if app.snapshot.fps_capper {
        rows.push(settings_sub_row(
            palette,
            Icon::Cpu,
            "CPU Quota",
            None,
            row![
                text_input("30", app.fps_limit.as_str())
                    .on_input_maybe((!locked).then_some(Message::FpsLimitChanged))
                    .size(13.0)
                    .padding(Padding {
                        top: 6.0,
                        bottom: 6.0,
                        left: 8.0,
                        right: 8.0,
                    })
                    .width(Length::Fixed(62.0))
                    .style(input_style(palette)),
                text("%").size(12.0).color(palette.muted),
            ]
            .spacing(6.0)
            .align_y(Vertical::Center)
            .into(),
            None,
        ));

        rows.push(settings_sub_row(
            palette,
            Icon::Focus,
            "Unlock at Focus",
            None,
            app.switch_el(Flag::StopLimitOnFocus, !locked, palette),
            Some(InfoTip::plain(
                "Removes the limit while Sober is focused. If the focused window cannot be determined, the limit stays off so the game never gets slowed down.",
            )),
        ));
    }

    let mut list = column![].width(Fill).spacing(0.0);
    for (index, item) in rows.into_iter().enumerate() {
        if index > 0 {
            list = list.push(divider(palette));
        }
        list = list.push(item);
    }

    let settings_card = card(list, palette);

    let preview_banner = match mode {
        InputMode::Cosmic | InputMode::Niri | InputMode::I3 | InputMode::X11 | InputMode::Gnome => {
            Some(
                container(
                    row![
                        icon(Icon::Beta, 13.0, palette.muted),
                        text(format!(
                            "Early support for {}: some features may be limited.",
                            mode.label()
                        ))
                        .size(11.5)
                        .color(palette.muted),
                    ]
                    .spacing(8.0)
                    .align_y(Vertical::Center),
                )
                .width(Fill)
                .padding(Padding {
                    top: 6.0,
                    bottom: 6.0,
                    left: 10.0,
                    right: 10.0,
                })
                .style(move |_theme| container::Style {
                    background: Some(Background::Color(palette.card)),
                    border: Border {
                        color: palette.hairline,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..container::Style::default()
                }),
            )
        }
        _ => None,
    };

    let install_banner = if !app.dismissed_install_banner
        && matches!(app.desktop_status, DesktopIntegrationStatus::NotInstalled)
    {
        Some(
            container(
                row![
                    icon(Icon::Layers, 16.0, palette.accent),
                    column![
                        text("Desktop Integration").size(12.5).font(SEMIBOLD),
                        text("Add to applications menu and desktop launcher")
                            .size(11.0)
                            .color(palette.muted),
                    ]
                    .spacing(2.0)
                    .width(Fill),
                    button(text("Add to System").size(11.0).font(SEMIBOLD))
                        .padding(Padding {
                            top: 5.0,
                            bottom: 5.0,
                            left: 12.0,
                            right: 12.0,
                        })
                        .style(styled(ButtonKind::Primary, palette))
                        .on_press(Message::InstallDesktopEntry),
                    tip(
                        icon_button(Icon::Close, palette, Message::DismissInstallBanner),
                        text("Dismiss").size(11.0),
                        TooltipPosition::Bottom,
                        palette,
                    ),
                ]
                .spacing(8.0)
                .align_y(Vertical::Center)
                .width(Fill),
            )
            .width(Fill)
            .padding(10.0)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(palette.card)),
                border: Border {
                    color: palette.hairline,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..container::Style::default()
            }),
        )
    } else {
        None
    };

    let aur_update_banner = if !app.dismissed_aur_update_banner {
        match (&app.desktop_status, &app.version.result) {
            (DesktopIntegrationStatus::Aur { package, helper }, Some(Ok((false, latest)))) => Some(
                container(
                    row![
                        icon(Icon::Info, 16.0, palette.accent),
                        column![
                            text(format!("Update v{latest} available (AUR)"))
                                .size(12.5)
                                .font(SEMIBOLD),
                            text(format!("Run '{helper} -S {package}' in terminal"))
                                .size(11.0)
                                .color(palette.muted),
                        ]
                        .spacing(2.0)
                        .width(Fill),
                        button(text("How to Update").size(11.0))
                            .padding(Padding {
                                top: 5.0,
                                bottom: 5.0,
                                left: 10.0,
                                right: 10.0,
                            })
                            .style(styled(ButtonKind::Subtle, palette))
                            .on_press(Message::ShowAurUpdateModal),
                        tip(
                            icon_button(Icon::Close, palette, Message::DismissAurUpdateBanner),
                            text("Dismiss").size(11.0),
                            TooltipPosition::Bottom,
                            palette,
                        ),
                    ]
                    .spacing(8.0)
                    .align_y(Vertical::Center)
                    .width(Fill),
                )
                .width(Fill)
                .padding(10.0)
                .style(move |_theme| container::Style {
                    background: Some(Background::Color(palette.card)),
                    border: Border {
                        color: palette.hairline,
                        width: 1.0,
                        radius: 10.0.into(),
                    },
                    ..container::Style::default()
                }),
            ),
            _ => None,
        }
    } else {
        None
    };

    let mut page_col = column![].width(Fill).spacing(10.0);
    if let Some(banner) = preview_banner {
        page_col = page_col.push(banner);
    }
    if let Some(banner) = aur_update_banner {
        page_col = page_col.push(banner);
    }
    if let Some(banner) = install_banner {
        page_col = page_col.push(banner);
    }
    page_col = page_col.push(settings_card);
    page_col.into()
}

fn diagnostics_page(app: &App, palette: Palette) -> Element<'_, Message> {
    let has_errors = app
        .diagnostics
        .iter()
        .any(|row| row.status == DiagStatus::Error);
    let (overall, overall_color) = if has_errors {
        ("Action Required", palette.danger)
    } else {
        ("Ready", palette.success)
    };

    let title = container(
        row![
            icon_tile(
                if has_errors {
                    Icon::Warning
                } else {
                    Icon::Info
                },
                palette
            ),
            text("Diagnostics").size(13.5).font(SEMIBOLD),
            Space::new().width(Fill),
            text(overall).size(11.5).color(overall_color),
            button(text("Refresh").size(11.0))
                .padding(Padding {
                    top: 4.0,
                    bottom: 4.0,
                    left: 10.0,
                    right: 10.0,
                })
                .style(styled(ButtonKind::Subtle, palette))
                .on_press(Message::RefreshDiagnostics),
        ]
        .spacing(10.0)
        .width(Fill)
        .align_y(Vertical::Center),
    )
    .width(Fill)
    .padding(Padding {
        top: 6.0,
        bottom: 6.0,
        left: 8.0,
        right: 8.0,
    })
    .style(move |_theme| container::Style {
        background: Some(Background::Color(palette.card)),
        border: Border {
            color: palette.hairline,
            width: 1.0,
            radius: 14.0.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.10),
            blur_radius: 12.0,
            ..Shadow::default()
        },
        ..container::Style::default()
    });

    let mut rows: Vec<Element<'_, Message>> = Vec::new();
    for entry in &app.diagnostics {
        rows.push(diagnostics_row(entry, palette));
    }

    let mut list = column![].width(Fill).spacing(0.0);
    for (index, item) in rows.into_iter().enumerate() {
        if index > 0 {
            list = list.push(divider(palette));
        }
        list = list.push(item);
    }

    let appearance = card(
        column![
            settings_row(
                palette,
                Icon::Palette,
                "Sync Colors With System",
                None,
                app.switch_el(Flag::ThemeSync, true, palette),
                Some(InfoTip::plain(
                    "Takes the light/dark scheme and the accent colour from the GTK theme of the desktop.",
                )),
            ),
            divider(palette),
            settings_row(
                palette,
                Icon::Layers,
                "Sync Background & Elements",
                None,
                app.switch_el(Flag::SystemColors, true, palette),
                Some(InfoTip::plain(
                    "Also takes the window background, cards and status colours from the GTK theme.",
                )),
            ),
            divider(palette),
            settings_row(
                palette,
                Icon::Moon,
                "Theme",
                None,
                row![
                    text(if app.snapshot.theme_dark {
                        "Dark"
                    } else {
                        "Light"
                    })
                    .size(11.5)
                    .color(palette.muted),
                    app.switch_el(Flag::ThemeDark, !app.snapshot.theme_sync, palette),
                ]
                .spacing(8.0)
                .align_y(Vertical::Center)
                .into(),
                app.snapshot
                    .theme_sync
                    .then(|| InfoTip::plain(
                        "Turn off Sync Colors With System to choose the theme manually.",
                    )),
            ),
        ]
        .width(Fill),
        palette,
    );

    let desktop_subtitle = match &app.desktop_status {
        DesktopIntegrationStatus::Aur { .. } => Some("Managed by system package manager (AUR)"),
        DesktopIntegrationStatus::Installed { .. } => {
            Some("Installed in ~/.local/share/applications and ~/.local/bin")
        }
        DesktopIntegrationStatus::NotInstalled => Some("Not currently added to applications menu"),
    };

    let cleanup_card = card(
        column![
            settings_row(
                palette,
                Icon::Layers,
                "Desktop Integration",
                desktop_subtitle,
                match &app.desktop_status {
                    DesktopIntegrationStatus::Aur { .. } => {
                        button(text("How to Uninstall").size(11.0))
                            .padding(Padding {
                                top: 4.0,
                                bottom: 4.0,
                                left: 10.0,
                                right: 10.0,
                            })
                            .style(styled(ButtonKind::Subtle, palette))
                            .on_press(Message::ShowAurUninstallModal)
                            .into()
                    }
                    DesktopIntegrationStatus::Installed { .. } => {
                        row![
                            button(text("Reinstall").size(11.0))
                                .padding(Padding {
                                    top: 4.0,
                                    bottom: 4.0,
                                    left: 10.0,
                                    right: 10.0,
                                })
                                .style(styled(ButtonKind::Subtle, palette))
                                .on_press(Message::InstallDesktopEntry),
                            button(text("Remove").size(11.0))
                                .padding(Padding {
                                    top: 4.0,
                                    bottom: 4.0,
                                    left: 10.0,
                                    right: 10.0,
                                })
                                .style(styled(ButtonKind::Subtle, palette))
                                .on_press(Message::UninstallDesktopEntry),
                        ]
                        .spacing(6.0)
                        .align_y(Vertical::Center)
                        .into()
                    }
                    DesktopIntegrationStatus::NotInstalled => {
                        button(text("Add to System").size(11.0))
                            .padding(Padding {
                                top: 4.0,
                                bottom: 4.0,
                                left: 10.0,
                                right: 10.0,
                            })
                            .style(styled(ButtonKind::Primary, palette))
                            .on_press(Message::InstallDesktopEntry)
                            .into()
                    }
                },
                None,
            ),
            divider(palette),
            settings_row(
                palette,
                Icon::Warning,
                "Purge All Data & Traces",
                Some(
                    "Deletes all config files (~/.config), desktop launchers, and exits application",
                ),
                button(text("Purge & Exit").size(11.0).color(palette.danger))
                    .padding(Padding {
                        top: 4.0,
                        bottom: 4.0,
                        left: 10.0,
                        right: 10.0,
                    })
                    .style(styled(ButtonKind::Subtle, palette))
                    .on_press(Message::ShowPurgeModal)
                    .into(),
                None,
            ),
        ]
        .width(Fill),
        palette,
    );

    column![
        title,
        Space::new().height(10.0),
        card(list, palette),
        Space::new().height(10.0),
        appearance,
        Space::new().height(10.0),
        cleanup_card,
    ]
    .width(Fill)
    .into()
}

fn gnome_relogin_modal(palette: Palette) -> Element<'static, Message> {
    let dialog = container(
        column![
            row![
                icon(Icon::Layers, 18.0, palette.accent),
                text("Session Relogin Required").size(14.0).font(BOLD),
            ]
            .spacing(10.0)
            .align_y(Vertical::Center),
            text(
                "The GNOME Shell extension has been installed and enabled.\n\nOn Wayland, GNOME Shell does not load newly installed extensions until the session restarts.\n\nPlease log out and log back in to your GNOME session to activate the extension."
            )
            .size(12.0)
            .color(palette.muted),
            button(
                container(text("OK, got it").size(12.5).font(SEMIBOLD))
                    .width(Fill)
                    .align_x(Horizontal::Center),
            )
            .padding(Padding {
                top: 8.0,
                bottom: 8.0,
                left: 16.0,
                right: 16.0,
            })
            .width(Fill)
            .style(styled(ButtonKind::Primary, palette))
            .on_press(Message::DismissGnomeModal),
        ]
        .spacing(14.0)
        .width(340.0),
    )
    .padding(20.0)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(palette.card)),
        border: Border {
            color: palette.hairline,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.40),
            blur_radius: 24.0,
            ..Shadow::default()
        },
        ..container::Style::default()
    });

    container(dialog)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(alpha(Color::BLACK, 0.55))),
            ..container::Style::default()
        })
        .into()
}

fn update_installed_modal(palette: Palette) -> Element<'static, Message> {
    let dialog = container(
        column![
            row![
                icon(Icon::Layers, 18.0, palette.accent),
                text("Update Installed Version?").size(14.0).font(BOLD),
            ]
            .spacing(10.0)
            .align_y(Vertical::Center),
            text(
                "A different version is currently installed in ~/.local/bin/antiafk-rbx-sober.\n\nWould you like to replace the installed version with this current build and keep your desktop launcher updated?"
            )
            .size(12.0)
            .color(palette.muted),
            row![
                button(
                    container(text("Not Now").size(12.0))
                        .width(Fill)
                        .align_x(Horizontal::Center)
                )
                .padding(Padding {
                    top: 8.0,
                    bottom: 8.0,
                    left: 14.0,
                    right: 14.0,
                })
                .width(Length::Fixed(100.0))
                .style(styled(ButtonKind::Subtle, palette))
                .on_press(Message::DismissUpdateInstalledModal),
                button(
                    container(text("Replace Installed").size(12.0).font(SEMIBOLD))
                        .width(Fill)
                        .align_x(Horizontal::Center)
                )
                .padding(Padding {
                    top: 8.0,
                    bottom: 8.0,
                    left: 14.0,
                    right: 14.0,
                })
                .width(Fill)
                .style(styled(ButtonKind::Primary, palette))
                .on_press(Message::InstallDesktopEntry),
            ]
            .spacing(10.0)
            .width(Fill),
        ]
        .spacing(14.0)
        .width(360.0),
    )
    .padding(20.0)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(palette.card)),
        border: Border {
            color: palette.hairline,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.40),
            blur_radius: 24.0,
            ..Shadow::default()
        },
        ..container::Style::default()
    });

    container(dialog)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(alpha(Color::BLACK, 0.55))),
            ..container::Style::default()
        })
        .into()
}

fn aur_update_modal(
    package: &str,
    helper: &str,
    latest_version: &str,
    palette: Palette,
) -> Element<'static, Message> {
    let update_cmd = format!("{helper} -S {package}");
    let version_note = if latest_version.is_empty() {
        "A newer version is available.".to_string()
    } else {
        format!("A newer version (v{latest_version}) is available.")
    };
    let dialog = container(
        column![
            row![
                icon(Icon::Info, 18.0, palette.accent),
                text("Update via AUR").size(14.0).font(BOLD),
            ]
            .spacing(10.0)
            .align_y(Vertical::Center),
            text(format!(
                "{version_note}\n\nSince AntiAFK-RBX-Sober is managed as an AUR package ({package}), update it through your package manager by running:\n\n  {update_cmd}"
            ))
            .size(12.0)
            .color(palette.muted),
            button(
                container(text("Close").size(12.5).font(SEMIBOLD))
                    .width(Fill)
                    .align_x(Horizontal::Center),
            )
            .padding(Padding {
                top: 8.0,
                bottom: 8.0,
                left: 16.0,
                right: 16.0,
            })
            .width(Fill)
            .style(styled(ButtonKind::Primary, palette))
            .on_press(Message::DismissAurUpdateModal),
        ]
        .spacing(14.0)
        .width(360.0),
    )
    .padding(20.0)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(palette.card)),
        border: Border {
            color: palette.hairline,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.40),
            blur_radius: 24.0,
            ..Shadow::default()
        },
        ..container::Style::default()
    });

    container(dialog)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(alpha(Color::BLACK, 0.55))),
            ..container::Style::default()
        })
        .into()
}

fn aur_uninstall_modal(package: &str, helper: &str, palette: Palette) -> Element<'static, Message> {
    let uninstall_cmd = format!("{helper} -R {package}");
    let dialog = container(
        column![
            row![
                icon(Icon::Warning, 18.0, palette.warning),
                text("Uninstall AUR Package").size(14.0).font(BOLD),
            ]
            .spacing(10.0)
            .align_y(Vertical::Center),
            text(format!(
                "AntiAFK-RBX-Sober is installed as an AUR system package ({package}).\n\nTo uninstall the application and remove all system files, run this command in your terminal:\n\n  {uninstall_cmd}"
            ))
            .size(12.0)
            .color(palette.muted),
            button(
                container(text("Close").size(12.5).font(SEMIBOLD))
                    .width(Fill)
                    .align_x(Horizontal::Center),
            )
            .padding(Padding {
                top: 8.0,
                bottom: 8.0,
                left: 16.0,
                right: 16.0,
            })
            .width(Fill)
            .style(styled(ButtonKind::Primary, palette))
            .on_press(Message::DismissAurUninstallModal),
        ]
        .spacing(14.0)
        .width(360.0),
    )
    .padding(20.0)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(palette.card)),
        border: Border {
            color: palette.hairline,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.40),
            blur_radius: 24.0,
            ..Shadow::default()
        },
        ..container::Style::default()
    });

    container(dialog)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(alpha(Color::BLACK, 0.55))),
            ..container::Style::default()
        })
        .into()
}

fn purge_modal(
    is_aur: bool,
    aur_pkg: Option<&str>,
    aur_helper: Option<&str>,
    palette: Palette,
) -> Element<'static, Message> {
    let aur_note = if is_aur {
        let pkg = aur_pkg.unwrap_or("antiafk-rbx-sober");
        let helper = aur_helper.unwrap_or("yay");
        format!(
            "\n\nNote: The application is installed via AUR ({pkg}). To completely remove the application binary from your system, run after exiting:\n  {helper} -R {pkg}"
        )
    } else {
        String::new()
    };

    let message_text = format!(
        "This will permanently delete:\n• All configuration files (~/.config/antiafk-rbx-sober)\n• Desktop launcher and icons (~/.local/share/applications, ~/.local/bin)\n• GNOME Shell helper extension (if present)\n\nThe application will close immediately after cleanup.{aur_note}"
    );

    let dialog = container(
        column![
            row![
                icon(Icon::Warning, 18.0, palette.danger),
                text("Purge All Data & Exit?").size(14.0).font(BOLD),
            ]
            .spacing(10.0)
            .align_y(Vertical::Center),
            text(message_text).size(12.0).color(palette.muted),
            row![
                button(
                    container(text("Cancel").size(12.0))
                        .width(Fill)
                        .align_x(Horizontal::Center)
                )
                .padding(Padding {
                    top: 8.0,
                    bottom: 8.0,
                    left: 14.0,
                    right: 14.0,
                })
                .width(Length::Fixed(100.0))
                .style(styled(ButtonKind::Subtle, palette))
                .on_press(Message::DismissPurgeModal),
                button(
                    container(
                        text("Purge & Exit")
                            .size(12.0)
                            .font(SEMIBOLD)
                            .color(palette.danger),
                    )
                    .width(Fill)
                    .align_x(Horizontal::Center)
                )
                .padding(Padding {
                    top: 8.0,
                    bottom: 8.0,
                    left: 14.0,
                    right: 14.0,
                })
                .width(Fill)
                .style(styled(ButtonKind::Subtle, palette))
                .on_press(Message::ConfirmPurgeAndExit),
            ]
            .spacing(10.0)
            .width(Fill),
        ]
        .spacing(14.0)
        .width(380.0),
    )
    .padding(20.0)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(palette.card)),
        border: Border {
            color: palette.hairline,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.40),
            blur_radius: 24.0,
            ..Shadow::default()
        },
        ..container::Style::default()
    });

    container(dialog)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(alpha(Color::BLACK, 0.55))),
            ..container::Style::default()
        })
        .into()
}

fn diagnostics_row<'a>(entry: &'a DiagRow, palette: Palette) -> Element<'a, Message> {
    let (label, color) = match entry.status {
        DiagStatus::Ok => ("Ready", palette.success),
        DiagStatus::Warning => ("Unavailable", palette.warning),
        DiagStatus::Error => ("Error", palette.danger),
    };
    let label = entry.label.as_deref().unwrap_or(label);
    let color = if entry.label.as_deref() == Some("Waiting") {
        palette.muted
    } else {
        color
    };

    let mut control = row![].spacing(8.0).align_y(Vertical::Center);

    if let Some(value) = entry.inline_detail() {
        control = control.push(text(value).size(11.5).color(palette.muted));
    }

    control = control.push(text(label).size(12.0).color(color));

    if let Some(action) = entry.action {
        let message = match action {
            DiagAction::Fix => Message::RunUinputCommand(true),
            DiagAction::RemoveRule => Message::RunUinputCommand(false),
            DiagAction::CheckVersion => Message::CheckVersion,
            DiagAction::RunGnomeQuickSetup => Message::RunGnomeQuickSetup,
            DiagAction::EnableKWinLogging => Message::EnableKWinLogging,
            DiagAction::CreateNiriWorkspace => Message::CreateNiriWorkspace,
            DiagAction::RemoveNiriWorkspace => Message::RemoveNiriWorkspace,
            DiagAction::InstallDesktop | DiagAction::UpdateDesktop => Message::InstallDesktopEntry,
            DiagAction::UninstallDesktop => Message::UninstallDesktopEntry,
            DiagAction::ShowAurUpdate => Message::ShowAurUpdateModal,
            DiagAction::ShowAurUninstall => Message::ShowAurUninstallModal,
            DiagAction::OpenReleases => Message::OpenUrl(RELEASES_URL),
        };
        let label = match action {
            DiagAction::Fix => "Fix",
            DiagAction::RemoveRule => "Remove Auto-Fix Rule",
            DiagAction::CheckVersion => "Check",
            DiagAction::RunGnomeQuickSetup => "Install & Enable",
            DiagAction::EnableKWinLogging => "Enable Logging",
            DiagAction::CreateNiriWorkspace => "Create",
            DiagAction::RemoveNiriWorkspace => "Remove",
            DiagAction::InstallDesktop => "Add to System",
            DiagAction::UpdateDesktop => "Update",
            DiagAction::UninstallDesktop => "Remove",
            DiagAction::ShowAurUpdate => "How to Update",
            DiagAction::ShowAurUninstall => "Uninstall Info",
            DiagAction::OpenReleases => "View Release",
        };
        control = control.push(
            button(text(label).size(11.0))
                .padding(Padding {
                    top: 4.0,
                    bottom: 4.0,
                    left: 10.0,
                    right: 10.0,
                })
                .style(styled(ButtonKind::Subtle, palette))
                .on_press_maybe((!entry.action_disabled).then_some(message)),
        );
    }

    settings_row(
        palette,
        entry.icon,
        &entry.title,
        entry.subtitle(),
        control.into(),
        None,
    )
}

fn bottom_bar(app: &App, palette: Palette) -> Element<'_, Message> {
    let mut items = column![].width(Fill).spacing(8.0);

    let is_gnome_helper_needed =
        app.mode == InputMode::Gnome && crate::inputs::gnome::probe().is_err();

    if app.page == Page::Settings {
        if is_gnome_helper_needed {
            let banner: Element<'_, Message> = text("GNOME extension is required for window control. Open diagnostics to install & enable.")
                .size(11.0)
                .color(palette.warning)
                .width(Fill)
                .into();
            items = items.push(banner);
        } else if let Some(error) = app.snapshot.error_message.as_deref() {
            let banner: Element<'_, Message> = text(error)
                .size(11.0)
                .color(palette.warning)
                .width(Fill)
                .into();
            items = items.push(banner);
        }
    }

    let blocked = !app.snapshot.running
        && (app.snapshot.runtime_status == RuntimeStatus::Error || is_gnome_helper_needed)
        && app.page == Page::Settings;

    let label = if app.page == Page::Diagnostics {
        "Back to Settings".to_string()
    } else if app.snapshot.running {
        "Stop AntiAFK".to_string()
    } else if blocked {
        "Open diagnostics".to_string()
    } else {
        "Start AntiAFK".to_string()
    };

    let action = if app.page == Page::Diagnostics {
        Message::ToggleDiagnostics
    } else if blocked {
        Message::OpenDiagnostics
    } else {
        Message::ToggleRunning
    };

    items = items.push(
        button(
            container(text(label).size(14.0).font(SEMIBOLD))
                .width(Fill)
                .height(Fill)
                .center_x(Fill)
                .center_y(Fill),
        )
        .width(Fill)
        .height(42.0)
        .padding(0.0)
        .on_press(action)
        .style(main_button_style(
            palette,
            if app.page == Page::Diagnostics || blocked {
                None
            } else {
                Some(app.button_anim)
            },
        )),
    );

    container(items)
        .width(Fill)
        .padding(Padding {
            top: 14.0,
            bottom: 16.0,
            left: 16.0,
            right: 16.0,
        })
        .into()
}

fn status_text(app: &App, palette: Palette) -> (String, Color) {
    if app.page == Page::Diagnostics {
        return (String::new(), palette.muted);
    }
    match app.snapshot.runtime_status {
        RuntimeStatus::Stopped => ("AntiAFK is off".to_string(), palette.muted),
        RuntimeStatus::WaitingForSober => ("Waiting for Sober".to_string(), palette.muted),
        RuntimeStatus::Ready => ("AntiAFK is on".to_string(), palette.success),
        RuntimeStatus::Paused => {
            let label = if crate::inputs::common::recent_input_is_keyboard() {
                "Paused - keyboard activity"
            } else {
                "Paused - user activity"
            };
            (label.to_string(), palette.warning)
        }
        RuntimeStatus::PerformingAction => ("Performing action".to_string(), palette.success),
        RuntimeStatus::ReconnectChecking => ("Reconnect: checking...".to_string(), palette.success),
        RuntimeStatus::ReconnectFound => ("Reconnect: found".to_string(), palette.success),
        RuntimeStatus::Reconnecting => ("Reconnect: clicking".to_string(), palette.success),
        RuntimeStatus::ReconnectNotFound => ("Reconnect: not found".to_string(), palette.warning),
        RuntimeStatus::Error => ("Setup required".to_string(), palette.danger),
    }
}

fn card<'a>(content: impl Into<Element<'a, Message>>, palette: Palette) -> Element<'a, Message> {
    container(content)
        .width(Fill)
        .padding(Padding {
            top: 4.0,
            bottom: 4.0,
            left: 8.0,
            right: 8.0,
        })
        .style(move |_theme| container::Style {
            background: Some(Background::Color(palette.card)),
            border: Border {
                color: palette.hairline,
                width: 1.0,
                radius: 14.0.into(),
            },
            shadow: Shadow {
                color: alpha(Color::BLACK, 0.10),
                blur_radius: 12.0,
                ..Shadow::default()
            },
            ..container::Style::default()
        })
        .into()
}

fn divider<'a>(palette: Palette) -> Element<'a, Message> {
    container(Space::new().width(Fill).height(1.0))
        .width(Fill)
        .height(1.0)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(palette.hairline)),
            ..container::Style::default()
        })
        .into()
}

#[derive(Clone, Copy)]
struct InfoTip<'a> {
    icon: Icon,
    text: &'a str,
}

impl<'a> InfoTip<'a> {
    fn plain(text: &'a str) -> Self {
        Self {
            icon: Icon::Info,
            text,
        }
    }
}

fn settings_row<'a>(
    palette: Palette,
    icon_kind: Icon,
    title: &'a str,
    subtitle: Option<&'a str>,
    control: Element<'a, Message>,
    info: Option<InfoTip<'a>>,
) -> Element<'a, Message> {
    row_with_meta(palette, icon_kind, title, subtitle, control, info, false)
}

fn settings_sub_row<'a>(
    palette: Palette,
    icon_kind: Icon,
    title: &'a str,
    subtitle: Option<&'a str>,
    control: Element<'a, Message>,
    info: Option<InfoTip<'a>>,
) -> Element<'a, Message> {
    row_with_meta(palette, icon_kind, title, subtitle, control, info, true)
}

fn icon_tile(kind: Icon, palette: Palette) -> Element<'static, Message> {
    container(icon(kind, 15.0, palette.muted))
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .center_x(Length::Fixed(28.0))
        .center_y(Length::Fixed(28.0))
        .style(move |_theme| container::Style {
            background: Some(Background::Color(palette.surface_hover)),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 9.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

#[allow(clippy::too_many_arguments)]
fn row_with_meta<'a>(
    palette: Palette,
    icon_kind: Icon,
    title: &'a str,
    subtitle: Option<&'a str>,
    control: Element<'a, Message>,
    info: Option<InfoTip<'a>>,
    indent: bool,
) -> Element<'a, Message> {
    let mut labels = column![text(title).size(13.0)].width(Fill).spacing(2.0);
    if let Some(subtitle) = subtitle {
        labels = labels.push(text(subtitle).size(11.0).color(palette.muted));
    }

    let mut right = row![].spacing(6.0).align_y(Vertical::Center);
    if let Some(info) = info {
        right = right.push(tip(
            icon(info.icon, 14.0, palette.faint),
            container(text(info.text).size(11.0).width(Length::Fixed(220.0))).padding(2.0),
            TooltipPosition::Left,
            palette,
        ));
    }
    right = right.push(control);

    let tile = icon_tile(icon_kind, palette);

    let mut content = row![].spacing(10.0).align_y(Vertical::Center).width(Fill);
    if indent {
        content = content.push(Space::new().width(Length::Fixed(14.0)));
    }
    content = content.push(tile).push(labels).push(right);

    container(content)
        .width(Fill)
        .padding(Padding {
            top: 5.0,
            bottom: 5.0,
            left: 0.0,
            right: 0.0,
        })
        .into()
}

fn switch<'a>(
    is_on: bool,
    enabled: bool,
    progress: f32,
    palette: Palette,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    let progress = progress.clamp(0.0, 1.0);
    let track_color = if enabled {
        mix(alpha(palette.fg, 0.22), palette.accent, progress)
    } else {
        alpha(palette.fg, 0.12)
    };

    let knob =
        container(Space::new().width(16.0).height(16.0)).style(move |_theme| container::Style {
            background: Some(Background::Color(if enabled {
                Color::WHITE
            } else {
                alpha(Color::WHITE, 0.65)
            })),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 8.0.into(),
            },
            ..container::Style::default()
        });

    let track = container(
        row![
            Space::new().width(Length::Fixed(3.0 + progress * 18.0)),
            knob,
            Space::new().width(Fill),
        ]
        .height(22.0)
        .align_y(Vertical::Center),
    )
    .width(40.0)
    .height(22.0)
    .style(move |_theme| container::Style {
        background: Some(Background::Color(track_color)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 11.0.into(),
        },
        ..container::Style::default()
    });

    button(track)
        .padding(0.0)
        .style(switch_button_style(palette))
        .on_press_maybe(enabled.then(|| on_toggle(!is_on)))
        .into()
}

fn switch_button_style(palette: Palette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered => Some(Background::Color(alpha(palette.accent, 0.16))),
            _ => None,
        };
        button::Style {
            background,
            text_color: palette.fg,
            border: Border {
                color: Color::TRANSPARENT,
                width: 3.0,
                radius: 14.0.into(),
            },
            shadow: Shadow::default(),
            ..button::Style::default()
        }
    }
}

fn icon_button(kind: Icon, palette: Palette, message: Message) -> Element<'static, Message> {
    button(
        container(icon(kind, 14.0, palette.muted))
            .width(Length::Fixed(26.0))
            .height(Length::Fixed(26.0))
            .center_x(Length::Fixed(26.0))
            .center_y(Length::Fixed(26.0)),
    )
    .width(26.0)
    .height(26.0)
    .padding(0.0)
    .on_press(message)
    .style(header_button_style(palette))
    .into()
}

fn header_button_style(palette: Palette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => {
                Some(Background::Color(palette.surface_hover))
            }
            _ => None,
        };
        button::Style {
            background,
            text_color: palette.fg,
            border: Border {
                color: palette.border,
                width: 1.0,
                radius: 7.0.into(),
            },
            shadow: Shadow::default(),
            ..button::Style::default()
        }
    }
}

#[derive(Clone, Copy)]
enum ButtonKind {
    Primary,
    Subtle,
}

fn style_for(kind: ButtonKind) -> fn(Palette, &Theme, button::Status) -> button::Style {
    match kind {
        ButtonKind::Primary => primary_style,
        ButtonKind::Subtle => subtle_style,
    }
}

fn styled(kind: ButtonKind, palette: Palette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| style_for(kind)(palette, theme, status)
}

fn main_button_style(
    palette: Palette,
    progress: Option<f32>,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    let blended =
        progress.map(|progress| mix(palette.accent, palette.danger, progress.clamp(0.0, 1.0)));

    move |theme, status| match blended {
        Some(base) => match status {
            button::Status::Hovered => {
                base_button(mix(base, Color::WHITE, 0.12), palette.on_accent, 10.0)
            }
            button::Status::Pressed => {
                base_button(mix(base, Color::BLACK, 0.12), palette.on_accent, 10.0)
            }
            button::Status::Disabled => {
                base_button(alpha(base, 0.4), alpha(palette.on_accent, 0.6), 10.0)
            }
            _ => base_button(base, palette.on_accent, 10.0),
        },
        None => subtle_style(palette, theme, status),
    }
}

fn base_button(background: Color, text_color: Color, radius: f32) -> button::Style {
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius.into(),
        },
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

fn primary_style(palette: Palette, _theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => base_button(
            mix(palette.accent, Color::WHITE, 0.12),
            palette.on_accent,
            10.0,
        ),
        button::Status::Pressed => base_button(
            mix(palette.accent, Color::BLACK, 0.12),
            palette.on_accent,
            10.0,
        ),
        button::Status::Disabled => base_button(
            alpha(palette.accent, 0.4),
            alpha(palette.on_accent, 0.6),
            10.0,
        ),
        _ => base_button(palette.accent, palette.on_accent, 10.0),
    }
}

fn subtle_style(palette: Palette, _theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => palette.surface_hover,
        _ => palette.surface,
    };
    let text_color = match status {
        button::Status::Disabled => palette.faint,
        _ => palette.fg,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: palette.border,
            width: 1.0,
            radius: 9.0.into(),
        },
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

fn command_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn uinput_writable() -> bool {
    std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/uinput")
        .is_ok()
}

fn build_diagnostics(
    settings: &AppState,
    version: &VersionState,
    desktop_status: &DesktopIntegrationStatus,
) -> Vec<DiagRow> {
    let mode = settings.input_mode();
    let mut rows = Vec::new();

    let preflight = crate::backend::preflight(settings);
    let preflight_ok = preflight.is_ok();

    let (version_label, version_status, version_detail, version_action) = if version.checking {
        (
            Some("Checking...".to_string()),
            DiagStatus::Ok,
            None,
            Some(DiagAction::CheckVersion),
        )
    } else {
        match version.result.as_ref() {
            Some(Ok((true, _))) => (
                Some(format!("v{CURRENT_VERSION} - latest")),
                DiagStatus::Ok,
                None,
                Some(DiagAction::CheckVersion),
            ),
            Some(Ok((false, latest))) => {
                if let DesktopIntegrationStatus::Aur { package, helper } = desktop_status {
                    (
                        Some(format!("v{latest} available (AUR)")),
                        DiagStatus::Warning,
                        Some(format!("Update via AUR: {helper} -S {package}")),
                        Some(DiagAction::ShowAurUpdate),
                    )
                } else {
                    (
                        Some(format!("v{latest} available")),
                        DiagStatus::Warning,
                        Some("New release on GitHub".to_string()),
                        Some(DiagAction::OpenReleases),
                    )
                }
            }
            Some(Err(_)) => (
                Some("Unavailable".to_string()),
                DiagStatus::Error,
                None,
                Some(DiagAction::CheckVersion),
            ),
            None => (None, DiagStatus::Ok, None, Some(DiagAction::CheckVersion)),
        }
    };
    rows.push(DiagRow {
        icon: Icon::Info,
        title: "Version".to_string(),
        detail: version_detail,
        status: version_status,
        action: version_action,
        action_disabled: version.checking,
        label: version_label,
    });

    rows.push(DiagRow {
        icon: Icon::Settings,
        title: "Desktop Session".to_string(),
        detail: Some(mode.display_label()),
        status: if mode.is_supported() {
            DiagStatus::Ok
        } else {
            DiagStatus::Error
        },
        action: None,
        action_disabled: false,
        label: None,
    });

    let sober_pids = crate::backend::sober_pids();
    let sober_running = !sober_pids.is_empty();
    rows.push(DiagRow {
        icon: Icon::Action,
        title: "Sober Process".to_string(),
        detail: if sober_running {
            Some(format!("Running (PID {})", sober_pids.join(", ")))
        } else {
            Some("Game not running".to_string())
        },
        status: DiagStatus::Ok,
        action: None,
        action_disabled: false,
        label: if sober_running {
            Some("Detected".to_string())
        } else {
            Some("Waiting".to_string())
        },
    });

    let (desktop_detail, desktop_status_type, desktop_label, desktop_action) = match desktop_status
    {
        DesktopIntegrationStatus::Aur { package, .. } => (
            Some(format!("Managed by AUR package ({package})")),
            DiagStatus::Ok,
            Some("AUR Package".to_string()),
            Some(DiagAction::ShowAurUninstall),
        ),
        DesktopIntegrationStatus::Installed { up_to_date: true } => (
            Some("Desktop launcher and icon active (~/.local/bin)".to_string()),
            DiagStatus::Ok,
            Some("Installed".to_string()),
            Some(DiagAction::UninstallDesktop),
        ),
        DesktopIntegrationStatus::Installed { up_to_date: false } => (
            Some("Installed binary in ~/.local/bin differs from this build".to_string()),
            DiagStatus::Warning,
            Some("Update Available".to_string()),
            Some(DiagAction::UpdateDesktop),
        ),
        DesktopIntegrationStatus::NotInstalled => (
            Some("Add to applications menu and desktop launcher".to_string()),
            DiagStatus::Warning,
            Some("Not Installed".to_string()),
            Some(DiagAction::InstallDesktop),
        ),
    };

    rows.push(DiagRow {
        icon: Icon::Layers,
        title: "Desktop Integration".to_string(),
        detail: desktop_detail,
        status: desktop_status_type,
        action: desktop_action,
        action_disabled: false,
        label: desktop_label,
    });

    let uinput_ok = uinput_writable();
    let rule_exists = std::path::Path::new(UINPUT_RULE).exists();
    if matches!(mode, InputMode::X11 | InputMode::I3) && !uinput_ok {
        rows.push(DiagRow {
            icon: Icon::Mouse,
            title: "Input Emulation".to_string(),
            detail: Some("xdotool fallback active".to_string()),
            status: DiagStatus::Ok,
            action: None,
            action_disabled: false,
            label: Some("xdotool".to_string()),
        });
    } else {
        rows.push(DiagRow {
            icon: if uinput_ok {
                Icon::Mouse
            } else {
                Icon::Warning
            },
            title: "Input Emulation (/dev/uinput)".to_string(),
            detail: (!uinput_ok && !rule_exists).then(|| "Grant access to /dev/uinput".to_string()),
            status: if uinput_ok {
                DiagStatus::Ok
            } else {
                DiagStatus::Error
            },
            action: (!uinput_ok || rule_exists).then_some(if rule_exists {
                DiagAction::RemoveRule
            } else {
                DiagAction::Fix
            }),
            action_disabled: false,
            label: if uinput_ok {
                None
            } else {
                Some("Fix".to_string())
            },
        });
    }

    match mode {
        InputMode::Hyprland => {
            rows.push(component_row("hyprctl Utility", "hyprctl", &["version"]));
        }
        InputMode::Kde => {
            let qdbus = command_succeeds("qdbus6", &["--version"])
                || command_succeeds("qdbus", &["--version"]);
            rows.push(DiagRow {
                icon: if qdbus { Icon::Cpu } else { Icon::Warning },
                title: "KDE D-Bus (qdbus)".to_string(),
                detail: None,
                status: if qdbus {
                    DiagStatus::Ok
                } else {
                    DiagStatus::Error
                },
                action: None,
                action_disabled: false,
                label: None,
            });
            let bridge = crate::inputs::kde::script_bridge();
            let bridge_ok = bridge.is_ok();
            rows.push(DiagRow {
                icon: if bridge_ok {
                    Icon::Settings
                } else {
                    Icon::Warning
                },
                title: "KWin Script Bridge".to_string(),
                detail: bridge.err(),
                status: if bridge_ok {
                    DiagStatus::Ok
                } else {
                    DiagStatus::Error
                },
                action: (!bridge_ok).then_some(DiagAction::EnableKWinLogging),
                action_disabled: false,
                label: (!bridge_ok).then_some("Enable Logging".to_string()),
            });
        }
        InputMode::Niri => {
            let socket_ok = std::env::var_os("NIRI_SOCKET").is_some();
            rows.push(DiagRow {
                icon: if socket_ok {
                    Icon::Layers
                } else {
                    Icon::Warning
                },
                title: "Niri Socket".to_string(),
                detail: None,
                status: if socket_ok {
                    DiagStatus::Ok
                } else {
                    DiagStatus::Error
                },
                action: None,
                action_disabled: false,
                label: None,
            });
            let wtype_ok = crate::inputs::niri::keyboard_ready();
            rows.push(DiagRow {
                icon: if wtype_ok {
                    Icon::Action
                } else {
                    Icon::Warning
                },
                title: "wtype Utility".to_string(),
                detail: Some(if wtype_ok {
                    "Installed (keyboard input)".to_string()
                } else {
                    "Missing: required for Niri keyboard emulation".to_string()
                }),
                status: if wtype_ok {
                    DiagStatus::Ok
                } else {
                    DiagStatus::Error
                },
                action: None,
                action_disabled: false,
                label: None,
            });
            let (all, matched) = crate::inputs::niri::window_summary();
            rows.push(DiagRow {
                icon: if matched > 0 {
                    Icon::Action
                } else {
                    Icon::Info
                },
                title: "Niri Window Target".to_string(),
                detail: Some(format!("{matched} Sober window(s) of {all} total")),
                status: if matched > 0 {
                    DiagStatus::Ok
                } else {
                    DiagStatus::Warning
                },
                action: None,
                action_disabled: false,
                label: None,
            });
            let ws_ready = crate::inputs::niri::hide_workspace_ready();
            rows.push(DiagRow {
                icon: if ws_ready {
                    Icon::Layers
                } else {
                    Icon::Warning
                },
                title: "Niri Hidden Workspace".to_string(),
                detail: Some(if ws_ready {
                    format!(
                        "workspace \"{}\" ready",
                        crate::inputs::niri::HIDDEN_WORKSPACE
                    )
                } else {
                    format!(
                        "workspace \"{}\" not configured",
                        crate::inputs::niri::HIDDEN_WORKSPACE
                    )
                }),
                status: if ws_ready {
                    DiagStatus::Ok
                } else {
                    DiagStatus::Warning
                },
                action: Some(if ws_ready {
                    DiagAction::RemoveNiriWorkspace
                } else {
                    DiagAction::CreateNiriWorkspace
                }),
                action_disabled: false,
                label: Some(if ws_ready {
                    "Remove".to_string()
                } else {
                    "Create".to_string()
                }),
            });
        }
        InputMode::Cosmic => {
            let cosmic_probe = crate::inputs::cosmic::probe();
            let cosmic_ok = cosmic_probe.is_ok();
            rows.push(DiagRow {
                icon: if cosmic_ok {
                    Icon::Layers
                } else {
                    Icon::Warning
                },
                title: "COSMIC Protocols".to_string(),
                detail: cosmic_probe.as_ref().err().cloned(),
                status: if cosmic_ok {
                    DiagStatus::Ok
                } else {
                    DiagStatus::Error
                },
                action: None,
                action_disabled: false,
                label: None,
            });
        }
        InputMode::Gnome => {
            let helper = crate::inputs::gnome::probe();
            let helper_ok = helper.is_ok();
            let extension_installed = is_gnome_extension_installed();
            let extension_enabled = is_gnome_extension_enabled();

            let (action, label) = if helper_ok {
                (None, None)
            } else {
                (
                    Some(DiagAction::RunGnomeQuickSetup),
                    Some("Install & Enable".to_string()),
                )
            };

            let detail = if helper_ok {
                Some("Active & connected via D-Bus".to_string())
            } else if !extension_installed {
                Some("Not installed. Click Install & Enable to configure.".to_string())
            } else if extension_enabled {
                Some(
                    "Installed & enabled. Please log out and back in to GNOME to apply."
                        .to_string(),
                )
            } else {
                Some("Installed but disabled. Click Install & Enable to enable.".to_string())
            };

            rows.push(DiagRow {
                icon: if helper_ok {
                    Icon::Layers
                } else {
                    Icon::Warning
                },
                title: "GNOME Extension".to_string(),
                detail,
                status: if helper_ok {
                    DiagStatus::Ok
                } else {
                    DiagStatus::Error
                },
                action,
                action_disabled: false,
                label,
            });
        }
        InputMode::I3 => {
            rows.push(component_row("xdotool Utility", "xdotool", &["--version"]));
            rows.push(component_row("i3-msg Utility", "i3-msg", &["-v"]));
        }
        InputMode::X11 => {
            rows.push(component_row("xdotool Utility", "xdotool", &["--version"]));
            let wmctrl_ok = command_succeeds("wmctrl", &["-m"]);
            rows.push(DiagRow {
                icon: if wmctrl_ok { Icon::Cpu } else { Icon::Info },
                title: "wmctrl Utility".to_string(),
                detail: Some(
                    if wmctrl_ok {
                        "Installed"
                    } else {
                        "Optional (xdotool fallback)"
                    }
                    .to_string(),
                ),
                status: if wmctrl_ok {
                    DiagStatus::Ok
                } else {
                    DiagStatus::Warning
                },
                action: None,
                action_disabled: false,
                label: None,
            });
        }
        InputMode::Unsupported => {}
    }

    let capture_tool = detect_capture_tool(mode);
    let capture_ok = capture_tool.is_some();
    rows.push(DiagRow {
        icon: if capture_ok {
            Icon::Reconnect
        } else if settings.auto_reconnect {
            Icon::Warning
        } else {
            Icon::Info
        },
        title: "Screen Capture".to_string(),
        detail: capture_tool.map(|t| {
            if t.ends_with("Extension") {
                format!("{t} active")
            } else {
                format!("{t} utility detected")
            }
        }),
        status: match capture_tool {
            Some(_) => DiagStatus::Ok,
            None if settings.auto_reconnect => DiagStatus::Error,
            None => DiagStatus::Warning,
        },
        action: None,
        action_disabled: false,
        label: None,
    });

    if settings.fps_capper {
        let quota = crate::backend::cpu_quota_report();
        rows.push(DiagRow {
            icon: Icon::Performance,
            title: "FPS Throttle".to_string(),
            detail: Some(if quota.limit.is_none() {
                "Inactive (unlocked or no game)".to_string()
            } else {
                format!(
                    "Active ({}% limit on {} process(es))",
                    quota.limit.unwrap_or_default(),
                    quota.pids
                )
            }),
            status: if quota.limit.is_some() || quota.focus == Some(true) {
                DiagStatus::Ok
            } else {
                DiagStatus::Warning
            },
            action: None,
            action_disabled: false,
            label: None,
        });
    }

    rows.push(DiagRow {
        icon: if preflight_ok {
            Icon::Check
        } else {
            Icon::Warning
        },
        title: "Preflight Check".to_string(),
        detail: preflight.err(),
        status: if preflight_ok {
            DiagStatus::Ok
        } else {
            DiagStatus::Error
        },
        action: None,
        action_disabled: false,
        label: if preflight_ok {
            Some("Ready".to_string())
        } else {
            Some("Action Required".to_string())
        },
    });

    rows
}

fn detect_capture_tool(mode: InputMode) -> Option<&'static str> {
    match mode {
        InputMode::Hyprland => {
            if crate::inputs::common::program_available("grim") {
                Some("grim")
            } else {
                None
            }
        }
        InputMode::Kde => {
            if crate::inputs::common::program_available("spectacle") {
                Some("spectacle")
            } else {
                None
            }
        }
        InputMode::Gnome => {
            if crate::inputs::gnome::probe().is_ok() {
                Some("GNOME Extension")
            } else if crate::inputs::common::program_available("grim") {
                Some("grim")
            } else {
                None
            }
        }
        InputMode::Niri | InputMode::Cosmic => {
            if crate::inputs::common::program_available("grim") {
                Some("grim")
            } else if crate::inputs::common::program_available("import") {
                Some("import")
            } else {
                None
            }
        }
        InputMode::X11 | InputMode::I3 => {
            for tool in ["maim", "import", "scrot"] {
                if crate::inputs::common::program_available(tool) {
                    return Some(tool);
                }
            }
            None
        }
        InputMode::Unsupported => None,
    }
}

fn component_row(title: &str, program: &str, args: &[&str]) -> DiagRow {
    let available = command_succeeds(program, args);
    DiagRow {
        icon: if available { Icon::Cpu } else { Icon::Warning },
        title: title.to_string(),
        detail: None,
        status: if available {
            DiagStatus::Ok
        } else {
            DiagStatus::Error
        },
        action: None,
        action_disabled: false,
        label: None,
    }
}

fn add_niri_hidden_workspace() -> Result<(), String> {
    let config_dir = dirs::config_dir()
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .ok_or_else(|| "Could not determine config directory".to_string())?
        .join("niri");
    let config_path = config_dir.join("config.kdl");
    if !config_path.exists() {
        return Err(format!(
            "Niri config file not found at {}",
            config_path.display()
        ));
    }
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read niri config: {e}"))?;
    let workspace_decl = format!("workspace \"{}\"", crate::inputs::niri::HIDDEN_WORKSPACE);
    if content.contains(&workspace_decl) {
        return Ok(());
    }

    let candidate = format!(
        "{}\n\n// AntiAFK hidden workspace\n{}\n",
        content.trim_end(),
        workspace_decl
    );
    let temp_file =
        std::env::temp_dir().join(format!("niri-config-test-{}.kdl", std::process::id()));
    std::fs::write(&temp_file, &candidate)
        .map_err(|e| format!("Failed to write candidate config: {e}"))?;

    let validate_output = Command::new("niri")
        .args(["validate", "-c", temp_file.to_str().unwrap_or("")])
        .output();
    let _ = std::fs::remove_file(&temp_file);

    match validate_output {
        Ok(out) if out.status.success() => {
            let backup_path = config_dir.join("config.kdl.bak");
            let _ = std::fs::copy(&config_path, backup_path);
            std::fs::write(&config_path, candidate)
                .map_err(|e| format!("Failed to update niri config: {e}"))?;
            Ok(())
        }
        Ok(out) => Err(format!(
            "Niri config validation failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => Err(format!("Failed to run niri validate: {e}")),
    }
}

fn remove_niri_hidden_workspace() -> Result<(), String> {
    let config_dir = dirs::config_dir()
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .ok_or_else(|| "Could not determine config directory".to_string())?
        .join("niri");
    let config_path = config_dir.join("config.kdl");
    if !config_path.exists() {
        return Err(format!(
            "Niri config file not found at {}",
            config_path.display()
        ));
    }
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read niri config: {e}"))?;
    let workspace_decl = format!("workspace \"{}\"", crate::inputs::niri::HIDDEN_WORKSPACE);
    if !content.contains(&workspace_decl) {
        return Ok(());
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut filtered = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.contains("// AntiAFK hidden workspace") {
            i += 1;
            if i < lines.len() && lines[i].contains(&workspace_decl) {
                i += 1;
                continue;
            }
        }
        if line.contains(&workspace_decl) {
            i += 1;
            continue;
        }
        filtered.push(line);
        i += 1;
    }
    let candidate = filtered.join("\n") + "\n";
    let temp_file =
        std::env::temp_dir().join(format!("niri-config-test-rm-{}.kdl", std::process::id()));
    std::fs::write(&temp_file, &candidate)
        .map_err(|e| format!("Failed to write candidate config: {e}"))?;

    let validate_output = Command::new("niri")
        .args(["validate", "-c", temp_file.to_str().unwrap_or("")])
        .output();
    let _ = std::fs::remove_file(&temp_file);

    match validate_output {
        Ok(out) if out.status.success() => {
            let backup_path = config_dir.join("config.kdl.bak");
            let _ = std::fs::copy(&config_path, backup_path);
            std::fs::write(&config_path, candidate)
                .map_err(|e| format!("Failed to update niri config: {e}"))?;
            Ok(())
        }
        Ok(out) => Err(format!(
            "Niri config validation failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => Err(format!("Failed to run niri validate: {e}")),
    }
}

fn is_gnome_extension_installed() -> bool {
    dirs::home_dir()
        .map(|h| {
            h.join(".local/share/gnome-shell/extensions")
                .join(GNOME_EXTENSION_UUID)
                .join("extension.js")
                .exists()
        })
        .unwrap_or(false)
}

fn is_gnome_extension_enabled() -> bool {
    let list_ok = Command::new("gnome-extensions")
        .args(["list", "--enabled"])
        .output()
        .map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .lines()
                .any(|line| line.trim() == GNOME_EXTENSION_UUID)
        })
        .unwrap_or(false);

    if list_ok {
        return true;
    }

    gsettings_value("org.gnome.shell", "enabled-extensions")
        .map(|val| val.contains(GNOME_EXTENSION_UUID))
        .unwrap_or(false)
}

async fn run_gnome_quick_setup() -> Result<String, String> {
    crate::install_gnome_helper()?;
    let _ = Command::new("gnome-extensions")
        .args(["enable", GNOME_EXTENSION_UUID])
        .output();
    Ok(format!("Configured {GNOME_EXTENSION_UUID}"))
}

async fn check_latest_version() -> Result<(bool, String), String> {
    let output = Command::new("curl")
        .args([
            "-s",
            "--connect-timeout",
            "3",
            "https://raw.githubusercontent.com/agzes/AntiAFK-RBX-Sober/main/version",
        ])
        .output();

    if let Ok(output) = output
        && output.status.success()
    {
        let latest = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !latest.is_empty() {
            return Ok((CURRENT_VERSION == latest, latest));
        }
    }

    Err("Failed to check for updates".to_string())
}

async fn run_uinput_command(command: String) {
    let _ = std::process::Command::new("pkexec")
        .args(["sh", "-c", &command])
        .output();
}

fn system_dark_preference() -> Option<bool> {
    if let Ok(theme) = std::env::var("GTK_THEME") {
        return Some(theme.to_lowercase().contains("dark"));
    }

    let mut answered = false;
    for (schema, key) in [
        ("org.gnome.desktop.interface", "color-scheme"),
        ("org.gnome.desktop.interface", "gtk-theme"),
        ("org.kde.gtk.config", "darkMode"),
    ] {
        let Some(value) = gsettings_value(schema, key) else {
            continue;
        };
        answered = true;
        let value = value.to_lowercase();
        if value.contains("prefer-dark") || value.contains("dark") || value.trim() == "true" {
            return Some(true);
        }
    }

    answered.then_some(false)
}

fn gsettings_value(schema: &str, key: &str) -> Option<String> {
    let output = Command::new("gsettings")
        .args(["get", schema, key])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

#[derive(Clone, Copy)]
struct SystemColors {
    background: Color,
    text: Color,
    card: Color,
    success: Color,
    danger: Color,
}

fn system_stylesheets() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut files = vec![
        format!("{home}/.config/gtk-4.0/gtk.css"),
        format!("{home}/.config/gtk-3.0/gtk.css"),
    ];

    if let Some(theme) = gsettings_value("org.gnome.desktop.interface", "gtk-theme") {
        let theme = theme.trim().trim_matches('\'');
        if !theme.is_empty() && !theme.contains(char::is_whitespace) {
            let mut roots = vec![
                format!("{home}/.themes"),
                format!("{home}/.local/share/themes"),
            ];
            let data_dirs = std::env::var("XDG_DATA_DIRS")
                .unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
            for root in data_dirs.split(':').filter(|root| !root.is_empty()) {
                roots.push(format!("{root}/themes"));
            }
            for root in roots {
                for css in ["gtk-4.0/gtk.css", "gtk-4.0/colors.css", "gtk-3.0/gtk.css"] {
                    files.push(format!("{root}/{theme}/{css}"));
                }
            }
        }
    }

    files
}

fn system_accent(dark: bool) -> Option<Color> {
    for path in system_stylesheets() {
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Some(color) = gtk_accent(&text, dark)
        {
            return Some(color);
        }
    }

    None
}

fn system_palette(dark: bool) -> Option<SystemColors> {
    for path in system_stylesheets() {
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Some(colors) = parse_system_palette(&text, dark)
        {
            return Some(colors);
        }
    }

    None
}

fn parse_system_palette(css: &str, dark: bool) -> Option<SystemColors> {
    let definitions = gtk_definitions(css);
    let value = |key: &str| {
        lookup_definition(&definitions, key, dark)
            .and_then(|value| resolve_value(value, &definitions, dark, 0))
    };

    let background = value("window_bg_color")?;
    let text = value("window_fg_color")?;
    Some(SystemColors {
        background,
        text,
        card: value("card_bg_color").unwrap_or_else(|| mix(background, text, 0.07)),
        success: value("success_color").unwrap_or(if dark { SUCCESS_DARK } else { SUCCESS_LIGHT }),
        danger: value("destructive_color").unwrap_or(if dark { DANGER_DARK } else { DANGER_LIGHT }),
    })
}

fn gtk_accent(css: &str, dark: bool) -> Option<Color> {
    let definitions = gtk_definitions(css);

    for key in [
        "accent_bg_color",
        "accent_color",
        "theme_selected_bg_color",
        "selected_bg_color",
    ] {
        let Some(value) = lookup_definition(&definitions, key, dark) else {
            continue;
        };
        if let Some(color) = resolve_value(value, &definitions, dark, 0) {
            return Some(color);
        }
    }

    None
}

fn gtk_definitions(css: &str) -> Vec<(Option<bool>, &str, &str)> {
    let mut definitions = Vec::new();
    let mut context: Option<bool> = None;
    let mut depth = 0usize;

    for line in css.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("@media") {
            context = if rest.contains("prefers-color-scheme: dark") {
                Some(true)
            } else if rest.contains("prefers-color-scheme: light") {
                Some(false)
            } else {
                None
            };
        }

        if let Some(rest) = trimmed.strip_prefix("@define-color")
            && rest.starts_with(char::is_whitespace)
        {
            let rest = rest.trim_start();
            if let Some((name, value)) = rest.split_once(char::is_whitespace) {
                definitions.push((context, name, value.trim().trim_end_matches(';')));
            }
        }

        depth += line.matches('{').count();
        let closes = line.matches('}').count();
        depth = depth.saturating_sub(closes);
        if closes > 0 && depth == 0 {
            context = None;
        }
    }

    definitions
}

fn lookup_definition<'a>(
    definitions: &[(Option<bool>, &'a str, &'a str)],
    name: &str,
    dark: bool,
) -> Option<&'a str> {
    for wanted in [Some(dark), None] {
        let mut found: Option<&str> = None;
        for (context, key, value) in definitions {
            if *key == name && *context == wanted {
                found = Some(*value);
            }
        }
        if found.is_some() {
            return found;
        }
    }
    None
}

fn resolve_value(
    value: &str,
    definitions: &[(Option<bool>, &str, &str)],
    dark: bool,
    depth: u8,
) -> Option<Color> {
    if depth > 5 {
        return None;
    }
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    let reference = value.strip_prefix('@')?;
    let target = lookup_definition(definitions, reference, dark)?;
    resolve_value(target, definitions, dark, depth + 1)
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let component = |range: std::ops::Range<usize>| u8::from_str_radix(hex.get(range)?, 16).ok();
    match hex.len() {
        6 => Some(Color::from_rgb8(
            component(0..2)?,
            component(2..4)?,
            component(4..6)?,
        )),
        3 => Some(Color::from_rgb8(
            component(0..1)? * 0x11,
            component(1..2)? * 0x11,
            component(2..3)? * 0x11,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gtk_accent_resolves_named_references() {
        let css = "@define-color blue_3 #3584e4;\n@define-color accent_bg_color @blue_3;\n";
        assert_eq!(
            gtk_accent(css, true),
            Some(Color::from_rgb8(0x35, 0x84, 0xE4))
        );
    }

    #[test]
    fn gtk_accent_skips_mixes_and_reads_short_hex() {
        let css = "@define-color accent_color mix(@accent_bg_color,white,0.4);\n\
                   @define-color theme_selected_bg_color #4f6;\n";
        assert_eq!(
            gtk_accent(css, true),
            Some(Color::from_rgb8(0x44, 0xFF, 0x66))
        );
    }

    #[test]
    fn system_palette_reads_the_matching_block() {
        let css = "@media (prefers-color-scheme: dark) {\n    \
                   @define-color window_bg_color #1a1111;\n    \
                   @define-color window_fg_color #f1dedd;\n    \
                   @define-color card_bg_color #271d1d;\n    \
                   @define-color success_color #b5ccba;\n    \
                   @define-color destructive_color #ffb4ab;\n}\n";
        let colors = parse_system_palette(css, true).unwrap();
        assert_eq!(colors.background, Color::from_rgb8(0x1a, 0x11, 0x11));
        assert_eq!(colors.text, Color::from_rgb8(0xf1, 0xde, 0xdd));
        assert_eq!(colors.card, Color::from_rgb8(0x27, 0x1d, 0x1d));
        assert_eq!(colors.success, Color::from_rgb8(0xb5, 0xcc, 0xba));
        assert_eq!(colors.danger, Color::from_rgb8(0xff, 0xb4, 0xab));
        assert!(parse_system_palette(css, false).is_none());
    }

    #[test]
    fn gtk_accent_prefers_the_matching_media_block() {
        let css = "@media (prefers-color-scheme: light) {\n    \
                   @define-color accent_bg_color #904a47;\n}\n\
                   @media (prefers-color-scheme: dark) {\n    \
                   @define-color accent_bg_color #ffb3af;\n}\n";
        assert_eq!(
            gtk_accent(css, true),
            Some(Color::from_rgb8(0xFF, 0xB3, 0xAF))
        );
        assert_eq!(
            gtk_accent(css, false),
            Some(Color::from_rgb8(0x90, 0x4A, 0x47))
        );
    }

    #[test]
    fn interval_presets_round_trip() {
        for preset in [
            Interval::Min4,
            Interval::Min6,
            Interval::Min9,
            Interval::Min19,
        ] {
            let seconds = preset.seconds().expect("preset has seconds");
            assert_eq!(Interval::from_seconds(seconds), preset);
        }
    }

    #[test]
    fn unknown_interval_is_custom() {
        assert_eq!(Interval::from_seconds(300), Interval::Custom);
        assert!(Interval::Custom.seconds().is_none());
    }

    #[test]
    fn custom_minutes_formatting_is_compact() {
        assert_eq!(format_minutes(240), "4");
        assert_eq!(format_minutes(270), "4.5");
        assert_eq!(format_minutes(1140), "19");
    }

    #[test]
    fn idle_action_maps_both_ways() {
        let mut state = AppState::default();
        IdleAction::Walk.apply(&mut state);
        assert_eq!(IdleAction::from_state(&state), IdleAction::Walk);
        IdleAction::Camera.apply(&mut state);
        assert_eq!(IdleAction::from_state(&state), IdleAction::Camera);
        IdleAction::Jump.apply(&mut state);
        assert_eq!(IdleAction::from_state(&state), IdleAction::Jump);
    }

    #[test]
    fn enabling_auto_start_clears_manual_stop() {
        let mut state = AppState {
            manually_stopped: true,
            ..AppState::default()
        };
        Flag::AutoStart.apply(&mut state, true);
        assert!(!state.manually_stopped);
        assert!(state.auto_start);
    }
}
