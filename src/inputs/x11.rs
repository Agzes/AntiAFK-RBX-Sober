use crate::environment::InputMode;
use crate::input::create_keyboard_device;
use crate::inputs::common::{
    command_output, is_sober_metadata, perform_keyboard_actions, reconnect_pixel_match,
    responsive_sleep, set_action_active, set_status, should_continue,
};
use crate::state::{APP_SLUG, RuntimeStatus, SharedState};
use evdev::uinput::VirtualDevice;
use image::GenericImageView;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct X11Window {
    pub id: String,
    pub class: String,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

fn has_command(program: &str) -> bool {
    crate::inputs::common::program_available(program)
}

fn parse_i3_node(node: &serde_json::Value, windows: &mut Vec<X11Window>) {
    if let Some(xid) = node.get("window").and_then(|v| v.as_i64())
        && xid > 0
    {
        let props = node.get("window_properties");
        let class = props
            .and_then(|p| p.get("class"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let title = props
            .and_then(|p| p.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let rect = node.get("rect");
        let x = rect
            .and_then(|r| r.get("x"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let y = rect
            .and_then(|r| r.get("y"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let width = rect
            .and_then(|r| r.get("width"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let height = rect
            .and_then(|r| r.get("height"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        windows.push(X11Window {
            id: xid.to_string(),
            class,
            title,
            x,
            y,
            width,
            height,
        });
    }
    if let Some(nodes) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in nodes {
            parse_i3_node(child, windows);
        }
    }
    if let Some(floating) = node.get("floating_nodes").and_then(|v| v.as_array()) {
        for child in floating {
            parse_i3_node(child, windows);
        }
    }
}

fn i3_tree_windows() -> Result<Vec<X11Window>, String> {
    if !has_command("i3-msg") || std::env::var_os("I3SOCK").is_none() {
        return Err("not an i3 session".to_string());
    }
    let output = command_output("i3-msg", &["-t", "get_tree"])?;
    let tree: serde_json::Value = serde_json::from_str(&output).map_err(|e| e.to_string())?;
    let mut windows = Vec::new();
    parse_i3_node(&tree, &mut windows);
    Ok(windows)
}

fn i3_focused_window() -> Option<X11Window> {
    if !has_command("i3-msg") || std::env::var_os("I3SOCK").is_none() {
        return None;
    }
    let output = command_output("i3-msg", &["-t", "get_tree"]).ok()?;
    let tree: serde_json::Value = serde_json::from_str(&output).ok()?;
    find_focused_i3_node(&tree)
}

fn find_focused_i3_node(node: &serde_json::Value) -> Option<X11Window> {
    if node
        .get("focused")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let mut wins = Vec::new();
        parse_i3_node(node, &mut wins);
        return wins.into_iter().next();
    }
    if let Some(nodes) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in nodes {
            if let Some(found) = find_focused_i3_node(child) {
                return Some(found);
            }
        }
    }
    if let Some(floating) = node.get("floating_nodes").and_then(|v| v.as_array()) {
        for child in floating {
            if let Some(found) = find_focused_i3_node(child) {
                return Some(found);
            }
        }
    }
    None
}

fn wmctrl_windows() -> Result<Vec<X11Window>, String> {
    let output = command_output("wmctrl", &["-lx"])?;
    let mut windows = Vec::new();
    for line in output.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 4 {
            continue;
        }
        let id = fields[0].to_string();
        let class = fields[3].to_string();
        let title = fields[4..].join(" ");
        if let Ok(mut window) = geometry(&id) {
            window.class = class;
            window.title = title;
            windows.push(window);
        }
    }
    Ok(windows)
}

fn xdotool_windows() -> Result<Vec<X11Window>, String> {
    let mut ids: Vec<String> = Vec::new();
    for query in ["sober", "roblox", "vinegar"] {
        if let Ok(out) = command_output("xdotool", &["search", "--class", query]) {
            for id in out.lines().map(str::trim).filter(|s| !s.is_empty()) {
                if !ids.contains(&id.to_string()) {
                    ids.push(id.to_string());
                }
            }
        }
        if let Ok(out) = command_output("xdotool", &["search", "--name", query]) {
            for id in out.lines().map(str::trim).filter(|s| !s.is_empty()) {
                if !ids.contains(&id.to_string()) {
                    ids.push(id.to_string());
                }
            }
        }
    }
    if ids.is_empty()
        && let Ok(out) = command_output("xdotool", &["search", "--name", ".*"])
    {
        for id in out.lines().map(str::trim).filter(|s| !s.is_empty()) {
            if !ids.contains(&id.to_string()) {
                ids.push(id.to_string());
            }
        }
    }

    let mut windows = Vec::new();
    for id in &ids {
        let class = Command::new("xprop")
            .args(["-id", id, "WM_CLASS"])
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
            .map(|value| {
                value
                    .split("= \"")
                    .nth(1)
                    .and_then(|value| value.split('"').next())
                    .unwrap_or_default()
                    .to_string()
            })
            .unwrap_or_default();
        let title = Command::new("xdotool")
            .args(["getwindowname", id])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_default();
        if let Ok(mut window) = geometry(id) {
            window.class = class;
            window.title = title;
            windows.push(window);
        } else {
            windows.push(X11Window {
                id: id.clone(),
                class,
                title,
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            });
        }
    }
    Ok(windows)
}

fn geometry(id: &str) -> Result<X11Window, String> {
    let output = command_output("xdotool", &["getwindowgeometry", "--shell", id])?;
    let mut x = 0;
    let mut y = 0;
    let mut width = 0;
    let mut height = 0;
    for line in output.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.parse::<i32>().unwrap_or(0);
        match key {
            "X" => x = value,
            "Y" => y = value,
            "WIDTH" => width = value,
            "HEIGHT" => height = value,
            _ => {}
        }
    }
    if width <= 0 || height <= 0 {
        return Err(format!("invalid geometry for X11 window {id}"));
    }
    Ok(X11Window {
        id: id.to_string(),
        class: String::new(),
        title: String::new(),
        x,
        y,
        width,
        height,
    })
}

fn list_windows() -> Result<Vec<X11Window>, String> {
    if std::env::var_os("I3SOCK").is_some()
        && let Ok(windows) = i3_tree_windows()
        && !windows.is_empty()
    {
        return Ok(windows);
    }
    wmctrl_windows().or_else(|_| xdotool_windows())
}

fn cursor_position() -> Result<(i32, i32), String> {
    let output = command_output("xdotool", &["getmouselocation", "--shell"])?;
    let mut x = 0;
    let mut y = 0;
    for line in output.lines() {
        if let Some(value) = line.strip_prefix("X=") {
            x = value.parse().unwrap_or(x);
        }
        if let Some(value) = line.strip_prefix("Y=") {
            y = value.parse().unwrap_or(y);
        }
    }
    Ok((x, y))
}

fn move_cursor(x: i32, y: i32) -> Result<(), String> {
    command_output(
        "xdotool",
        &["mousemove", "--sync", &x.to_string(), &y.to_string()],
    )
    .map(|_| ())
}

fn activate(id: &str) -> Result<(), String> {
    if has_command("i3-msg") && std::env::var_os("I3SOCK").is_some() {
        let _ = Command::new("i3-msg")
            .arg(format!("[id={id}] scratchpad show"))
            .output();
        let selector = format!("[id={id}] focus");
        if Command::new("i3-msg")
            .arg(&selector)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            return Ok(());
        }
    }
    command_output("xdotool", &["windowactivate", "--sync", id]).map(|_| ())
}

fn hide(id: &str) -> Result<(), String> {
    if has_command("i3-msg") && std::env::var_os("I3SOCK").is_some() {
        let selector = format!("[id={id}] move scratchpad");
        if Command::new("i3-msg")
            .arg(&selector)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            return Ok(());
        }
    }
    command_output("xdotool", &["windowminimize", id]).map(|_| ())
}

fn active_window() -> Result<Option<String>, String> {
    let id = command_output("xdotool", &["getactivewindow"])?;
    let id = id.trim();
    if id.is_empty() || id == "0x0" {
        Ok(None)
    } else {
        Ok(Some(id.to_string()))
    }
}

fn window_class(id: &str) -> String {
    let Ok(output) = Command::new("xprop").args(["-id", id, "WM_CLASS"]).output() else {
        return String::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.split('"')
        .enumerate()
        .filter_map(|(i, s)| if i % 2 == 1 { Some(s) } else { None })
        .collect::<Vec<_>>()
        .join(" ")
}

fn window_name(id: &str) -> String {
    Command::new("xdotool")
        .args(["getwindowname", id])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_default()
}

fn user_active() -> Result<bool, String> {
    if crate::inputs::common::has_recent_input(3) {
        return Ok(true);
    }
    let before = cursor_position()?;
    thread::sleep(Duration::from_secs(3));
    let after = cursor_position()?;
    Ok((before.0 - after.0).abs() > 2 || (before.1 - after.1).abs() > 2)
}

fn tap_x11(key: &str, duration: Duration) -> Result<(), String> {
    command_output("xdotool", &["keydown", key]).map(|_| ())?;
    thread::sleep(duration);
    command_output("xdotool", &["keyup", key]).map(|_| ())
}

fn perform_actions(
    keyboard: &mut Option<VirtualDevice>,
    settings: &crate::state::AppState,
) -> Result<(), String> {
    match keyboard.as_mut() {
        Some(keyboard) => perform_keyboard_actions(keyboard, settings),
        None => {
            if settings.jump {
                tap_x11("space", Duration::from_millis(30))?;
            }
            if settings.walk {
                tap_x11("w", Duration::from_millis(150))?;
                thread::sleep(Duration::from_millis(50));
                tap_x11("s", Duration::from_millis(150))?;
            }
            if settings.spin_jiggle {
                tap_x11("i", Duration::from_millis(30))?;
                thread::sleep(Duration::from_millis(50));
                tap_x11("o", Duration::from_millis(30))?;
            }
            Ok(())
        }
    }
}

pub fn direct_input_available() -> bool {
    has_command("xdotool")
}

fn perform_direct_actions(id: &str, settings: &crate::state::AppState) -> Result<(), String> {
    if settings.jump {
        command_output("xdotool", &["key", "--window", id, "space"]).map(|_| ())?;
    }
    if settings.walk {
        command_output("xdotool", &["key", "--window", id, "w"]).map(|_| ())?;
        thread::sleep(Duration::from_millis(50));
        command_output("xdotool", &["key", "--window", id, "s"]).map(|_| ())?;
    }
    if settings.spin_jiggle {
        command_output("xdotool", &["key", "--window", id, "i"]).map(|_| ())?;
        thread::sleep(Duration::from_millis(50));
        command_output("xdotool", &["key", "--window", id, "o"]).map(|_| ())?;
    }
    Ok(())
}

fn capture_window(id: &str) -> Option<PathBuf> {
    let path = std::env::temp_dir().join(format!("{APP_SLUG}-x11-{id}.png"));
    let _ = std::fs::remove_file(&path);
    let import_status = Command::new("import")
        .args(["-window", id, path.to_str()?])
        .status()
        .ok();
    if import_status.is_some_and(|status| status.success()) {
        return Some(path);
    }
    let maim_status = Command::new("maim")
        .args(["-i", id, path.to_str()?])
        .status()
        .ok();
    if maim_status.is_some_and(|status| status.success()) {
        return Some(path);
    }
    None
}

fn reconnect_click(window: &X11Window) -> Result<bool, String> {
    let Some(path) = capture_window(&window.id) else {
        return Ok(false);
    };
    let result = image::open(&path)
        .ok()
        .map(|image| {
            let (width, height) = image.dimensions();
            if width < 50 || height < 50 {
                return false;
            }
            let mut found = false;
            let cx = (width / 2) as i32;
            let cy = (height / 2) as i32;
            for dy in -40..=40 {
                for dx in -40..=40 {
                    let px = (cx + dx).clamp(0, width as i32 - 1) as u32;
                    let py = (cy + dy).clamp(0, height as i32 - 1) as u32;
                    let pixel = image.get_pixel(px, py);
                    if reconnect_pixel_match(pixel[0], pixel[1], pixel[2]) {
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }
            found
        })
        .unwrap_or(false);
    let _ = std::fs::remove_file(path);
    if !result {
        return Ok(false);
    }
    let (click_x, click_y) =
        crate::inputs::common::reconnect_button(window.x, window.y, window.width, window.height);
    move_cursor(click_x, click_y)?;
    thread::sleep(Duration::from_millis(80));
    for _ in 0..3 {
        command_output("xdotool", &["click", "1"]).map(|_| ())?;
        thread::sleep(Duration::from_millis(80));
    }
    Ok(true)
}

pub fn run(state: &SharedState) -> Result<(), String> {
    let mode = state.lock().unwrap().input_mode();
    if !matches!(mode, InputMode::X11 | InputMode::I3) {
        return Err("X11 input mode is not active.".to_string());
    }
    if !has_command("xdotool") {
        return Err("xdotool is required for X11/i3 window control.".to_string());
    }
    let mut keyboard = create_keyboard_device().ok();
    let mut hidden_ids: Vec<String> = Vec::new();

    loop {
        if !should_continue(state, mode) {
            break;
        }
        let settings = { state.lock().unwrap().clone() };
        if !settings.stealth {
            for id in hidden_ids.drain(..) {
                let _ = activate(&id);
            }
        }

        if settings.user_safe {
            match user_active() {
                Ok(true) => {
                    set_status(state, RuntimeStatus::Paused);
                    if responsive_sleep(state, mode, 5) {
                        break;
                    }
                    continue;
                }
                Ok(false) => {}
                Err(error) => return Err(error),
            }
        }

        let mut targets = find_targets();
        if !settings.multi_instance {
            targets.truncate(1);
        }
        if targets.is_empty() {
            targets = find_targets_by_pid();
            if !settings.multi_instance {
                targets.truncate(1);
            }
        }
        if targets.is_empty() {
            set_status(state, RuntimeStatus::WaitingForSober);
            if responsive_sleep(state, mode, 2) {
                break;
            }
            continue;
        }

        let previous = active_window()?;
        let cursor = cursor_position().ok();
        set_action_active(state, true);
        set_status(state, RuntimeStatus::PerformingAction);
        thread::sleep(Duration::from_millis(500));

        let direct = direct_input_available();

        let mut error = None;
        for target in targets {
            if !should_continue(state, mode) {
                break;
            }
            if let Err(err) = activate(&target.id) {
                error = Some(err);
                break;
            }
            thread::sleep(Duration::from_millis(150));
            let center_x = target.x + target.width / 2;
            let center_y = target.y + target.height / 2;
            let _ = move_cursor(center_x, center_y);
            let _ = command_output("xdotool", &["click", "1"]);
            thread::sleep(Duration::from_millis(50));
            if let Err(err) = perform_actions(&mut keyboard, &settings) {
                error = Some(err);
                break;
            }
            if let Err(err) = perform_direct_actions(&target.id, &settings) {
                error = Some(err);
                break;
            }
            if settings.auto_reconnect
                && let Err(err) = reconnect_click(&target)
            {
                error = Some(err);
                break;
            }
            if settings.stealth {
                if let Err(err) = hide(&target.id) {
                    error = Some(err);
                    break;
                }
                if !hidden_ids.contains(&target.id) {
                    hidden_ids.push(target.id.clone());
                }
            }
        }

        if !direct
            && let Some(previous) = previous
            && !hidden_ids.contains(&previous)
        {
            let _ = activate(&previous);
        }
        if let Some((x, y)) = cursor {
            let _ = move_cursor(x, y);
        }
        set_action_active(state, false);

        if let Some(error) = error {
            for id in hidden_ids.drain(..) {
                let _ = activate(&id);
            }
            return Err(error);
        }
        set_status(state, RuntimeStatus::Ready);
        if responsive_sleep(state, mode, settings.interval_seq) {
            break;
        }
    }

    for id in hidden_ids.drain(..) {
        let _ = activate(&id);
    }
    set_action_active(state, false);
    set_status(state, RuntimeStatus::Stopped);
    Ok(())
}

pub fn show_sober() -> Result<(), String> {
    let target = list_windows()?
        .into_iter()
        .find(|window| is_sober_metadata(None, Some(&window.class), Some(&window.title)))
        .ok_or_else(|| "Sober window was not found.".to_string())?;
    activate(&target.id)
}

pub fn hide_sober() -> Result<(), String> {
    let target = list_windows()?
        .into_iter()
        .find(|window| is_sober_metadata(None, Some(&window.class), Some(&window.title)))
        .ok_or_else(|| "Sober window was not found.".to_string())?;
    hide(&target.id)
}

pub fn focused_is_sober() -> Option<bool> {
    if let Some(win) = i3_focused_window() {
        return Some(is_sober_metadata(None, Some(&win.class), Some(&win.title)));
    }
    let id = active_window().ok().flatten()?;
    let class = window_class(&id);
    let name = window_name(&id);
    if class.is_empty() && name.is_empty() {
        return None;
    }
    Some(is_sober_metadata(None, Some(&class), Some(&name)))
}

pub fn find_targets() -> Vec<X11Window> {
    list_windows()
        .map(|windows| {
            windows
                .into_iter()
                .filter(|window| is_sober_metadata(None, Some(&window.class), Some(&window.title)))
                .collect()
        })
        .unwrap_or_default()
}

pub fn find_targets_by_pid() -> Vec<X11Window> {
    let pids = crate::backend::sober_pids();
    if pids.is_empty() {
        return Vec::new();
    }
    let windows = match list_windows() {
        Ok(windows) => windows,
        Err(_) => return Vec::new(),
    };
    windows
        .into_iter()
        .filter(|window| {
            if is_sober_metadata(None, Some(&window.class), Some(&window.title)) {
                return true;
            }
            window_pid(&window.id).is_some_and(|pid| pids.contains(&pid))
        })
        .collect()
}

fn window_pid(id: &str) -> Option<String> {
    let output = Command::new("xdotool")
        .args(["getwindowpid", id])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if pid.is_empty() || pid == "0" {
        return None;
    }
    Some(pid)
}

pub fn supports_enumeration() -> bool {
    (has_command("i3-msg") && std::env::var_os("I3SOCK").is_some())
        || (has_command("xdotool") && (has_command("wmctrl") || has_command("xprop")))
}

pub fn supports_capture() -> bool {
    has_command("import") || has_command("maim")
}

pub fn supports_hide() -> bool {
    has_command("xdotool") || (has_command("i3-msg") && std::env::var_os("I3SOCK").is_some())
}
