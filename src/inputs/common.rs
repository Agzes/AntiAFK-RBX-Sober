use crate::environment::InputMode;
use crate::input::{emit_key, tap_key};
use crate::state::{
    ABSOLUTE_MOUSE_DEVICE, AppState, POINTER_DEVICE, RuntimeStatus, SELF_MARKER, SharedState,
    set_runtime_status,
};
use evdev::{
    AbsInfo, AbsoluteAxisCode, EventType, InputEvent, KeyCode, UinputAbsSetup,
    uinput::VirtualDevice,
};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static ACTIVITY_INIT: AtomicBool = AtomicBool::new(false);
static LAST_INPUT_EVENT_TIME: AtomicU64 = AtomicU64::new(0);
static LAST_INPUT_WAS_KEY: AtomicBool = AtomicBool::new(false);

fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn record_user_activity() {
    LAST_INPUT_EVENT_TIME.store(current_time_millis(), Ordering::Relaxed);
}

pub fn record_user_activity_with_type(is_key: bool) {
    LAST_INPUT_WAS_KEY.store(is_key, Ordering::Relaxed);
    record_user_activity();
}

pub fn recent_input_is_keyboard() -> bool {
    LAST_INPUT_WAS_KEY.load(Ordering::Relaxed)
}

pub fn init_input_listener() {
    if ACTIVITY_INIT.swap(true, Ordering::SeqCst) {
        return;
    }
    thread::Builder::new()
        .name("antiafk-input-listener".to_string())
        .spawn(|| {
            let mut devices = Vec::new();
            if let Ok(entries) = std::fs::read_dir("/dev/input") {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let name_str = file_name.to_string_lossy();
                    if name_str.starts_with("event")
                        && let Ok(dev) = evdev::Device::open(entry.path())
                    {
                        let dev_name = dev.name().unwrap_or("").to_string();
                        if !dev_name.contains("AntiAFK")
                            && !dev_name.contains(POINTER_DEVICE)
                            && !dev_name.contains(ABSOLUTE_MOUSE_DEVICE)
                        {
                            devices.push(dev);
                        }
                    }
                }
            }
            if devices.is_empty() {
                return;
            }
            let mut poll_fds: Vec<libc::pollfd> = devices
                .iter()
                .map(|d| libc::pollfd {
                    fd: d.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                })
                .collect();

            loop {
                for pfd in &mut poll_fds {
                    pfd.revents = 0;
                }
                let ret = unsafe {
                    libc::poll(poll_fds.as_mut_ptr(), poll_fds.len() as libc::nfds_t, 500)
                };
                if ret > 0 {
                    let mut had_activity = false;
                    let mut is_key = false;
                    for (i, pfd) in poll_fds.iter().enumerate() {
                        if (pfd.revents & libc::POLLIN) != 0
                            && let Some(dev) = devices.get_mut(i)
                            && let Ok(events) = dev.fetch_events()
                        {
                            for ev in events {
                                if ev.event_type() == EventType::KEY {
                                    had_activity = true;
                                    is_key = true;
                                    break;
                                } else if ev.event_type() == EventType::RELATIVE {
                                    had_activity = true;
                                    is_key = false;
                                    break;
                                }
                            }
                        }
                    }
                    if had_activity {
                        record_user_activity_with_type(is_key);
                    }
                }
            }
        })
        .ok();
}

pub fn has_recent_input(secs: u64) -> bool {
    init_input_listener();
    let last = LAST_INPUT_EVENT_TIME.load(Ordering::Relaxed);
    if last == 0 {
        return false;
    }
    let now = current_time_millis();
    now.saturating_sub(last) < (secs * 1000)
}

const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

pub const POINTER_ABS_MAX: i32 = 65535;

