use crate::environment::{InputMode, detect_mode};
use crate::inputs::common::{find_qdbus, program_available};
use crate::state::{
    APP_ID, APP_TITLE, AppState, RuntimeStatus, SELF_MARKER, SharedState, set_runtime_error,
    set_runtime_status,
};
use notify_rust::Notification;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

fn command_works(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn has_uinput_access() -> bool {
    std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/uinput")
        .is_ok()
}

pub fn supports_hide(mode: InputMode) -> bool {
    match mode {
        InputMode::Hyprland | InputMode::Kde => true,
        InputMode::Niri => crate::inputs::niri::supports_hide(),
        InputMode::Cosmic => crate::inputs::cosmic::supports_hide_mode(),
        InputMode::X11 | InputMode::I3 => crate::inputs::x11::supports_hide(),
        InputMode::Gnome => crate::inputs::gnome::supports_hide(),
        InputMode::Unsupported => false,
    }
}

pub fn supports_enumeration(mode: InputMode) -> bool {
    match mode {
        InputMode::Hyprland | InputMode::Kde | InputMode::Niri | InputMode::Cosmic => true,
        InputMode::Gnome => true,
        InputMode::X11 | InputMode::I3 => crate::inputs::x11::supports_enumeration(),
        InputMode::Unsupported => false,
    }
}

pub fn supports_capture(mode: InputMode) -> bool {
    match mode {
        InputMode::Hyprland => program_available("grim"),
        InputMode::Kde => program_available("spectacle"),
        InputMode::Niri => crate::inputs::niri::supports_capture(),
        InputMode::Cosmic => crate::inputs::cosmic::supports_capture_mode(),
        InputMode::Gnome => crate::inputs::gnome::supports_capture(),
        InputMode::X11 | InputMode::I3 => crate::inputs::x11::supports_capture(),
        InputMode::Unsupported => false,
    }
}

pub fn cpu_quota_supported(mode: InputMode) -> bool {
    mode != InputMode::Cosmic
}

pub fn show_sober(mode: InputMode) -> Result<(), String> {
    match mode {
        InputMode::Hyprland => crate::inputs::hyprland::show_sober(),
        InputMode::Kde => crate::inputs::kde::show_sober(),
        InputMode::Niri => crate::inputs::niri::show_sober(),
        InputMode::Cosmic => crate::inputs::cosmic::show_sober(),
        InputMode::Gnome => crate::inputs::gnome::show_sober(),
        InputMode::X11 | InputMode::I3 => crate::inputs::x11::show_sober(),
        InputMode::Unsupported => Err("This desktop is not supported yet.".to_string()),
    }
}

pub fn hide_sober(mode: InputMode) -> Result<(), String> {
    if !supports_hide(mode) {
        return Err("Hide Game is not available on this desktop.".to_string());
    }
    match mode {
        InputMode::Hyprland => crate::inputs::hyprland::hide_sober(),
        InputMode::Kde => crate::inputs::kde::hide_sober(),
        InputMode::Niri => crate::inputs::niri::hide_sober(),
        InputMode::Cosmic => crate::inputs::cosmic::hide_sober(),
        InputMode::Gnome => crate::inputs::gnome::hide_sober(),
        InputMode::X11 | InputMode::I3 => crate::inputs::x11::hide_sober(),
        InputMode::Unsupported => Err("Hide Game is not available on this desktop.".to_string()),
    }
}

pub fn restore_hidden_windows() {
    let mode = detect_mode();
    let _ = show_sober(mode);
    crate::inputs::niri::restore_hidden_windows();
}

pub fn preflight(settings: &AppState) -> Result<(), String> {
    let mode = InputMode::from_legacy(settings.mode);
    let mut issues = Vec::new();

    match mode {
        InputMode::Hyprland => {
            if !command_works("hyprctl", &["version"]) {
                issues.push("The hyprctl command is not available.".to_string());
            }
            if settings.auto_reconnect && !program_available("grim") {
                issues.push("The grim command is required for Auto Reconnect.".to_string());
            }
        }
        InputMode::Kde => {
            if let Err(error) = crate::inputs::kde::script_bridge() {
                issues.push(error);
            }
            if std::env::var_os("WAYLAND_DISPLAY").is_none()
                && std::env::var("XDG_SESSION_TYPE").as_deref() != Ok("wayland")
            {
                issues.push("KDE Plasma 6 in a Wayland session is required.".to_string());
            }
            if find_qdbus().is_none() {
                issues.push("The qdbus6 or qdbus command is not available.".to_string());
            }
            if !command_works("journalctl", &["--version"]) {
                issues.push("The journalctl command is required for window detection.".to_string());
            }
            if settings.auto_reconnect && !program_available("spectacle") {
                issues.push("The spectacle command is required for Auto Reconnect.".to_string());
            }
        }
        InputMode::Niri => {
            if std::env::var_os("WAYLAND_DISPLAY").is_none() {
                issues.push("niri in a Wayland session is required.".to_string());
            }
            if std::env::var_os("NIRI_SOCKET").is_none() {
                issues.push("NIRI_SOCKET is not set. Start the app from a niri session.".to_string());
            }
            if !crate::inputs::niri::keyboard_ready() {
                issues.push(
                    "niri needs wtype for keyboard input: libinput ignores virtual uinput devices, so keys would never reach the game."
                        .to_string(),
                );
            }
            if settings.auto_reconnect && !supports_capture(mode) {
                issues.push("The grim command is required for Auto Reconnect.".to_string());
            }
            if settings.stealth && !crate::inputs::niri::hide_workspace_ready() {
                issues.push(
                    "Hide Game on Niri uses workspace antiafk-rbx-sober-hidden. Declare it in your Niri config or click Create in Diagnostics."
                        .to_string(),
                );
            }
        }
        InputMode::Cosmic => {
            if let Err(error) = crate::inputs::cosmic::probe() {
                issues.push(error);
            } else if !crate::inputs::cosmic::supports_activate_mode() {
                issues.push(
                    "The COSMIC compositor does not advertise window activation.".to_string(),
                );
            }
            if settings.auto_reconnect && !supports_capture(mode) {
                issues.push("grim or import is required for Auto Reconnect.".to_string());
            }
        }
        InputMode::Gnome => {
            if let Err(error) = crate::inputs::gnome::probe() {
                issues.push(error);
            }
            if settings.auto_reconnect && !supports_capture(mode) {
                issues.push(
                    "The GNOME extension is required for Auto Reconnect.".to_string(),
                );
            }
        }
        InputMode::X11 => {
            if std::env::var_os("DISPLAY").is_none() {
                issues.push("An X11 session (DISPLAY) is required.".to_string());
            }
            if !program_available("xdotool") {
                issues.push("The xdotool command is required for the X11 backend.".to_string());
            }
            if settings.auto_reconnect && !supports_capture(mode) {
                issues.push("Auto Reconnect requires maim, import or scrot.".to_string());
            }
        }
        InputMode::I3 => {
            if std::env::var_os("I3SOCK").is_none() {
                issues.push("An i3 session (I3SOCK) is required.".to_string());
            }
            if !program_available("xdotool") {
                issues.push("The xdotool command is required.".to_string());
            }
            if settings.auto_reconnect && !supports_capture(mode) {
                issues.push("Auto Reconnect requires maim, import or scrot.".to_string());
            }
        }
        InputMode::Unsupported => issues.push(
            "This desktop is not supported yet. Use Hyprland, KDE Plasma 6, Niri, COSMIC, GNOME, X11 or i3."
                .to_string(),
        ),
    }

    if mode.uses_uinput() && !has_uinput_access() {
        issues.push(
            "Access to /dev/uinput is denied. Run the permission fix in Settings.".to_string(),
        );
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues.join("\n"))
    }
}

