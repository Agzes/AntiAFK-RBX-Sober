use crate::state::{SharedState, APP_ID};
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

            if check_sober_running() {
                sober_absent_ticks = 0;
                if auto_start && !is_running && !manually_stopped {
                    state_auto.lock().unwrap().running = true;
                    notify("Sober detected. Anti-AFK enabled.");
                }
            } else {
                sober_absent_ticks += 1;
                if sober_absent_ticks >= 2 {
                    if is_running {
                        let mut s = state_auto.lock().unwrap();
                        s.running = false;
                        s.manually_stopped = false;
                        notify("Sober closed. Anti-AFK disabled.");
                    } else if manually_stopped {
                        state_auto.lock().unwrap().manually_stopped = false;
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
                (s.fps_capper, s.fps_limit, s.stop_limit_on_focus, s.running, s.action_active)
            };

            if enabled && is_running && fps_limit > 0 && !action_active {
                let mut current_target_scopes = HashSet::new();
                let main_pids = get_all_sober_pids(&my_pid);
                for pid in &main_pids {
                    if let Some(scope) = get_systemd_scope(pid) {
                        if scope.contains("app") || scope.contains("sober") || scope.contains("vinegar") {
                            current_target_scopes.insert(scope);
                        }
                    }
                }

                if stop_on_focus && is_focused_sober() {
                    reset_scopes(&mut active_scopes);
                    thread::sleep(Duration::from_millis(500));
                    continue;
                }

                let quota = fps_limit.clamp(3, 99);
                active_scopes.retain(|s, _| {
                    if current_target_scopes.contains(s) { true }
                    else { set_cpu_limit(s, ""); false }
                });

                for scope in &current_target_scopes {
                    if active_scopes.get(scope) != Some(&quota) {
                        set_cpu_limit(scope, &format!("{quota}%"));
                        active_scopes.insert(scope.clone(), quota);
                    }
                }
                thread::sleep(Duration::from_secs(1));
            } else {
                reset_scopes(&mut active_scopes);
                thread::sleep(Duration::from_millis(500));
            }
        }
    });

    let state_main = state.clone();
    thread::spawn(move || {
        loop {
            let (is_running, mode) = {
                let s = state_main.lock().unwrap();
                (s.running, s.mode)
            };

            if is_running {
                let res = match mode {
                    0 => crate::inputs::swapper::run(&state_main),
                    1 => crate::inputs::plasma::run(&state_main),
                    _ => Ok(()),
                };

                if let Err(e) = res {
                    let mut s = state_main.lock().unwrap();
                    s.running = false;
                    notify(&format!("Error: {e}"));
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}

fn notify(msg: &str) {
    let _ = Notification::new()
        .appname(APP_ID)
        .summary("AntiAFK-RBX")
        .body(msg)
        .icon(APP_ID)
        .timeout(5000)
        .show();
}

fn is_focused_sober() -> bool {
    if is_hyprland() {
        if let Ok(out) = Command::new("hyprctl").args(["activewindow", "-j"]).output() {
            if let Ok(json) = serde_json::from_slice::<Value>(&out.stdout) {
                let class = json.get("class").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                return class.contains("sober") || class.contains("roblox") || class.contains("vinegar");
            }
        }
    } else if is_plasma() {
        let qdbus = if Command::new("qdbus6").arg("--version").status().is_ok() { "qdbus6" } else { "qdbus" };
        let script = r#"
            var w = workspace.activeWindow;
            if (w) {
                var cls = (w.resourceClass || "").toLowerCase();
                var title = (w.caption || "").toLowerCase();
                var app = (w.desktopFileName || "").toLowerCase();
                var isSober = (cls.indexOf("sober") !== -1 || cls.indexOf("roblox") !== -1 || 
                               cls.indexOf("vinegar") !== -1 || app.indexOf("sober") !== -1) && 
                               title.indexOf("antiafk") === -1;
                print("ANTIAFK_FOCUS_STATE:" + isSober);
            } else {
                print("ANTIAFK_FOCUS_STATE:false");
            }
        "#;
        
        let script_path = "/tmp/antiafk_backend_focus.js";
        let _ = std::fs::write(script_path, script);
        
        let _ = Command::new(qdbus).args(["org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting.unloadScript", "antiafk_backend_focus"]).output();
        
        if let Ok(out) = Command::new(qdbus).args(["org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting.loadScript", script_path, "antiafk_backend_focus"]).output() {
            let id_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Ok(id) = id_str.parse::<i32>() {
                let script_obj = format!("/Scripting/Script{}", id);
                let _ = Command::new(qdbus).args(["org.kde.KWin", &script_obj, "org.kde.kwin.Script.run"]).output();
                
                thread::sleep(Duration::from_millis(200));
                
                if let Ok(output) = Command::new("journalctl")
                    .args(["--user", "-n", "30", "--since", "10 seconds ago", "--no-pager", "-o", "cat"])
                    .output()
                {
                    let text = String::from_utf8_lossy(&output.stdout);
                    for line in text.lines().rev() {
                        if let Some(pos) = line.find("ANTIAFK_FOCUS_STATE:") {
                            return line[pos + 20..].trim() == "true";
                        }
                    }
                }
            }
        }
    }
    false
}

fn set_cpu_limit(scope: &str, limit: &str) {
    let val = if limit.is_empty() { "" } else { limit };
    let _ = Command::new("systemctl").args(["--user", "set-property", scope, &format!("CPUQuota={val}")]).output();
}

fn reset_scopes(scopes: &mut HashMap<String, u32>) {
    for scope in scopes.keys() { set_cpu_limit(scope, ""); }
    scopes.clear();
}

fn check_sober_running() -> bool {
    !get_all_sober_pids(&std::process::id().to_string()).is_empty()
}

fn get_systemd_scope(pid: &str) -> Option<String> {
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
    for line in cgroup.lines() {
        if let Some(path) = line.split("::").nth(1) {
            if let Some(scope) = path.split('/').next_back() {
                if scope.ends_with(".scope") || scope.ends_with(".service") {
                    return Some(scope.to_string());
                }
            }
        }
    }
    None
}

fn get_all_sober_pids(exclude_pid: &str) -> Vec<String> {
    let output = Command::new("pgrep").args(["-if", "sober|roblox|vinegar|Sober.bin"]).output().ok();
    if let Some(out) = output {
        let s = String::from_utf8_lossy(&out.stdout);
        let my_pid = exclude_pid.to_string();
        return s.lines().map(|l| l.trim().to_string()).filter(|p| !p.is_empty() && p != &my_pid).collect();
    }
    Vec::new()
}

pub fn is_hyprland() -> bool {
    crate::state::AppState::is_hyprland()
}
pub fn is_plasma() -> bool {
    crate::state::AppState::is_plasma()
}