pub fn create_pointer_device() -> Result<VirtualDevice, String> {
    let abs_x = UinputAbsSetup::new(
        AbsoluteAxisCode::ABS_X,
        AbsInfo::new(0, 0, POINTER_ABS_MAX, 0, 0, 0),
    );
    let abs_y = UinputAbsSetup::new(
        AbsoluteAxisCode::ABS_Y,
        AbsInfo::new(0, 0, POINTER_ABS_MAX, 0, 0, 0),
    );
    let mut keys = evdev::AttributeSet::<KeyCode>::new();
    keys.insert(KeyCode::BTN_LEFT);
    let mut relative = evdev::AttributeSet::<evdev::RelativeAxisCode>::new();
    relative.insert(evdev::RelativeAxisCode::REL_X);
    relative.insert(evdev::RelativeAxisCode::REL_Y);

    VirtualDevice::builder()
        .map_err(|error: std::io::Error| error.to_string())?
        .name(POINTER_DEVICE)
        .with_keys(&keys)
        .map_err(|error| error.to_string())?
        .with_relative_axes(&relative)
        .map_err(|error| error.to_string())?
        .with_absolute_axis(&abs_x)
        .map_err(|error| error.to_string())?
        .with_absolute_axis(&abs_y)
        .map_err(|error| error.to_string())?
        .build()
        .map_err(|error| {
            format!("Pointer device creation failed: {error}. Run: sudo chmod 666 /dev/uinput")
        })
}

pub fn warp_cursor(pointer: &mut VirtualDevice, x: i32, y: i32, screen_w: i32, screen_h: i32) {
    if screen_w <= 0 || screen_h <= 0 {
        return;
    }
    let abs_x =
        (x as i64 * POINTER_ABS_MAX as i64 / screen_w as i64).clamp(0, POINTER_ABS_MAX as i64);
    let abs_y =
        (y as i64 * POINTER_ABS_MAX as i64 / screen_h as i64).clamp(0, POINTER_ABS_MAX as i64);
    let _ = pointer.emit(&[
        InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_X.0,
            abs_x as i32,
        ),
        InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_Y.0,
            abs_y as i32,
        ),
        InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
    ]);
}

pub fn click_pointer(pointer: &mut VirtualDevice, hold: Duration) {
    let _ = pointer.emit(&[
        InputEvent::new(EventType::KEY.0, KeyCode::BTN_LEFT.0, 1),
        InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
    ]);
    thread::sleep(hold);
    let _ = pointer.emit(&[
        InputEvent::new(EventType::KEY.0, KeyCode::BTN_LEFT.0, 0),
        InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
    ]);
}

pub const RECONNECT_RGB: (u8, u8, u8) = (57, 59, 61);
pub const RECONNECT_TOLERANCE: i16 = 25;

pub fn reconnect_pixel_match(r: u8, g: u8, b: u8) -> bool {
    let diff = (i16::from(r) - RECONNECT_RGB.0 as i16).abs()
        + (i16::from(g) - RECONNECT_RGB.1 as i16).abs()
        + (i16::from(b) - RECONNECT_RGB.2 as i16).abs();
    diff < RECONNECT_TOLERANCE
        || ((35..=65).contains(&r)
            && (i16::from(r) - i16::from(g)).abs() <= 5
            && (i16::from(g) - i16::from(b)).abs() <= 5)
}

pub fn reconnect_probe(x: i32, y: i32, width: i32, height: i32) -> (i32, i32) {
    (x + (width - 400) / 2 + 10, y + (height - 250) / 2 + 10)
}

pub fn reconnect_button(x: i32, y: i32, width: i32, height: i32) -> (i32, i32) {
    (
        x + (width - 400) / 2 + (400 - 161 - 27) + (161 / 2),
        y + (height - 250) / 2 + (250 - 34 - 21) + (34 / 2) + 6,
    )
}

pub fn find_qdbus() -> Option<String> {
    for program in ["qdbus6", "qdbus"] {
        let available = Command::new(program)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        if available {
            return Some(program.to_string());
        }
    }
    None
}

