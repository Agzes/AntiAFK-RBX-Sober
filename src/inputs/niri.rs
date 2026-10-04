use crate::environment::InputMode;
use crate::input::create_keyboard_device;
use crate::inputs::common::{
    capture_wayland_screen, click_pointer, command_json, command_output, create_pointer_device,
    is_sober_metadata, perform_keyboard_actions, reconnect_button, reconnect_pixel_match,
    responsive_sleep, responsive_sleep_interval, set_action_active, set_status, should_continue,
    warp_cursor, wayland_capture_available,
};
use crate::state::{APP_SLUG, RuntimeStatus, SharedState};
use image::GenericImageView;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const SCRATCHPAD: &str = "special";
pub const HIDDEN_WORKSPACE: &str = "antiafk-rbx-sober-hidden";
const USER_IDLE: Duration = Duration::from_secs(4);

#[derive(Clone, Debug, Deserialize)]
struct NiriLayout {
    #[serde(default)]
    window_size: Option<(i32, i32)>,
    #[serde(default)]
    tile_pos_in_workspace_view: Option<(f64, f64)>,
}

#[derive(Clone, Debug, Deserialize)]
struct NiriWindow {
    id: u64,
    title: Option<String>,
    app_id: Option<String>,
    #[serde(default)]
    pid: Option<i32>,
    #[serde(default)]
    workspace_id: Option<u64>,
    #[serde(default)]
    layout: Option<NiriLayout>,
}

#[derive(Debug, Deserialize)]
struct NiriWorkspace {
    id: u64,
    #[serde(default)]
    idx: u8,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    output: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct NiriLogical {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Deserialize)]
struct NiriOutput {
    #[serde(default)]
    logical: Option<NiriLogical>,
}

struct Layout {
    min_x: i32,
    min_y: i32,
    width: i32,
    height: i32,
    outputs: HashMap<String, NiriLogical>,
    workspace_outputs: HashMap<u64, String>,
}

struct Rect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Rect {
    fn center(&self) -> (i32, i32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }
}

static HIDDEN: OnceLock<Mutex<Vec<(u64, String)>>> = OnceLock::new();
static ACTIVITY: OnceLock<Mutex<Instant>> = OnceLock::new();
static LAST_INPUT: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn hidden() -> &'static Mutex<Vec<(u64, String)>> {
    HIDDEN.get_or_init(|| Mutex::new(Vec::new()))
}

fn activity() -> &'static Mutex<Instant> {
    ACTIVITY.get_or_init(|| Mutex::new(Instant::now()))
}

fn last_input() -> &'static Mutex<Option<String>> {
    LAST_INPUT.get_or_init(|| Mutex::new(None))
}

fn note_input(detail: impl Into<String>) {
    *last_input().lock().unwrap() = Some(detail.into());
}

#[allow(dead_code)]
pub fn last_key_sent() -> Option<String> {
    last_input().lock().unwrap().clone()
}

pub fn keyboard_ready() -> bool {
    create_keyboard_device().is_ok()
}

pub fn user_recently_interacted() -> bool {
    crate::inputs::common::has_recent_input(3) || activity().lock().unwrap().elapsed() < USER_IDLE
}

pub fn supports_hide() -> bool {
    command_output("niri", &["--version"]).is_ok()
}

pub fn supports_capture() -> bool {
    wayland_capture_available()
}

pub fn hide_workspace_ready() -> bool {
    workspaces()
        .unwrap_or_default()
        .iter()
        .any(|workspace| workspace.name.as_deref() == Some(HIDDEN_WORKSPACE))
}

#[allow(dead_code)]
pub fn limitations() -> String {
    "niri does not report pointer position directly; AntiAFK computes window geometry from layout and restores cursor to the previously active window.".to_string()
}

#[allow(dead_code)]
pub fn cursor_note() -> String {
    "niri does not report pointer position directly. AntiAFK restores cursor to the previously active window after actions.".to_string()
}

fn windows() -> Result<Vec<NiriWindow>, String> {
    let value = command_json("niri", &["msg", "--json", "windows"])?;
    serde_json::from_value(value).map_err(|error| format!("invalid niri window response: {error}"))
}

