use crate::environment::{InputMode, is_hyprland};
use crate::input::{create_keyboard_device, create_mouse_device, emit_key};
use crate::inputs::common::{
    is_sober_metadata, perform_keyboard_actions, reconnect_button, reconnect_pixel_match,
    reconnect_probe, responsive_sleep, responsive_sleep_interval,
};
use crate::state::{RuntimeStatus, SharedState, set_runtime_status};
use evdev::KeyCode;
use serde_json::Value;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub fn run(state_arc: &SharedState) -> Result<(), String> {
    if !is_hyprland() {
        return Err("Hyprland environment is required.".to_string());
    }
    let mut kb_device = create_keyboard_device()?;
    let mut mouse_device = create_mouse_device()?;

    loop {
        let s = { state_arc.lock().unwrap().clone() };
        if !s.running {
            if s.stealth {
                restore_hidden_windows();
            }
            set_runtime_status(state_arc, RuntimeStatus::Stopped);
            break;
        }
        if s.input_mode() != InputMode::Hyprland {
            break;
        }

        if s.user_safe && is_user_active_info(3).0 {
            set_runtime_status(state_arc, RuntimeStatus::Paused);
            if responsive_sleep(state_arc, InputMode::Hyprland, 5) {
                break;
            }
            continue;
        }

        let cursor_output = Command::new("hyprctl")
            .args(["cursorpos", "-j"])
            .output()
            .map_err(|e| e.to_string())?;
        let cursor_json: Value =
            serde_json::from_slice(&cursor_output.stdout).unwrap_or(Value::Null);
        let orig_x = cursor_json.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
        let orig_y = cursor_json.get("y").and_then(|v| v.as_i64()).unwrap_or(0);

        let prev_window = Command::new("hyprctl")
            .args(["activewindow", "-j"])
            .output()
            .ok()
            .and_then(|out| serde_json::from_slice::<Value>(&out.stdout).ok())
            .and_then(|json| {
                json.get("address")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });

        let clients_output = Command::new("hyprctl")
            .args(["clients", "-j"])
            .output()
            .map_err(|e| e.to_string())?;
        let clients_json: Value =
            serde_json::from_slice(&clients_output.stdout).unwrap_or(Value::Null);

        let mut target_windows = Vec::new();
        let my_pid = i64::from(std::process::id());

        if let Some(clients) = clients_json.as_array() {
            for client in clients {
                let class = client
                    .get("class")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                let title = client
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                let pid = client.get("pid").and_then(|v| v.as_i64()).unwrap_or(0);
                let workspace = client
                    .get("workspace")
                    .and_then(|v| v.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("1");

                if pid == my_pid {
                    continue;
                }
                if is_sober_metadata(None, Some(&class), Some(&title)) {
                    let addr = client
                        .get("address")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let at = client.get("at").and_then(|v| v.as_array());
                    let size = client.get("size").and_then(|v| v.as_array());
                    if let (Some(a), Some(p), Some(s)) = (addr, at, size) {
                        let x = p.first().and_then(|v| v.as_i64()).unwrap_or(0);
                        let y = p.get(1).and_then(|v| v.as_i64()).unwrap_or(0);
                        let w = s.first().and_then(|v| v.as_i64()).unwrap_or(1);
                        let h = s.get(1).and_then(|v| v.as_i64()).unwrap_or(1);
                        target_windows.push((a, x, y, w, h, workspace.to_string()));
                    }
                }
            }
        }

        if target_windows.is_empty() {
            set_runtime_status(state_arc, RuntimeStatus::WaitingForSober);
            if responsive_sleep(state_arc, InputMode::Hyprland, 2) {
                break;
            }
            continue;
        }

        if !s.multi_instance {
            target_windows.truncate(1);
        }

        {
            let mut state = state_arc.lock().unwrap();
            state.action_active = true;
            state.runtime_status = RuntimeStatus::PerformingAction;
        }
        thread::sleep(Duration::from_secs(1));

        let mut last_x = orig_x as i32;
        let mut last_y = orig_y as i32;

        for (addr, _wx, _wy, _ww, _wh, ws) in target_windows {
            if s.stealth {
                let _ = Command::new("hyprctl")
                    .args([
                        "dispatch",
                        "movetoworkspace",
                        &ws,
                        &format!("address:{addr}"),
                    ])
                    .output();
                thread::sleep(Duration::from_millis(200));
            }

            let _ = Command::new("hyprctl")
                .args(["dispatch", "focuswindow", &format!("address:{addr}")])
                .output();
            thread::sleep(Duration::from_millis(150));

            let fresh_clients = Command::new("hyprctl")
                .args(["clients", "-j"])
                .output()
                .ok();
            let mut found_pos = (0, 0, 0, 0);
            if let Some(out) = fresh_clients {
                let json: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
                if let Some(arr) = json.as_array() {
                    for c in arr {
                        if c.get("address").and_then(|v| v.as_str()) == Some(&addr) {
                            if let (Some(at), Some(size)) = (
                                c.get("at").and_then(|v| v.as_array()),
                                c.get("size").and_then(|v| v.as_array()),
                            ) {
                                found_pos = (
                                    at.first().and_then(|v| v.as_i64()).unwrap_or(0),
                                    at.get(1).and_then(|v| v.as_i64()).unwrap_or(0),
                                    size.first().and_then(|v| v.as_i64()).unwrap_or(1),
                                    size.get(1).and_then(|v| v.as_i64()).unwrap_or(1),
                                );
                            }
                            break;
                        }
                    }
                }
            }

            if found_pos.2 > 0 {
                let cx = (found_pos.0 + found_pos.2 / 2) as i32;
                let cy = (found_pos.1 + found_pos.3 / 2) as i32;
                incremental_mouse_move(last_x, last_y, cx, cy, 3, 30);
                last_x = cx;
                last_y = cy;

                thread::sleep(Duration::from_millis(50));
                let _ = emit_key(&mut mouse_device, KeyCode::BTN_LEFT, true);
                thread::sleep(Duration::from_millis(30));
                let _ = emit_key(&mut mouse_device, KeyCode::BTN_LEFT, false);
                thread::sleep(Duration::from_millis(50));

                if s.auto_reconnect {
                    let (window_x, window_y, window_w, window_h) = (
                        found_pos.0 as i32,
                        found_pos.1 as i32,
                        found_pos.2 as i32,
                        found_pos.3 as i32,
                    );
                    let (check_x, check_y) =
                        reconnect_probe(window_x, window_y, window_w, window_h);
                    if let Some((r, g, b)) = get_pixel_color(check_x, check_y)
                        && reconnect_pixel_match(r, g, b)
                    {
                        let (target_x, target_y) =
                            reconnect_button(window_x, window_y, window_w, window_h);
                        incremental_mouse_move(cx, cy, target_x, target_y, 3, 80);
                        thread::sleep(Duration::from_millis(100));
                        for _ in 0..3 {
                            let _ = emit_key(&mut mouse_device, KeyCode::BTN_LEFT, true);
                            thread::sleep(Duration::from_millis(30));
                            let _ = emit_key(&mut mouse_device, KeyCode::BTN_LEFT, false);
                            thread::sleep(Duration::from_millis(30));
                        }
                        incremental_mouse_move(target_x, target_y, cx, cy, 3, 60);
                    }
                }

                let _ = perform_keyboard_actions(&mut kb_device, &s);
            }

            if s.stealth {
                let _ = Command::new("hyprctl")
                    .args([
                        "dispatch",
                        "movetoworkspacesilent",
                        "special",
                        &format!("address:{addr}"),
                    ])
                    .output();
            }
        }

        let _ = Command::new("hyprctl")
            .args([
                "dispatch",
                "movecursor",
                &orig_x.to_string(),
                &orig_y.to_string(),
            ])
            .output();
        if let Some(prev_addr) = prev_window {
            let _ = Command::new("hyprctl")
                .args(["dispatch", "focuswindow", &format!("address:{prev_addr}")])
                .output();
        }
        {
            let mut state = state_arc.lock().unwrap();
            state.action_active = false;
            state.runtime_status = RuntimeStatus::Ready;
        }

        if responsive_sleep_interval(state_arc, InputMode::Hyprland) {
            let s = { state_arc.lock().unwrap().clone() };
            if !s.running {
                if s.stealth {
                    restore_hidden_windows();
                }
                set_runtime_status(state_arc, RuntimeStatus::Stopped);
            }
            break;
        }
    }
    let s = { state_arc.lock().unwrap().clone() };
    if s.stealth {
        restore_hidden_windows();
    }
    Ok(())
}

pub fn show_sober() -> Result<(), String> {
    let clients_output = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .map_err(|e| e.to_string())?;
    let clients: Value = serde_json::from_slice(&clients_output.stdout).unwrap_or(Value::Null);
    let mut found = false;
    if let Some(arr) = clients.as_array() {
        for c in arr {
            let class = c.get("class").and_then(|v| v.as_str()).unwrap_or("");
            let title = c.get("title").and_then(|v| v.as_str()).unwrap_or("");
            if is_sober_metadata(None, Some(class), Some(title))
                && let Some(addr) = c.get("address").and_then(|v| v.as_str())
            {
                let _ = Command::new("hyprctl")
                    .args([
                        "dispatch",
                        "movetoworkspace",
                        &format!("current,address:{addr}"),
                    ])
                    .output();
                let _ = Command::new("hyprctl")
                    .args(["dispatch", "focuswindow", &format!("address:{addr}")])
                    .output();
                found = true;
            }
        }
    }
    if !found {
        return Err("Sober window was not found.".to_string());
    }
    Ok(())
}

pub fn hide_sober() -> Result<(), String> {
    let clients_output = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .map_err(|e| e.to_string())?;
    let clients: Value = serde_json::from_slice(&clients_output.stdout).unwrap_or(Value::Null);
    let mut found = false;
    if let Some(arr) = clients.as_array() {
        for c in arr {
            let class = c.get("class").and_then(|v| v.as_str()).unwrap_or("");
            let title = c.get("title").and_then(|v| v.as_str()).unwrap_or("");
            if is_sober_metadata(None, Some(class), Some(title))
                && let Some(addr) = c.get("address").and_then(|v| v.as_str())
            {
                let _ = Command::new("hyprctl")
                    .args([
                        "dispatch",
                        "movetoworkspacesilent",
                        &format!("special,address:{addr}"),
                    ])
                    .output();
                found = true;
            }
        }
    }
    if !found {
        return Err("Sober window was not found.".to_string());
    }
    Ok(())
}

pub fn restore_hidden_windows() {
    let _ = show_sober();
}

fn is_user_active_info(secs: u64) -> (bool, Option<(String, String, String)>) {
    if !is_hyprland() {
        return (false, None);
    }
    if crate::inputs::common::has_recent_input(secs) {
        return (true, None);
    }
    let get_cursor = || {
        Command::new("hyprctl")
            .args(["cursorpos", "-j"])
            .output()
            .ok()
            .and_then(|out| {
                let json: Value = serde_json::from_slice(&out.stdout).ok()?;
                Some((json.get("x")?.as_i64()?, json.get("y")?.as_i64()?))
            })
    };
    let s_pos = get_cursor();
    thread::sleep(Duration::from_secs(secs));
    let e_pos = get_cursor();
    (s_pos != e_pos, None)
}

fn get_pixel_color(x: i32, y: i32) -> Option<(u8, u8, u8)> {
    let output = Command::new("grim")
        .args(["-t", "ppm", "-g", &format!("{x},{y} 1x1"), "-"])
        .output()
        .ok()?;
    let bytes = output.stdout;
    if bytes.len() >= 13 && bytes.starts_with(b"P6") {
        let l = bytes.len();
        return Some((bytes[l - 3], bytes[l - 2], bytes[l - 1]));
    }
    None
}

fn incremental_mouse_move(s_x: i32, s_y: i32, e_x: i32, e_y: i32, steps: u32, dur: u64) {
    if s_x == e_x && s_y == e_y {
        return;
    }
    let steps = steps.clamp(1, 5);
    let slice = dur / steps as u64;
    for i in 1..=steps {
        let p = i as f64 / steps as f64;
        let cur_x = s_x + ((e_x - s_x) as f64 * p) as i32;
        let cur_y = s_y + ((e_y - s_y) as f64 * p) as i32;
        let _ = Command::new("hyprctl")
            .args([
                "dispatch",
                "movecursor",
                &cur_x.to_string(),
                &cur_y.to_string(),
            ])
            .output();
        if dur > 0 && slice > 0 {
            thread::sleep(Duration::from_millis(slice));
        }
    }
}