pub fn command_output(program: &str, args: &[&str]) -> Result<String, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to execute {program}: {error}"))?;
    let deadline = Instant::now() + COMMAND_TIMEOUT;

    loop {
        if child
            .try_wait()
            .map_err(|error| format!("failed waiting for {program}: {error}"))?
            .is_some()
        {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("{program} timed out after 5 seconds"));
        }
        thread::sleep(Duration::from_millis(10));
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed reading {program} output: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn command_json(program: &str, args: &[&str]) -> Result<serde_json::Value, String> {
    let output = command_output(program, args)?;
    serde_json::from_str(&output).map_err(|error| format!("invalid JSON from {program}: {error}"))
}

pub fn is_sober_metadata(app_id: Option<&str>, class: Option<&str>, title: Option<&str>) -> bool {
    let values = [app_id, class, title]
        .into_iter()
        .flatten()
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();

    values.iter().any(|value| {
        value.contains("sober") || value.contains("roblox") || value.contains("org.vinegarhq")
    }) && !values.iter().any(|value| value.contains(SELF_MARKER))
}

pub fn set_action_active(state: &SharedState, active: bool) {
    let mut state = state.lock().unwrap();
    state.action_active = active;
}

pub fn set_status(state: &SharedState, status: RuntimeStatus) {
    set_runtime_status(state, status);
}

pub fn should_continue(state: &SharedState, mode: InputMode) -> bool {
    let state = state.lock().unwrap();
    state.running && state.input_mode() == mode
}

pub fn responsive_sleep(state: &SharedState, mode: InputMode, seconds: u64) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(seconds);
    while std::time::Instant::now() < deadline {
        if !should_continue(state, mode) {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    !should_continue(state, mode)
}

pub fn responsive_sleep_interval(state: &SharedState, mode: InputMode) -> bool {
    let seconds = state.lock().unwrap().interval_seq;
    responsive_sleep(state, mode, seconds)
}

pub fn perform_keyboard_actions(
    keyboard: &mut VirtualDevice,
    settings: &AppState,
) -> Result<(), String> {
    perform_keyboard_actions_with_durations(
        keyboard,
        settings,
        Duration::from_millis(30),
        Duration::from_millis(150),
        Duration::from_millis(30),
    )
}

pub fn perform_keyboard_actions_with_durations(
    keyboard: &mut VirtualDevice,
    settings: &AppState,
    jump_duration: Duration,
    walk_duration: Duration,
    spin_duration: Duration,
) -> Result<(), String> {
    if settings.jump {
        tap_key(keyboard, KeyCode::KEY_SPACE, jump_duration)
            .map_err(|error| format!("failed to send jump key: {error}"))?;
    }

    if settings.walk {
        tap_key(keyboard, KeyCode::KEY_W, walk_duration)
            .map_err(|error| format!("failed to send walk key: {error}"))?;
        thread::sleep(Duration::from_millis(50));
        tap_key(keyboard, KeyCode::KEY_S, walk_duration)
            .map_err(|error| format!("failed to send walk key: {error}"))?;
    }

    if settings.spin_jiggle {
        tap_key(keyboard, KeyCode::KEY_I, spin_duration)
            .map_err(|error| format!("failed to send camera key: {error}"))?;
        thread::sleep(Duration::from_millis(50));
        tap_key(keyboard, KeyCode::KEY_O, spin_duration)
            .map_err(|error| format!("failed to send camera key: {error}"))?;
    }

    Ok(())
}

pub fn click_mouse(mouse: &mut VirtualDevice) -> Result<(), String> {
    emit_key(mouse, KeyCode::BTN_LEFT, true)
        .map_err(|error| format!("failed to press mouse button: {error}"))?;
    thread::sleep(Duration::from_millis(30));
    emit_key(mouse, KeyCode::BTN_LEFT, false)
        .map_err(|error| format!("failed to release mouse button: {error}"))
}

pub const ABSOLUTE_MOUSE_MAX: i32 = 32767;

pub fn create_absolute_mouse_device() -> Result<VirtualDevice, String> {
    let abs_x = UinputAbsSetup::new(
        AbsoluteAxisCode::ABS_X,
        AbsInfo::new(0, 0, ABSOLUTE_MOUSE_MAX, 0, 0, 0),
    );
    let abs_y = UinputAbsSetup::new(
        AbsoluteAxisCode::ABS_Y,
        AbsInfo::new(0, 0, ABSOLUTE_MOUSE_MAX, 0, 0, 0),
    );
    let mut keys = evdev::AttributeSet::<KeyCode>::new();
    keys.insert(KeyCode::BTN_LEFT);
    let mut relative = evdev::AttributeSet::<evdev::RelativeAxisCode>::new();
    relative.insert(evdev::RelativeAxisCode::REL_X);
    relative.insert(evdev::RelativeAxisCode::REL_Y);

    VirtualDevice::builder()
        .map_err(|error| error.to_string())?
        .name(ABSOLUTE_MOUSE_DEVICE)
        .with_keys(&keys)
        .map_err(|error| error.to_string())?
        .with_relative_axes(&relative)
        .map_err(|error| error.to_string())?
        .with_absolute_axis(&abs_x)
        .map_err(|error| error.to_string())?
        .with_absolute_axis(&abs_y)
        .map_err(|error| error.to_string())?
        .build()
        .map_err(|error| format!("absolute pointer creation failed: {error}"))
}

pub fn move_absolute_mouse(
    mouse: &mut VirtualDevice,
    x: i32,
    y: i32,
    screen_width: i32,
    screen_height: i32,
) -> Result<(), String> {
    if screen_width <= 0 || screen_height <= 0 {
        return Err("invalid screen bounds for absolute pointer".to_string());
    }
    let abs_x = (x as i64 * ABSOLUTE_MOUSE_MAX as i64 / screen_width as i64)
        .clamp(0, ABSOLUTE_MOUSE_MAX as i64) as i32;
    let abs_y = (y as i64 * ABSOLUTE_MOUSE_MAX as i64 / screen_height as i64)
        .clamp(0, ABSOLUTE_MOUSE_MAX as i64) as i32;
    mouse
        .emit(&[
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, abs_x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, abs_y),
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ])
        .map_err(|error| format!("failed to move absolute pointer: {error}"))
}