pub fn start_backend(state: SharedState) {
    let state_auto = state.clone();
    thread::spawn(move || {
        let mut sober_absent_ticks = 0;
        loop {
            let settings = { state_auto.lock().unwrap().clone() };

            if check_sober_running() {
                sober_absent_ticks = 0;
                if settings.auto_start && !settings.running && !settings.manually_stopped {
                    match preflight(&settings) {
                        Ok(()) => {
                            state_auto.lock().unwrap().running = true;
                            set_runtime_status(&state_auto, RuntimeStatus::WaitingForSober);
                            notify("Sober detected. AntiAFK enabled.");
                        }
                        Err(error) => {
                            set_runtime_error(&state_auto, error.clone());
                            notify(&format!("Cannot auto-start: {error}"));
                        }
                    }
                }
            } else {
                sober_absent_ticks += 1;
                if sober_absent_ticks >= 6 {
                    let current_status = state_auto.lock().unwrap().runtime_status;
                    if current_status == RuntimeStatus::Error {
                    } else if settings.running {
                        {
                            let mut state = state_auto.lock().unwrap();
                            state.running = false;
                            state.manually_stopped = false;
                        }
                        restore_hidden_windows();
                        set_runtime_status(
                            &state_auto,
                            if settings.auto_start {
                                RuntimeStatus::WaitingForSober
                            } else {
                                RuntimeStatus::Stopped
                            },
                        );
                        notify("Sober closed. AntiAFK disabled.");
                    } else if settings.manually_stopped {
                        let mut state = state_auto.lock().unwrap();
                        state.manually_stopped = false;
                        let next_status = if state.auto_start {
                            RuntimeStatus::WaitingForSober
                        } else {
                            RuntimeStatus::Stopped
                        };
                        state.runtime_status = next_status;
                        state.error_message = None;
                    }
                }
            }
            thread::sleep(Duration::from_secs(1));
        }
    });

    let state_fps = state.clone();
    thread::spawn(move || {
        let mut active_scopes: HashMap<String, u32> = HashMap::new();
        loop {
            let (enabled, fps_limit, stop_on_focus, is_running, action_active, mode) = {
                let s = state_fps.lock().unwrap();
                (
                    s.fps_capper,
                    s.fps_limit,
                    s.stop_limit_on_focus,
                    s.running,
                    s.action_active,
                    detect_mode(),
                )
            };

            if enabled && is_running && fps_limit > 0 && !action_active && cpu_quota_supported(mode)
            {
                let main_pids = sober_pids();
                if main_pids.is_empty() {
                    reset_scopes(&mut active_scopes);
                    set_quota_report(CpuQuotaReport {
                        pids: 0,
                        scopes: Vec::new(),
                        limit: None,
                        focus: None,
                    });
                    thread::sleep(Duration::from_millis(500));
                    continue;
                }

                let mut current_target_scopes = HashSet::new();
                for pid in &main_pids {
                    if let Some(scope) = get_systemd_scope(pid)
                        && (scope.contains("app") || scope.contains("sober"))
                    {
                        current_target_scopes.insert(scope);
                    }
                }

                let focus = if stop_on_focus {
                    focused_is_sober()
                } else {
                    None
                };
                let unlocked = stop_on_focus && focus == Some(true);

                let quota = fps_limit.clamp(3, 99);

                active_scopes.retain(|s, _| {
                    if current_target_scopes.contains(s) && !unlocked {
                        true
                    } else {
                        set_cpu_limit(s, "");
                        false
                    }
                });

                if !unlocked {
                    for scope in &current_target_scopes {
                        if active_scopes.get(scope) != Some(&quota) {
                            set_cpu_limit(scope, &format!("{quota}%"));
                            active_scopes.insert(scope.clone(), quota);
                        }
                    }
                    let active_ms = (u64::from(quota)).clamp(10, 95);
                    let stop_ms = 100 - active_ms;
                    cont_pids(&main_pids);
                    thread::sleep(Duration::from_millis(active_ms));
                    stop_pids(&main_pids);
                    thread::sleep(Duration::from_millis(stop_ms));
                    cont_pids(&main_pids);
                } else {
                    reset_scopes(&mut active_scopes);
                    cont_pids(&main_pids);
                    thread::sleep(Duration::from_millis(500));
                }

                set_quota_report(CpuQuotaReport {
                    pids: main_pids.len(),
                    scopes: if current_target_scopes.is_empty() {
                        vec!["pid-throttle".to_string()]
                    } else {
                        current_target_scopes.iter().cloned().collect()
                    },
                    limit: if unlocked { None } else { Some(quota) },
                    focus,
                });
            } else {
                reset_scopes(&mut active_scopes);
                cont_pids(&sober_pids());
                if enabled {
                    set_quota_report(CpuQuotaReport {
                        pids: 0,
                        scopes: Vec::new(),
                        limit: None,
                        focus: if stop_on_focus {
                            focused_is_sober()
                        } else {
                            None
                        },
                    });
                } else {
                    set_quota_report(CpuQuotaReport::default());
                }
                thread::sleep(Duration::from_millis(500));
            }
        }
    });

    let state_main = state.clone();
    thread::spawn(move || {
        loop {
            let (is_running, mode) = {
                let mut s = state_main.lock().unwrap();
                let detected = detect_mode().as_legacy();
                if detected != s.mode {
                    s.mode = detected;
                }
                (s.running, s.mode)
            };

            if is_running {
                let res = match InputMode::from_legacy(mode) {
                    InputMode::Hyprland => crate::inputs::hyprland::run(&state_main),
                    InputMode::Kde => crate::inputs::kde::run(&state_main),
                    InputMode::Niri => crate::inputs::niri::run(&state_main),
                    InputMode::Cosmic => crate::inputs::cosmic::run(&state_main),
                    InputMode::Gnome => crate::inputs::gnome::run(&state_main),
                    InputMode::X11 | InputMode::I3 => crate::inputs::x11::run(&state_main),
                    InputMode::Unsupported => Err("This desktop is not supported yet.".to_string()),
                };

                if let Err(error) = res {
                    restore_hidden_windows();
                    set_runtime_error(&state_main, error.clone());
                    notify(&format!("Error: {error}"));
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}

fn notify(msg: &str) {
    let _ = Notification::new()
        .appname(APP_ID)
        .summary(APP_TITLE)
        .body(msg)
        .icon(APP_ID)
        .timeout(5000)
        .show();
}

pub fn focused_is_sober() -> Option<bool> {
    let mode = detect_mode();
    match mode {
        InputMode::Hyprland => hyprland_focused_is_sober(),
        InputMode::Kde => crate::inputs::kde::focused_is_sober(),
        InputMode::Niri => Some(crate::inputs::niri::focused_is_sober()),
        InputMode::Cosmic => Some(crate::inputs::cosmic::focused_is_sober()),
        InputMode::Gnome => Some(crate::inputs::gnome::focused_is_sober()),
        InputMode::X11 | InputMode::I3 => crate::inputs::x11::focused_is_sober(),
        InputMode::Unsupported => None,
    }
}

fn hyprland_focused_is_sober() -> Option<bool> {
    let out = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .ok()?;
    let json = serde_json::from_slice::<Value>(&out.stdout).ok()?;
    let class = json.get("class").and_then(|v| v.as_str()).unwrap_or("");
    let title = json.get("title").and_then(|v| v.as_str()).unwrap_or("");
    Some(crate::inputs::common::is_sober_metadata(
        None,
        Some(class),
        Some(title),
    ))
}

#[derive(Clone, Default)]
pub struct CpuQuotaReport {
    pub pids: usize,
    #[allow(dead_code)]
    pub scopes: Vec<String>,
    pub limit: Option<u32>,
    pub focus: Option<bool>,
}

static QUOTA_REPORT: RwLock<Option<CpuQuotaReport>> = RwLock::new(None);

fn set_quota_report(report: CpuQuotaReport) {
    if let Ok(mut slot) = QUOTA_REPORT.write() {
        *slot = Some(report);
    }
}

pub fn cpu_quota_report() -> CpuQuotaReport {
    QUOTA_REPORT
        .read()
        .ok()
        .and_then(|slot| slot.clone())
        .unwrap_or_default()
}

#[allow(dead_code)]
pub fn uinput_works() -> bool {
    has_uinput_access() && crate::input::create_keyboard_device().is_ok()
}

fn set_cpu_limit(scope: &str, limit: &str) {
    let val = if limit.is_empty() { "" } else { limit };
    let _ = Command::new("systemctl")
        .args(["--user", "set-property", scope, &format!("CPUQuota={val}")])
        .output();
}

fn reset_scopes(scopes: &mut HashMap<String, u32>) {
    for scope in scopes.keys() {
        set_cpu_limit(scope, "");
    }
    scopes.clear();
}

fn cont_pids(pids: &[String]) {
    for pid in pids {
        if let Ok(p) = pid.parse::<i32>() {
            unsafe {
                libc::kill(p, libc::SIGCONT);
            }
        }
    }
}

fn stop_pids(pids: &[String]) {
    for pid in pids {
        if let Ok(p) = pid.parse::<i32>() {
            unsafe {
                libc::kill(p, libc::SIGSTOP);
            }
        }
    }
}

fn check_sober_running() -> bool {
    !sober_pids().is_empty()
}

fn get_systemd_scope(pid: &str) -> Option<String> {
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
    for line in cgroup.lines() {
        if let Some(path) = line.split("::").nth(1) {
            for part in path.split('/').rev() {
                if part.ends_with(".scope") || part.ends_with(".service") {
                    return Some(part.to_string());
                }
            }
        }
    }
    None
}

pub fn sober_pids() -> Vec<String> {
    let me = std::process::id();
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return found;
    };
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == me {
            continue;
        }
        let cmdline = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        let text = String::from_utf8_lossy(&cmdline)
            .replace('\0', " ")
            .to_lowercase();
        if text.contains(SELF_MARKER) {
            continue;
        }
        if text.contains("sober") || text.contains("roblox") || text.contains("vinegar") {
            found.push(pid.to_string());
            continue;
        }
        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm"))
            .unwrap_or_default()
            .to_lowercase();
        if comm.contains("sober") || comm.contains("roblox") {
            found.push(pid.to_string());
        }
    }
    found
}
