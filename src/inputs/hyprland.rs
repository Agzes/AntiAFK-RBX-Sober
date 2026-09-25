use crate::input::{create_keyboard_device, create_mouse_device, emit_key, tap_key};
use crate::state::{RuntimeStatus, SharedState, set_runtime_status};
use evdev::{Key, uinput::VirtualDevice};
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
            set_runtime_status(state_arc, RuntimeStatus::Stopped);
            break;
        }
        if s.mode != 0 {
            break;
        }

        if s.user_safe && is_user_active_info(3).0 {
            set_runtime_status(state_arc, RuntimeStatus::Paused);
            thread::sleep(Duration::from_secs(5));
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
                if class.contains("sober")
                    || title.contains("roblox")
                    || class.contains("vinegar")
                    || class == "sober"
                {
                    let addr = client
                        .get("address")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let at = client.get("at").and_then(|v| v.as_array());
                    let size = client.get("size").and_then(|v| v.as_array());
                    if let (Some(a), Some(p), Some(s)) = (addr, at, size) {
                        let x = p[0].as_i64().unwrap_or(0);
                        let y = p[1].as_i64().unwrap_or(0);
                        let w = s[0].as_i64().unwrap_or(1);
                        let h = s[1].as_i64().unwrap_or(1);
                        target_windows.push((a, x, y, w, h, workspace.to_string()));
                    }
                }
            }
        }

        if target_windows.is_empty() {
            set_runtime_status(state_arc, RuntimeStatus::WaitingForSober);
            thread::sleep(Duration::from_secs(2));
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

        let mut last_x = orig_x;
        let mut last_y = orig_y;

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
                            let at = c.get("at").and_then(|v| v.as_array()).unwrap();
                            let size = c.get("size").and_then(|v| v.as_array()).unwrap();
                            found_pos = (
                                at[0].as_i64().unwrap_or(0),
                                at[1].as_i64().unwrap_or(0),
                                size[0].as_i64().unwrap_or(1),
                                size[1].as_i64().unwrap_or(1),
                            );
                            break;
                        }
                    }
                }
            }

            if found_pos.2 > 0 {
                let cx = found_pos.0 + found_pos.2 / 2;
                let cy = found_pos.1 + found_pos.3 / 2;
                incremental_mouse_move(&mut mouse_device, last_x, last_y, cx, cy, 5, 30);
                last_x = cx;
                last_y = cy;

                thread::sleep(Duration::from_millis(50));
                let _ = emit_key(&mut mouse_device, Key::BTN_LEFT, true);
                thread::sleep(Duration::from_millis(30));
                let _ = emit_key(&mut mouse_device, Key::BTN_LEFT, false);
                thread::sleep(Duration::from_millis(50));

                if s.auto_reconnect {
                    let check_x = found_pos.0 + (found_pos.2 - 400) / 2 + 10;
                    let check_y = found_pos.1 + (found_pos.3 - 250) / 2 + 10;
                    if let Some((r, g, b)) = get_pixel_color(check_x, check_y) {
                        if r == 57 && g == 59 && b == 61 {
                            let target_x = found_pos.0
                                + (found_pos.2 - 400) / 2
                                + (400 - 161 - 27)
                                + (161 / 2);
                            let target_y =
                                found_pos.1 + (found_pos.3 - 250) / 2 + (250 - 34 - 21) + (34 / 2);
                            incremental_mouse_move(
                                &mut mouse_device,
                                cx,
                                cy,
                                target_x,
                                target_y,
                                15,
                                100,
                            );
                            thread::sleep(Duration::from_millis(100));
                            for _ in 0..3 {
                                let _ = emit_key(&mut mouse_device, Key::BTN_LEFT, true);
                                thread::sleep(Duration::from_millis(30));
                                let _ = emit_key(&mut mouse_device, Key::BTN_LEFT, false);
                                thread::sleep(Duration::from_millis(30));
                            }
                            incremental_mouse_move(
                                &mut mouse_device,
                                target_x,
                                target_y,
                                cx,
                                cy,
                                10,
                                80,
                            );
                        }
                    }
                }

                if s.jump {
                    let _ = tap_key(&mut kb_device, Key::KEY_SPACE, Duration::from_millis(30));
                }
                if s.walk {
                    let _ = tap_key(&mut kb_device, Key::KEY_W, Duration::from_millis(150));
                    thread::sleep(Duration::from_millis(50));
                    let _ = tap_key(&mut kb_device, Key::KEY_S, Duration::from_millis(150));
                }
                if s.spin_jiggle {
                    let _ = tap_key(&mut kb_device, Key::KEY_I, Duration::from_millis(30));
                    thread::sleep(Duration::from_millis(50));
                    let _ = tap_key(&mut kb_device, Key::KEY_O, Duration::from_millis(30));
                }
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
        {
            let mut state = state_arc.lock().unwrap();
            state.action_active = false;
            state.runtime_status = RuntimeStatus::Ready;
        }

        if responsive_sleep(state_arc, 0) {
            if !state_arc.lock().unwrap().running {
                set_runtime_status(state_arc, RuntimeStatus::Stopped);
            }
            break;
        }
    }
    Ok(())
}

fn is_hyprland() -> bool {
    crate::state::AppState::is_hyprland()
}

fn is_user_active_info(secs: u64) -> (bool, Option<(String, String, String)>) {
    if !is_hyprland() {
        return (false, None);
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

fn get_pixel_color(x: i64, y: i64) -> Option<(u8, u8, u8)> {
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

fn incremental_mouse_move(
    _dev: &mut VirtualDevice,
    s_x: i64,
    s_y: i64,
    e_x: i64,
    e_y: i64,
    steps: i32,
    dur: u64,
) {
    if s_x == e_x && s_y == e_y {
        return;
    }
    for i in 1..=steps {
        let p = i as f64 / steps as f64;
        let cur_x = s_x + ((e_x - s_x) as f64 * p) as i64;
        let cur_y = s_y + ((e_y - s_y) as f64 * p) as i64;
        let _ = Command::new("hyprctl")
            .args([
                "dispatch",
                "movecursor",
                &cur_x.to_string(),
                &cur_y.to_string(),
            ])
            .output();
        if dur > 0 {
            thread::sleep(Duration::from_millis(dur / steps as u64));
        }
    }
}

fn responsive_sleep(state_arc: &SharedState, mode: usize) -> bool {
    let interval = state_arc.lock().unwrap().interval_seq;
    for _ in 0..interval {
        thread::sleep(Duration::from_secs(1));
        let s = state_arc.lock().unwrap().clone();
        if !s.running || s.mode != mode {
            return true;
        }
    }
    false
}
