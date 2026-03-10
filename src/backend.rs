use crate::input::{create_keyboard_device, create_mouse_device, emit_key};
use crate::state::{SharedState, APP_ID};
use evdev::{Key, uinput::VirtualDevice};
use notify_rust::Notification;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::thread;
use std::time::Duration;

pub fn start_backend(state: SharedState) {
    let state_auto = state.clone();
    thread::spawn(move || {
        let mut sober_absent_ticks = 0;
        loop {
            let (auto_start, is_running, manually_stopped) = {
                let s = state_auto.lock().unwrap();
                (s.auto_start, s.running, s.manually_stopped)
            };

            let sober_running = check_sober_running();

            if sober_running {
                sober_absent_ticks = 0;
                if auto_start && !is_running && !manually_stopped {
                    let mut s = state_auto.lock().unwrap();
                    s.running = true;
                    let _ = Notification::new()
                        .appname(APP_ID)
                        .summary("AntiAFK-RBX-Sober")
                        .body("Sober detected. Anti-AFK enabled.")
                        .icon(APP_ID)
                        .timeout(3000)
                        .show();
                }
            } else {
                sober_absent_ticks += 1;
                if sober_absent_ticks >= 2 {
                    if is_running {
                        let mut s = state_auto.lock().unwrap();
                        s.running = false;
                        s.manually_stopped = false;
                        let _ = Notification::new()
                            .appname(APP_ID)
                            .summary("AntiAFK-RBX-Sober")
                            .body("Sober closed. Anti-AFK disabled.")
                            .icon(APP_ID)
                            .timeout(3000)
                            .show();
                    } else if manually_stopped {
                        let mut s = state_auto.lock().unwrap();
                        s.manually_stopped = false;
                    }
                }
            }

            thread::sleep(Duration::from_secs(7));
        }
    });

    let state_fps = state.clone();
    thread::spawn(move || {
        let mut active_scopes: HashMap<String, u32> = HashMap::new();
        let my_pid = std::process::id().to_string();

        loop {
            let (enabled, fps_limit, stop_on_focus, is_running, action_active) = {
                let s = state_fps.lock().unwrap();
                (
                    s.fps_capper,
                    s.fps_limit,
                    s.stop_limit_on_focus,
                    s.running,
                    s.action_active,
                )
            };

            if enabled && is_running && fps_limit > 0 && !action_active {
                let mut current_target_scopes = HashSet::new();
                let main_pids = get_all_sober_pids(&my_pid);

                for pid in &main_pids {
                    let name = get_process_name(pid).to_lowercase();
                    let is_browser = name.contains("vivaldi")
                        || name.contains("chromium")
                        || name.contains("chrome")
                        || name.contains("firefox");
                    let is_me = name.contains("antiafk");

                    if !is_browser
                        && !is_me
                        && let Some(scope) = get_systemd_scope(pid)
                        && (scope.contains("app")
                            || scope.contains("sober")
                            || scope.contains("vinegar")
                            || scope.contains("flatpak"))
                        && !scope.contains("vivaldi")
                        && !scope.contains("chromium")
                        && !scope.contains("firefox")
                    {
                        current_target_scopes.insert(scope);
                    }
                }

                let mut is_focused = false;
                if stop_on_focus {
                    if is_hyprland() {
                        if let Ok(active_output) = Command::new("hyprctl")
                            .args(["activewindow", "-j"])
                            .output()
                            && let Ok(json) = serde_json::from_slice::<Value>(&active_output.stdout)
                        {
                            let class = json
                                .get("class")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_lowercase();
                            let title = json
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if (class.contains("sober")
                                || title.contains("roblox")
                                || class.contains("vinegar")
                                || class.contains("org.vinegarhq.sober"))
                                && !class.contains("antiafk")
                                && !title.contains("anti-afk")
                            {
                                is_focused = true;
                            }
                        }
                    }
                }

                if is_focused || current_target_scopes.is_empty() {
                    if !active_scopes.is_empty() {
                        for scope in active_scopes.keys() {
                            let _ = Command::new("systemctl")
                                .args(["--user", "set-property", scope, "CPUQuota="])
                                .output();
                        }
                        active_scopes.clear();
                    }
                    thread::sleep(Duration::from_millis(500));
                    continue;
                }

                let quota_pct = fps_limit.clamp(3, 99);

                active_scopes.retain(|s, _| {
                    if current_target_scopes.contains(s) {
                        true
                    } else {
                        let _ = Command::new("systemctl")
                            .args(["--user", "set-property", s, "CPUQuota="])
                            .output();
                        false
                    }
                });

                for scope in &current_target_scopes {
                    let current_limit = active_scopes.get(scope);
                    if current_limit != Some(&quota_pct) {
                        let _ = Command::new("systemctl")
                            .args([
                                "--user",
                                "set-property",
                                scope,
                                &format!("CPUQuota={quota_pct}%"),
                            ])
                            .output();
                        active_scopes.insert(scope.clone(), quota_pct);
                    }
                }

                thread::sleep(Duration::from_secs(1));
            } else {
                if !active_scopes.is_empty() {
                    for scope in active_scopes.keys() {
                        let _ = Command::new("systemctl")
                            .args(["--user", "set-property", scope, "CPUQuota="])
                            .output();
                    }
                    active_scopes.clear();
                }
                thread::sleep(Duration::from_millis(500));
            }
        }
    });

    thread::spawn(move || {
        loop {
            let current_state = { state.lock().unwrap().clone() };

            if current_state.running {
                let res = match current_state.mode {
                    0 => run_window_focus_mode(&state),
                    _ => Ok(()),
                };

                if let Err(e) = res {
                    eprintln!("Backend error: {e}");
                    let mut s = state.lock().unwrap();
                    s.running = false;
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}

fn run_window_focus_mode(state_arc: &SharedState) -> Result<(), String> {
    if !is_hyprland() {
        return Err("Window Focus mode currently only supports Hyprland.".to_string());
    }
    let mut kb_device = create_keyboard_device()?;
    let mut mouse_device = create_mouse_device()?;

    loop {
        let s = { state_arc.lock().unwrap().clone() };
        if !s.running || s.mode != 0 {
            break;
        }
        if s.user_safe {
            let mut wait_total = 0;
            let mut notification_sent = false;
            while wait_total < 60 {
                let (active, _) = is_user_active_info(3);
                if active {
                    if !notification_sent {
                        let _ = Notification::new()
                            .appname(APP_ID)
                            .summary("AntiAFK-RBX-Sober")
                            .body("User activity detected. Action paused.")
                            .icon(APP_ID)
                            .timeout(5000)
                            .show();
                        notification_sent = true;
                    }
                    wait_total += 3;
                    continue;
                }
                break;
            }
            if notification_sent {
                let _ = Notification::new()
                    .appname(APP_ID)
                    .summary("AntiAFK-RBX-Sober")
                    .body("Activity stopped. Resuming action.")
                    .icon(APP_ID)
                    .timeout(3000)
                    .show();
            }
        }

        let cursor_output = Command::new("hyprctl")
            .args(["cursorpos", "-j"])
            .output()
            .map_err(|e| e.to_string())?;
        let cursor_json: Value =
            serde_json::from_slice(&cursor_output.stdout).unwrap_or(Value::Null);
        let orig_x = cursor_json
            .get("x")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        let orig_y = cursor_json
            .get("y")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);

        let mut last_x = orig_x;
        let mut last_y = orig_y;

        let active_output = Command::new("hyprctl")
            .args(["activewindow", "-j"])
            .output()
            .map_err(|e| e.to_string())?;
        let active_json: Value =
            serde_json::from_slice(&active_output.stdout).unwrap_or(Value::Null);
        let current_addr = active_json
            .get("address")
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string);

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
                let pid = client
                    .get("pid")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);
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
                        .map(std::string::ToString::to_string);
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

        if !s.multi_instance && !target_windows.is_empty() {
            target_windows.truncate(1);
        }

        {
            state_arc.lock().unwrap().action_active = true;
        }

        for (addr, _wx, _wy, _ww, _wh, ws) in target_windows {
            if s.hides_game {
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
            let (fwx, fwy, fww, fwh) = if let Some(out) = fresh_clients {
                let json: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
                let mut found = (0, 0, 0, 0);
                if let Some(arr) = json.as_array() {
                    for c in arr {
                        if c.get("address").and_then(|v| v.as_str()) == Some(&addr) {
                            let at = c.get("at").and_then(|v| v.as_array()).unwrap();
                            let size = c.get("size").and_then(|v| v.as_array()).unwrap();
                            found = (
                                at[0].as_i64().unwrap_or(0),
                                at[1].as_i64().unwrap_or(0),
                                size[0].as_i64().unwrap_or(1),
                                size[1].as_i64().unwrap_or(1),
                            );
                            break;
                        }
                    }
                }
                found
            } else {
                (0, 0, 0, 0)
            };

            if fww == 0 {
                continue;
            }

            let cx = fwx + fww / 2;
            let cy = fwy + fwh / 2;

            incremental_mouse_move(&mut mouse_device, last_x, last_y, cx, cy, 5, 30);
            last_x = cx;
            last_y = cy;

            thread::sleep(Duration::from_millis(10));
            let _ = emit_key(&mut mouse_device, Key::BTN_LEFT, true);
            thread::sleep(Duration::from_millis(30));
            let _ = emit_key(&mut mouse_device, Key::BTN_LEFT, false);
            thread::sleep(Duration::from_millis(50));

            if s.auto_reconnect {
                let kick_width = 400;
                let kick_height = 250;
                let elem_x = (fww - kick_width) / 2;
                let elem_y = (fwh - kick_height) / 2;
                let check_x = fwx + elem_x + 10;
                let check_y = fwy + elem_y + 10;
                let is_kick_dialog = match get_pixel_color(check_x, check_y) {
                    Some((r, g, b)) => r == 57 && g == 59 && b == 61,
                    None => false,
                };

                if is_kick_dialog {
                    let btn_width = 161;
                    let btn_height = 34;
                    let btn_rel_x = kick_width - btn_width - 27;
                    let btn_rel_y = kick_height - btn_height - 21;
                    let target_x = fwx + elem_x + btn_rel_x + (btn_width / 2);
                    let target_y = fwy + elem_y + btn_rel_y + (btn_height / 2);
                    incremental_mouse_move(&mut mouse_device, cx, cy, target_x, target_y, 20, 150);
                    thread::sleep(Duration::from_millis(100));
                    for _ in 0..3 {
                        let _ = emit_key(&mut mouse_device, Key::BTN_LEFT, true);
                        thread::sleep(Duration::from_millis(30));
                        let _ = emit_key(&mut mouse_device, Key::BTN_LEFT, false);
                        thread::sleep(Duration::from_millis(30));
                    }
                    thread::sleep(Duration::from_millis(100));
                    incremental_mouse_move(&mut mouse_device, target_x, target_y, cx, cy, 15, 100);
                }
            }

            if s.jump {
                let _ = emit_key(&mut kb_device, Key::KEY_SPACE, true);
                thread::sleep(Duration::from_millis(30));
                let _ = emit_key(&mut kb_device, Key::KEY_SPACE, false);
            }
            if s.walk {
                let _ = emit_key(&mut kb_device, Key::KEY_W, true);
                thread::sleep(Duration::from_millis(150));
                let _ = emit_key(&mut kb_device, Key::KEY_W, false);
            }
            if s.spin_jiggle {
                let _ = emit_key(&mut kb_device, Key::KEY_I, true);
                thread::sleep(Duration::from_millis(30));
                let _ = emit_key(&mut kb_device, Key::KEY_I, false);
                thread::sleep(Duration::from_millis(50));
                let _ = emit_key(&mut kb_device, Key::KEY_O, true);
                thread::sleep(Duration::from_millis(30));
                let _ = emit_key(&mut kb_device, Key::KEY_O, false);
            }

            if s.hides_game {
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

        if let Some(prev_addr) = current_addr {
            let _ = Command::new("hyprctl")
                .args(["dispatch", "focuswindow", &format!("address:{prev_addr}")])
                .output();
            thread::sleep(Duration::from_millis(50));
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
            state_arc.lock().unwrap().action_active = false;
        }

        if responsive_sleep(state_arc, 0) {
            break;
        }
    }

    let active_ws = Command::new("hyprctl")
        .args(["activeworkspace", "-j"])
        .output()
        .ok()
        .and_then(|out| {
            let json: Value = serde_json::from_slice(&out.stdout).ok()?;
            json.get("name")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string)
        })
        .unwrap_or_else(|| "1".to_string());

    if let Ok(out) = Command::new("hyprctl").args(["clients", "-j"]).output()
        && let Ok(clients) = serde_json::from_slice::<Value>(&out.stdout)
        && let Some(arr) = clients.as_array()
    {
        for client in arr {
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
            let addr = client.get("address").and_then(|v| v.as_str());
            let workspace_name = client
                .get("workspace")
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if (class.contains("sober")
                || title.contains("roblox")
                || class.contains("vinegar")
                || class == "sober")
                && workspace_name.contains("special")
                && let Some(a) = addr
            {
                let _ = Command::new("hyprctl")
                    .args([
                        "dispatch",
                        "movetoworkspacesilent",
                        &format!("{active_ws},address:{a}"),
                    ])
                    .output();
            }
        }
    }

    Ok(())
}

fn check_sober_running() -> bool {
    if is_hyprland() {
        let clients_output = Command::new("hyprctl").args(["clients", "-j"]).output();
        if let Ok(out) = clients_output {
            let clients_json: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
            if let Some(clients) = clients_json.as_array() {
                let my_pid = std::process::id() as i64;
                for client in clients {
                    let pid = client
                        .get("pid")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0);
                    if pid == my_pid {
                        continue;
                    }
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
                    
                    if class == "dev.agzes.antiafk-rbx-sober" || class == "antiafk-rbx-sober" {
                        continue;
                    }

                    if class.contains("sober")
                        || title.contains("roblox")
                        || class.contains("vinegar")
                        || class.contains("org.vinegarhq.sober")
                    {
                        return true;
                    }
                }
            }
        }
    }
    !get_all_sober_pids(&std::process::id().to_string()).is_empty()
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
    let get_keyboard_state = || {
        Command::new("hyprctl")
            .args(["devices", "-j"])
            .output()
            .ok()
            .and_then(|out| {
                let json: Value = serde_json::from_slice(&out.stdout).ok()?;
                let keyboards = json.get("keyboards")?.as_array()?;
                for kb in keyboards {
                    if kb.get("main")?.as_bool() == Some(true) {
                        return Some(kb.get("active_keymap")?.as_str()?.to_string());
                    }
                }
                None
            })
    };
    let start_cursor = get_cursor();
    let start_kb = get_keyboard_state();
    thread::sleep(Duration::from_secs(secs));
    let end_cursor = get_cursor();
    let end_kb = get_keyboard_state();
    let active = start_cursor != end_cursor || start_kb != end_kb;
    let window = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .ok()
        .and_then(|out| {
            let json: Value = serde_json::from_slice(&out.stdout).ok()?;
            Some((
                json.get("address")?.as_str()?.to_string(),
                json.get("title")?.as_str()?.to_string(),
                json.get("class")?.as_str()?.to_string(),
            ))
        });
    (active, window)
}

fn get_pixel_color(x: i64, y: i64) -> Option<(u8, u8, u8)> {
    let output = Command::new("grim")
        .args(["-t", "ppm", "-g", &format!("{x},{y} 1x1"), "-"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let bytes = output.stdout;
    if bytes.len() >= 13 && bytes.starts_with(b"P6") {
        let l = bytes.len();
        return Some((bytes[l - 3], bytes[l - 2], bytes[l - 1]));
    }
    None
}

fn incremental_mouse_move(
    _mouse_device: &mut VirtualDevice,
    start_x: i64,
    start_y: i64,
    end_x: i64,
    end_y: i64,
    steps: i32,
    duration_ms: u64,
) {
    if start_x == end_x && start_y == end_y {
        return;
    }
    for i in 1..=steps {
        let progress = f64::from(i) / f64::from(steps);
        let factor = 0.5 * (1.0 - (progress * std::f64::consts::PI).cos());
        let cur_x = start_x + ((end_x - start_x) as f64 * factor) as i64;
        let cur_y = start_y + ((end_y - start_y) as f64 * factor) as i64;
        let _ = Command::new("hyprctl")
            .args([
                "dispatch",
                "movecursor",
                &cur_x.to_string(),
                &cur_y.to_string(),
            ])
            .output();
        if steps > 1 && duration_ms > 0 {
            thread::sleep(Duration::from_millis(duration_ms / steps as u64));
        }
    }
}

fn responsive_sleep(state_arc: &SharedState, mode: usize) -> bool {
    let interval = { state_arc.lock().unwrap().interval_seq };
    for _ in 0..interval {
        thread::sleep(Duration::from_secs(1));
        let s = { state_arc.lock().unwrap().clone() };
        if !s.running || s.mode != mode {
            return true;
        }
    }
    false
}

fn get_process_name(pid: &str) -> String {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .map_or_else(|_| "unknown".to_string(), |s| s.trim().to_string())
}

fn get_systemd_scope(pid: &str) -> Option<String> {
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
    for line in cgroup.lines() {
        if let Some(path) = line.split("::").nth(1)
            && let Some(scope) = path.split('/').next_back()
            && (scope.ends_with(".scope") || scope.ends_with(".service"))
        {
            return Some(scope.to_string());
        }
    }
    None
}

fn get_all_sober_pids(exclude_pid: &str) -> Vec<String> {
    let output = Command::new("pgrep")
        .args([
            "-if",
            "sober|roblox|vinegar|Sober.bin|org.vinegarhq.Sober|RobloxPlayerBeta",
        ])
        .output()
        .ok();
    if let Some(out) = output {
        let s = String::from_utf8_lossy(&out.stdout);
        let my_pid = exclude_pid.to_string();
        return s
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|pid| {
                if pid.is_empty() || pid == &my_pid {
                    return false;
                }
                
                if let Ok(status) = std::fs::read_to_string(format!("/proc/{pid}/status")) {
                    if status.contains("State:\tZ (zombie)") {
                        return false;
                    }
                }

                let name = get_process_name(pid).to_lowercase();
                if name.contains("antiafk"){
                    return false;
                }
                
                if let Ok(cmdline) = std::fs::read_to_string(format!("/proc/{pid}/cmdline")) {
                    let cmdline = cmdline.to_lowercase();
                    if cmdline.contains("antiafk") || cmdline.contains("rust-analyzer") || cmdline.contains("language_server") {
                        return false;
                    }
                }
                
                true
            })
            .collect();
    }
    Vec::new()
}

pub fn is_hyprland() -> bool {
    std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
}