static FOCUSED_CACHE: Mutex<Option<(Instant, Option<NiriWindow>)>> = Mutex::new(None);

fn focused_window() -> Option<NiriWindow> {
    if let Ok(guard) = FOCUSED_CACHE.lock()
        && let Some((instant, ref win)) = *guard
        && instant.elapsed() < Duration::from_millis(400)
    {
        return win.clone();
    }
    let win = (|| {
        let value = command_json("niri", &["msg", "--json", "focused-window"]).ok()?;
        if value.is_null() {
            return None;
        }
        serde_json::from_value(value).ok()
    })();
    if let Ok(mut guard) = FOCUSED_CACHE.lock() {
        *guard = Some((Instant::now(), win.clone()));
    }
    win
}

fn workspaces() -> Result<Vec<NiriWorkspace>, String> {
    let value = command_json("niri", &["msg", "--json", "workspaces"])?;
    serde_json::from_value(value)
        .map_err(|error| format!("invalid niri workspace response: {error}"))
}

fn targets() -> Result<Vec<NiriWindow>, String> {
    Ok(windows()?.into_iter().filter(sober_window).collect())
}

fn sober_window(window: &NiriWindow) -> bool {
    window.pid != Some(std::process::id() as i32)
        && is_sober_metadata(window.app_id.as_deref(), None, window.title.as_deref())
}

pub fn window_summary() -> (usize, usize) {
    let all = windows().unwrap_or_default();
    let matched = targets().map(|found| found.len()).unwrap_or(0);
    (all.len(), matched)
}

fn focus_window(id: u64) -> Result<(), String> {
    command_output(
        "niri",
        &["msg", "action", "focus-window", "--id", &id.to_string()],
    )
    .map(|_| ())
}

fn workspace_of(id: Option<u64>) -> Option<String> {
    let workspace = workspaces()
        .ok()?
        .into_iter()
        .find(|workspace| Some(workspace.id) == id)?;
    workspace
        .name
        .filter(|name| !name.is_empty())
        .or_else(|| Some(workspace.idx.to_string()))
}

fn move_to_workspace(id: u64, workspace: &str, follow: bool) -> Result<(), String> {
    let follow = if follow { "true" } else { "false" };
    command_output(
        "niri",
        &[
            "msg",
            "action",
            "move-window-to-workspace",
            workspace,
            "--window-id",
            &id.to_string(),
            "--focus",
            follow,
        ],
    )
    .map(|_| ())
}

fn stash(id: u64) -> Result<(), String> {
    if move_to_workspace(id, HIDDEN_WORKSPACE, false).is_ok() {
        return Ok(());
    }
    if move_to_workspace(id, SCRATCHPAD, false).is_ok() {
        return Ok(());
    }
    let all_workspaces = workspaces().unwrap_or_default();
    let current_ws_id = windows().ok().and_then(|wins| {
        wins.into_iter()
            .find(|w| w.id == id)
            .and_then(|w| w.workspace_id)
    });
    for ws in &all_workspaces {
        if Some(ws.id) != current_ws_id {
            let idx_str = ws.idx.to_string();
            let target = ws.name.as_deref().unwrap_or(&idx_str);
            if move_to_workspace(id, target, false).is_ok() {
                return Ok(());
            }
        }
    }
    let max_idx = all_workspaces.iter().map(|w| w.idx).max().unwrap_or(1);
    let next_idx = (max_idx + 1).to_string();
    if move_to_workspace(id, &next_idx, false).is_ok() {
        return Ok(());
    }
    command_output(
        "niri",
        &[
            "msg",
            "action",
            "move-window-to-workspace-down",
            "--window-id",
            &id.to_string(),
        ],
    )
    .map(|_| ())
    .map_err(|err| format!("failed to hide window in niri: {err}"))
}

fn hidden_workspace(id: u64) -> Option<String> {
    hidden()
        .lock()
        .unwrap()
        .iter()
        .find(|(window, _)| *window == id)
        .map(|(_, workspace)| workspace.clone())
}