pub fn click_absolute_mouse(
    mouse: &mut VirtualDevice,
    x: i32,
    y: i32,
    screen_width: i32,
    screen_height: i32,
) -> Result<(), String> {
    move_absolute_mouse(mouse, x, y, screen_width, screen_height)?;
    click_mouse(mouse)
}

pub fn program_available(program: &str) -> bool {
    let program_path = Path::new(program);
    if program_path.is_absolute() || program_path.components().count() > 1 {
        return program_path.is_file();
    }
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| directory.join(program).is_file())
    })
}

pub fn capture_wayland_screen(path: &Path) -> Result<(), String> {
    let path_text = path
        .to_str()
        .ok_or_else(|| "screenshot path is not valid UTF-8".to_string())?;
    let candidates: [(&str, Vec<&str>); 2] = [
        ("grim", vec![path_text]),
        ("gnome-screenshot", vec!["-f", path_text]),
    ];
    for (program, args) in candidates {
        if program_available(program) && command_output(program, &args).is_ok() && path.exists() {
            return Ok(());
        }
    }
    Err("no supported Wayland screenshot utility (grim or gnome-screenshot)".to_string())
}

pub fn wayland_capture_available() -> bool {
    ["grim", "gnome-screenshot"]
        .into_iter()
        .any(program_available)
}

#[cfg(test)]
mod tests {
    use super::is_sober_metadata;

    #[test]
    fn matches_sober_metadata_but_not_our_window() {
        assert!(is_sober_metadata(
            Some("org.vinegarhq.Sober"),
            Some("sober"),
            Some("Roblox")
        ));
        assert!(!is_sober_metadata(
            Some("org.gnome.Shel"),
            Some("AntiAFK"),
            Some("AntiAFK - Sober")
        ));
    }
}