fn remember_hidden(id: u64, workspace: String) {
    let mut entries = hidden().lock().unwrap();
    if !entries.iter().any(|(window, _)| *window == id) {
        entries.push((id, workspace));
    }
}

fn forget_hidden(id: u64) {
    hidden().lock().unwrap().retain(|(window, _)| *window != id);
}

fn layout() -> Option<Layout> {
    let value = command_json("niri", &["msg", "--json", "outputs"]).ok()?;
    let outputs: HashMap<String, NiriOutput> = serde_json::from_value(value).ok()?;
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    let mut map = HashMap::new();

    for (name, output) in outputs {
        let Some(logical) = output.logical else {
            continue;
        };
        if logical.width <= 0 || logical.height <= 0 {
            continue;
        }
        min_x = min_x.min(logical.x);
        min_y = min_y.min(logical.y);
        max_x = max_x.max(logical.x + logical.width);
        max_y = max_y.max(logical.y + logical.height);
        map.insert(name, logical);
    }

    if map.is_empty() {
        return None;
    }
    Some(Layout {
        min_x,
        min_y,
        width: max_x - min_x,
        height: max_y - min_y,
        outputs: map,
        workspace_outputs: workspaces()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|workspace| workspace.output.map(|output| (workspace.id, output)))
            .collect(),
    })
}

fn window_rect(window: &NiriWindow, layout: &Layout) -> Rect {
    let output = window
        .workspace_id
        .and_then(|id| layout.workspace_outputs.get(&id).cloned())
        .and_then(|name| layout.outputs.get(&name).copied())
        .unwrap_or(NiriLogical {
            x: layout.min_x,
            y: layout.min_y,
            width: layout.width,
            height: layout.height,
        });

    let reported = window.layout.as_ref().and_then(|item| item.window_size);
    let (width, height) = match reported {
        Some((width, height)) if width > 0 && height > 0 => (width, height),
        _ => (output.width, output.height),
    };

    let tile = window
        .layout
        .as_ref()
        .and_then(|item| item.tile_pos_in_workspace_view);
    if let Some((tile_x, tile_y)) = tile {
        let inside = |x: i32, y: i32| {
            x >= output.x
                && y >= output.y
                && x < output.x + output.width
                && y < output.y + output.height
        };
        for (x, y) in [
            (output.x + tile_x as i32, output.y + tile_y as i32),
            (tile_x as i32, tile_y as i32),
        ] {
            if inside(x, y) {
                return Rect {
                    x,
                    y,
                    width,
                    height,
                };
            }
        }
    }

    Rect {
        x: output.x + (output.width - width) / 2,
        y: output.y + (output.height - height) / 2,
        width,
        height,
    }
}

fn to_virtual(rect_point: (i32, i32), layout: &Layout) -> (i32, i32) {
    (rect_point.0 - layout.min_x, rect_point.1 - layout.min_y)
}

fn watch_activity() {
    let _ = thread::Builder::new()
        .name(format!("{APP_SLUG}-niri-events"))
        .spawn(|| {
            loop {
                if stream_events().is_err() {
                    *activity().lock().unwrap() = Instant::now();
                }
                thread::sleep(Duration::from_millis(500));
            }
        });
}

fn stream_events() -> Result<(), String> {
    let Some(socket) = std::env::var_os("NIRI_SOCKET") else {
        return Err("NIRI_SOCKET is not set".to_string());
    };
    let mut stream = UnixStream::connect(socket).map_err(|error| error.to_string())?;
    stream
        .write_all(b"{\"EventStream\":{}}\n")
        .map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;

    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else {
            return Err("niri event stream closed".to_string());
        };
        if line.contains("WindowFocusChanged")
            || line.contains("WindowOpenedOrChanged")
            || line.contains("WindowClosed")
            || line.contains("WorkspaceActivated")
        {
            *activity().lock().unwrap() = Instant::now();
        }
    }
    Ok(())
}

pub fn run(state: &SharedState) -> Result<(), String> {
    if std::env::var_os("NIRI_SOCKET").is_none() {
        return Err("Niri mode requires Niri environment.".to_string());
    }
    if windows().is_err() {
        return Err("The niri IPC socket is unavailable or incompatible.".to_string());
    }

    watch_activity();
    let mut keyboard = create_keyboard_device()?;
    let mut pointer = create_pointer_device()?;
    thread::sleep(Duration::from_millis(500));

    loop {
        if !should_continue(state, InputMode::Niri) {
            break;
        }
        let settings = { state.lock().unwrap().clone() };

        if settings.user_safe && user_recently_interacted() {
            set_status(state, RuntimeStatus::Paused);
            if responsive_sleep(state, InputMode::Niri, 5) {
                break;
            }
            continue;
        }

        let Some(layout) = layout() else {
            thread::sleep(Duration::from_secs(2));
            continue;
        };

        let mut found = targets()?;
        if !settings.multi_instance {
            found.truncate(1);
        }
        if found.is_empty() {
            set_status(state, RuntimeStatus::WaitingForSober);
            if responsive_sleep(state, InputMode::Niri, 2) {
                break;
            }
            continue;
        }

        set_action_active(state, true);
        set_status(state, RuntimeStatus::PerformingAction);
        thread::sleep(Duration::from_millis(500));

        let previous = focused_window();
        let prev_id = previous.as_ref().map(|window| window.id);
        let prev_center = previous
            .as_ref()
            .map(|p| to_virtual(window_rect(p, &layout).center(), &layout));
        let prev_is_target = previous
            .as_ref()
            .map(|p| found.iter().any(|t| t.id == p.id))
            .unwrap_or(false);
        let mut failure = None;

        for window in &found {
            let rect = window_rect(window, &layout);

            if settings.stealth {
                let _ = restore_hidden(window);
            }
            if let Err(error) = focus_window(window.id) {
                failure = Some(error);
                break;
            }
            let _ = command_output("niri", &["msg", "action", "center-column"]);
            thread::sleep(Duration::from_millis(150));

            let (center_x, center_y) = to_virtual(rect.center(), &layout);
            warp_cursor(
                &mut pointer,
                center_x,
                center_y,
                layout.width,
                layout.height,
            );
            thread::sleep(Duration::from_millis(150));
            click_pointer(&mut pointer, Duration::from_millis(30));
            thread::sleep(Duration::from_millis(50));

            if let Err(error) = perform_keyboard_actions(&mut keyboard, &settings) {
                failure = Some(error);
                break;
            }
            note_input("uinput ok");

            if settings.auto_reconnect {
                set_status(state, RuntimeStatus::ReconnectChecking);
                let tmp_path = std::env::temp_dir().join(format!("{APP_SLUG}-niri-recon.png"));
                let _ = std::fs::remove_file(&tmp_path);

                let geom_arg = format!("{},{} {}x{}", rect.x, rect.y, rect.width, rect.height);
                let capture_status = std::process::Command::new("grim")
                    .args(["-g", &geom_arg, tmp_path.to_str().unwrap_or("")])
                    .status();

                let img = match capture_status {
                    Ok(status) if status.success() => image::open(&tmp_path).ok(),
                    _ => {
                        if capture_wayland_screen(&tmp_path).is_ok() {
                            image::open(&tmp_path).ok()
                        } else {
                            None
                        }
                    }
                };
                let _ = std::fs::remove_file(&tmp_path);

                if let Some(img) = img {
                    let (img_w, img_h) = (img.width(), img.height());
                    if img_w >= 50 && img_h >= 50 {
                        let mut found = false;
                        let cx = (img_w / 2) as i32;
                        let cy = (img_h / 2) as i32;
                        for dy in -40..=40 {
                            for dx in -40..=40 {
                                let px = (cx + dx).clamp(0, img_w as i32 - 1) as u32;
                                let py = (cy + dy).clamp(0, img_h as i32 - 1) as u32;
                                let p = img.get_pixel(px, py);
                                if reconnect_pixel_match(p[0], p[1], p[2]) {
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                break;
                            }
                        }

                        if found {
                            set_status(state, RuntimeStatus::ReconnectFound);
                            thread::sleep(Duration::from_millis(150));
                            let btn = reconnect_button(rect.x, rect.y, rect.width, rect.height);
                            let (btn_vx, btn_vy) = to_virtual(btn, &layout);
                            warp_cursor(&mut pointer, btn_vx, btn_vy, layout.width, layout.height);
                            thread::sleep(Duration::from_millis(100));
                            set_status(state, RuntimeStatus::Reconnecting);
                            for _ in 0..3 {
                                click_pointer(&mut pointer, Duration::from_millis(30));
                                thread::sleep(Duration::from_millis(50));
                            }
                            warp_cursor(
                                &mut pointer,
                                center_x,
                                center_y,
                                layout.width,
                                layout.height,
                            );
                        } else {
                            set_status(state, RuntimeStatus::ReconnectNotFound);
                            thread::sleep(Duration::from_millis(150));
                        }
                    } else {
                        set_status(state, RuntimeStatus::ReconnectNotFound);
                        thread::sleep(Duration::from_millis(150));
                    }
                } else {
                    set_status(state, RuntimeStatus::ReconnectNotFound);
                    thread::sleep(Duration::from_millis(150));
                }
            }

            if settings.stealth
                && let Err(error) = hide(window)
            {
                failure = Some(error);
                break;
            }
        }

        if let Some(prev_id) = prev_id
            && !prev_is_target
        {
            let _ = focus_window(prev_id);
            thread::sleep(Duration::from_millis(100));
            if let Some((pcx, pcy)) = prev_center {
                warp_cursor(&mut pointer, pcx, pcy, layout.width, layout.height);
            }
        } else if settings.stealth {
            if let Ok(wins) = windows()
                && let Some(other) = wins.into_iter().find(|w| !sober_window(w))
            {
                let _ = focus_window(other.id);
            }
        }
        set_action_active(state, false);

        if let Some(error) = failure {
            set_status(state, RuntimeStatus::Stopped);
            return Err(error);
        }

        set_status(state, RuntimeStatus::Ready);
        if responsive_sleep_interval(state, InputMode::Niri) {
            break;
        }
    }

    set_action_active(state, false);
    set_status(state, RuntimeStatus::Stopped);
    Ok(())
}

fn restore_hidden(window: &NiriWindow) -> Result<(), String> {
    match hidden_workspace(window.id) {
        Some(workspace) => move_to_workspace(window.id, &workspace, true),
        None => Ok(()),
    }
}

fn hide(window: &NiriWindow) -> Result<(), String> {
    let workspace = workspace_of(window.workspace_id);
    stash(window.id)?;
    if let Some(workspace) = workspace {
        remember_hidden(window.id, workspace);
    }
    Ok(())
}

pub fn focused_is_sober() -> bool {
    let Some(window) = focused_window() else {
        return false;
    };
    sober_window(&window)
}

pub fn show_sober() -> Result<(), String> {
    let window = first_target()?;
    restore_hidden(&window)?;
    forget_hidden(window.id);
    focus_window(window.id)
}

pub fn hide_sober() -> Result<(), String> {
    let window = first_target()?;
    if let Some(workspace) = hidden_workspace(window.id) {
        move_to_workspace(window.id, &workspace, false)?;
        forget_hidden(window.id);
        return Ok(());
    }
    let workspace = workspace_of(window.workspace_id)
        .ok_or_else(|| "Niri did not report a workspace for the Sober window".to_string())?;
    stash(window.id)?;
    remember_hidden(window.id, workspace);
    Ok(())
}

fn first_target() -> Result<NiriWindow, String> {
    targets()?
        .into_iter()
        .next()
        .ok_or_else(|| "Sober window was not found.".to_string())
}

pub fn restore_hidden_windows() {
    let entries = {
        let mut entries = hidden().lock().unwrap();
        std::mem::take(&mut *entries)
    };
    for (id, workspace) in entries {
        let _ = move_to_workspace(id, &workspace, false);
    }
}
