use crate::environment::InputMode;
use crate::input::create_keyboard_device;
use crate::inputs::common::{
    click_pointer, create_pointer_device, find_qdbus, has_recent_input,
    perform_keyboard_actions_with_durations, reconnect_button, reconnect_pixel_match,
    responsive_sleep_interval, warp_cursor,
};
use crate::state::{APP_SLUG, RuntimeStatus, SELF_MARKER, SharedState, set_runtime_status};
use image::GenericImageView;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
const TARGET_SCRIPT: &str = r#"
    var isTarget = function (w) {
        if (!w) { return false; }
        var cls = (w.resourceClass || "").toLowerCase();
        var title = (w.caption || "").toLowerCase();
        var app = (w.desktopFileName || "").toLowerCase();
        return (cls.indexOf("sober") !== -1 || cls.indexOf("roblox") !== -1 ||
            app.indexOf("sober") !== -1) && title.indexOf("{self_marker}") === -1;
    };
    var targets = function () {
        var windows = workspace.windowList();
        var found = [];
        for (var i = 0; i < windows.length; i++) {
            if (isTarget(windows[i])) { found.push(windows[i]); }
        }
        return found;
    };
"#;

fn target_script() -> String {
    TARGET_SCRIPT.replace("{self_marker}", SELF_MARKER)
}

struct Geometry {
    center_x: i32,
    center_y: i32,
    width: i32,
    height: i32,
    screen_width: i32,
    screen_height: i32,
    was_minimized: bool,
}

fn next_nonce() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let clock = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_nanos() as u64)
        .unwrap_or_default();
    clock
        ^ COUNTER
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

pub fn run(state_arc: &SharedState) -> Result<(), String> {
    let qdbus = find_qdbus().ok_or("Neither qdbus6 nor qdbus found.")?;
    let mut kb_device = create_keyboard_device()?;
    let mut pointer = create_pointer_device()?;

    thread::sleep(Duration::from_millis(500));

    loop {
        let s = { state_arc.lock().unwrap().clone() };
        if !s.running {
            if s.stealth {
                unminimize_all_target_windows(&qdbus);
            }
            set_runtime_status(state_arc, RuntimeStatus::Stopped);
            break;
        }
        if s.input_mode() != InputMode::Kde {
            break;
        }

        if s.user_safe && (has_recent_input(3) || is_user_active_cursor(&qdbus, 3)) {
            set_runtime_status(state_arc, RuntimeStatus::Paused);
            thread::sleep(Duration::from_secs(2));
            continue;
        }

        let instance_count = if s.multi_instance {
            get_target_window_count(&qdbus).max(1)
        } else {
            1
        };

        if get_target_window_count(&qdbus) == 0 {
            set_runtime_status(state_arc, RuntimeStatus::WaitingForSober);
            thread::sleep(Duration::from_secs(3));
            continue;
        }

        {
            let mut state = state_arc.lock().unwrap();
            state.action_active = true;
            state.runtime_status = RuntimeStatus::PerformingAction;
        }
        thread::sleep(Duration::from_secs(1));

        let initial_pos = get_current_cursor_pos(&qdbus);
        let initial_window = get_active_window_internal_id(&qdbus);

        for i in 0..instance_count {
            if s.user_safe && (has_recent_input(1) || is_user_active_cursor(&qdbus, 1)) {
                break;
            }

            let geometry = focus_and_get_geometry(&qdbus, i);
            thread::sleep(Duration::from_millis(300));

            if let Some(geometry) = geometry.as_ref() {
                warp_cursor(
                    &mut pointer,
                    geometry.center_x,
                    geometry.center_y,
                    geometry.screen_width,
                    geometry.screen_height,
                );
                thread::sleep(Duration::from_millis(150));
            }

            perform_keyboard_actions_with_durations(
                &mut kb_device,
                &s,
                Duration::from_millis(50),
                Duration::from_millis(200),
                Duration::from_millis(50),
            )?;

            if s.auto_reconnect
                && let Some(geometry) = geometry.as_ref()
                && let Some((r, g, b)) = get_pixel_color()
                && reconnect_pixel_match(r, g, b)
            {
                let window_x = geometry.center_x - geometry.width / 2;
                let window_y = geometry.center_y - geometry.height / 2;
                let (click_x, click_y) =
                    reconnect_button(window_x, window_y, geometry.width, geometry.height);
                warp_cursor(
                    &mut pointer,
                    click_x,
                    click_y,
                    geometry.screen_width,
                    geometry.screen_height,
                );
                thread::sleep(Duration::from_millis(200));
                for _ in 0..3 {
                    click_pointer(&mut pointer, Duration::from_millis(50));
                    thread::sleep(Duration::from_millis(100));
                }
                warp_cursor(
                    &mut pointer,
                    geometry.center_x,
                    geometry.center_y,
                    geometry.screen_width,
                    geometry.screen_height,
                );
            }

            if s.stealth || geometry.is_some_and(|geometry| geometry.was_minimized) {
                minimize_window_by_index(&qdbus, i);
            }
        }

        if let Some(win_id) = initial_window {
            restore_active_window_by_id(&qdbus, &win_id);
            thread::sleep(Duration::from_millis(100));
        }

        if let Some((orig_x, orig_y, screen_w, screen_h)) = initial_pos {
            warp_cursor(&mut pointer, orig_x, orig_y, screen_w, screen_h);
            thread::sleep(Duration::from_millis(100));
        }

        {
            let mut state = state_arc.lock().unwrap();
            state.action_active = false;
            state.runtime_status = RuntimeStatus::Ready;
        }

        if responsive_sleep_interval(state_arc, InputMode::Kde) {
            if !state_arc.lock().unwrap().running {
                set_runtime_status(state_arc, RuntimeStatus::Stopped);
            }
            break;
        }
    }
    Ok(())
}

fn get_current_cursor_pos(qdbus: &str) -> Option<(i32, i32, i32, i32)> {
    let value = run_query(qdbus, |marker| {
        format!(
            r#"
            var cp = workspace.cursorPos;
            var vs = workspace.virtualScreenSize;
            print("{marker}" + Math.round(cp.x) + "," + Math.round(cp.y) + "," + vs.width + "," + vs.height);
            "#
        )
    })?;
    let parts = parse_numbers(&value, 4)?;
    Some((parts[0], parts[1], parts[2], parts[3]))
}

fn get_active_window_internal_id(qdbus: &str) -> Option<String> {
    run_query(qdbus, |marker| {
        format!(
            r#"
            var w = workspace.activeWindow;
            if (w) {{ print("{marker}" + w.internalId); }}
            "#
        )
    })
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
}

fn restore_active_window_by_id(qdbus: &str, win_id: &str) {
    run_query(qdbus, |marker| {
        format!(
            r#"
            var windows = workspace.windowList();
            for (var i = 0; i < windows.length; i++) {{
                if (windows[i].internalId == "{win_id}") {{
                    workspace.activeWindow = windows[i];
                    break;
                }}
            }}
            print("{marker}ok");
            "#,
            win_id = win_id
        )
    });
}

fn is_user_active_cursor(qdbus: &str, secs: u64) -> bool {
    let p1 = get_current_cursor_pos(qdbus);
    thread::sleep(Duration::from_secs(secs));
    let p2 = get_current_cursor_pos(qdbus);

    match (p1, p2) {
        (Some((x1, y1, _, _)), Some((x2, y2, _, _))) => (x1 - x2).abs() > 2 || (y1 - y2).abs() > 2,
        _ => false,
    }
}

fn focus_and_get_geometry(qdbus: &str, index: usize) -> Option<Geometry> {
    let value = run_query(qdbus, |marker| {
        format!(
            r#"
            {target}
            var found = targets();
            if (found.length > {index}) {{
                var target = found[{index}];
                var wasMinimized = target.minimized ? 1 : 0;
                if (target.minimized) {{ target.minimized = false; }}
                workspace.activeWindow = target;
                var geo = target.frameGeometry;
                var vs = workspace.virtualScreenSize;
                print("{marker}" + Math.round(geo.x + geo.width / 2) + "," +
                    Math.round(geo.y + geo.height / 2) + "," +
                    Math.round(geo.width) + "," + Math.round(geo.height) + "," +
                    vs.width + "," + vs.height + "," + wasMinimized);
            }}
            "#,
            target = target_script(),
            index = index,
            marker = marker
        )
    })?;
    let parts = parse_numbers(&value, 7)?;
    Some(Geometry {
        center_x: parts[0],
        center_y: parts[1],
        width: parts[2],
        height: parts[3],
        screen_width: parts[4],
        screen_height: parts[5],
        was_minimized: parts[6] != 0,
    })
}

fn minimize_window_by_index(qdbus: &str, index: usize) {
    run_query(qdbus, |marker| {
        format!(
            r#"
            {target}
            var found = targets();
            if (found.length > {index}) {{
                found[{index}].minimized = true;
            }}
            print("{marker}ok");
            "#,
            target = target_script(),
            index = index,
            marker = marker
        )
    });
}

pub fn show_sober() -> Result<(), String> {
    let qdbus = find_qdbus().ok_or("qdbus6 or qdbus is not available")?;
    unminimize_all_target_windows(&qdbus);
    if focus_and_get_geometry(&qdbus, 0).is_none() {
        return Err("Sober window was not found".to_string());
    }
    Ok(())
}

pub fn hide_sober() -> Result<(), String> {
    let qdbus = find_qdbus().ok_or("qdbus6 or qdbus is not available")?;
    if get_target_window_count(&qdbus) == 0 {
        return Err("Sober window was not found".to_string());
    }
    minimize_window_by_index(&qdbus, 0);
    Ok(())
}

pub fn focused_is_sober() -> Option<bool> {
    let qdbus = find_qdbus()?;
    let value = run_query(&qdbus, |marker| {
        format!(
            r#"
            {target}
            var active = workspace.activeWindow;
            print("{marker}" + (isTarget(active) ? "1" : "0"));
            "#,
            target = target_script(),
            marker = marker
        )
    })?;
    match value.trim() {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

pub fn script_bridge() -> Result<(), String> {
    let qdbus = find_qdbus().ok_or("qdbus6 or qdbus is not available")?;
    match run_query(&qdbus, |marker| format!("print(\"{marker}ok\");")) {
        Some(_) => Ok(()),
        None => Err(SCRIPT_LOGGING_HINT.to_string()),
    }
}

pub const SCRIPT_LOGGING_HINT: &str = "KWin does not forward script output to the journal. Run systemctl --user set-environment QT_LOGGING_RULES=kwin_*.debug=true and log out and back in.";

pub fn enable_script_logging() -> Result<(), String> {
    let output = Command::new("systemctl")
        .args([
            "--user",
            "set-environment",
            "QT_LOGGING_RULES=kwin_*.debug=true",
        ])
        .output()
        .map_err(|error| format!("systemctl failed: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "systemctl could not set the environment: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn parse_numbers(value: &str, expected: usize) -> Option<Vec<i32>> {
    let numbers = value
        .split(',')
        .map(|part| part.trim().parse::<i32>())
        .collect::<Result<Vec<i32>, _>>()
        .ok()?;
    if numbers.len() < expected {
        return None;
    }
    Some(numbers)
}

static JOURNAL_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn journal_cache() -> &'static Mutex<HashMap<String, String>> {
    JOURNAL_CACHE.get_or_init(|| {
        let cache = Mutex::new(HashMap::new());
        thread::Builder::new()
            .name("antiafk-kde-journal".into())
            .spawn(|| {
                let mut child = match Command::new("journalctl")
                    .args(["--user", "-f", "-n", "0", "-o", "cat"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    Ok(c) => c,
                    Err(_) => return,
                };
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines().map_while(Result::ok) {
                        if let Some(pos) = line.find("ANTIAFK") {
                            let sub = &line[pos..];
                            if let Some((prefix, val)) = sub.split_once(':') {
                                let marker = format!("{prefix}:");
                                if let Ok(mut map) = journal_cache().lock() {
                                    map.insert(marker, val.trim().to_string());
                                    if map.len() > 100 {
                                        map.clear();
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .ok();
        cache
    })
}

fn run_query(qdbus: &str, script: impl FnOnce(&str) -> String) -> Option<String> {
    let _ = journal_cache();
    let marker = format!("ANTIAFK{}:", next_nonce());
    let body = script(&marker);
    run_kwin_script(qdbus, &body);
    read_journal_value(&marker)
}

fn read_journal_value(marker: &str) -> Option<String> {
    let deadline = Instant::now() + Duration::from_millis(300);
    while Instant::now() < deadline {
        if let Ok(mut cache) = journal_cache().lock()
            && let Some(val) = cache.remove(marker)
        {
            return Some(val);
        }
        thread::sleep(Duration::from_millis(15));
    }

    let output = Command::new("journalctl")
        .args([
            "--user",
            "-n",
            "50",
            "--since",
            "20 seconds ago",
            "--no-pager",
            "-o",
            "cat",
        ])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines().rev() {
        if let Some(position) = line.find(marker) {
            return Some(line[position + marker.len()..].trim().to_string());
        }
    }
    None
}

fn run_kwin_script(qdbus: &str, script: &str) {
    let nonce = next_nonce();
    let script_name = format!("{SELF_MARKER}_{}_{}", std::process::id(), nonce);
    let script_path = std::env::temp_dir().join(format!("{script_name}.js"));
    if std::fs::write(&script_path, script).is_err() {
        return;
    }
    let script_path_text = script_path.to_string_lossy().to_string();

    let _ = Command::new(qdbus)
        .args([
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting.unloadScript",
            &script_name,
        ])
        .output();

    let output = Command::new(qdbus)
        .args([
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting.loadScript",
            &script_path_text,
            &script_name,
        ])
        .output();

    if let Ok(out) = output {
        let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !id.is_empty() {
            let script_obj = format!("/Scripting/Script{id}");

            let _ = Command::new(qdbus)
                .args(["org.kde.KWin", &script_obj, "org.kde.kwin.Script.run"])
                .output();

            thread::sleep(Duration::from_millis(100));

            let _ = Command::new(qdbus)
                .args(["org.kde.KWin", &script_obj, "org.kde.kwin.Script.stop"])
                .output();
        }
    }

    let _ = Command::new(qdbus)
        .args([
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting.unloadScript",
            &script_name,
        ])
        .output();
    let _ = std::fs::remove_file(&script_path);
}

fn get_target_window_count(qdbus: &str) -> usize {
    let value = run_query(qdbus, |marker| {
        format!(
            r#"
            {target}
            print("{marker}" + targets().length);
            "#,
            target = target_script(),
            marker = marker
        )
    });
    value
        .as_deref()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0)
}

fn unminimize_all_target_windows(qdbus: &str) {
    run_query(qdbus, |marker| {
        format!(
            r#"
            {target}
            var found = targets();
            for (var i = 0; i < found.length; i++) {{
                if (found[i].minimized) {{ found[i].minimized = false; }}
            }}
            print("{marker}ok");
            "#,
            target = target_script(),
            marker = marker
        )
    });
}

fn get_pixel_color() -> Option<(u8, u8, u8)> {
    let tmp_path = std::env::temp_dir().join(format!("{APP_SLUG}-spectacle.png"));
    let tmp_arg = tmp_path.to_string_lossy().into_owned();
    let _ = std::fs::remove_file(&tmp_path);

    let status = Command::new("spectacle")
        .args(["-b", "-n", "-a", "-o", &tmp_arg])
        .status()
        .ok()?;

    if !status.success() {
        return None;
    }

    let img = image::open(&tmp_path).ok()?;
    let _ = std::fs::remove_file(&tmp_path);

    let (width, height) = img.dimensions();
    let step = 15;

    for y in (height * 3 / 10..height * 7 / 10).step_by(step) {
        for x in (width * 3 / 10..width * 7 / 10).step_by(step) {
            let pixel = img.get_pixel(x, y);
            if reconnect_pixel_match(pixel[0], pixel[1], pixel[2]) {
                return Some((pixel[0], pixel[1], pixel[2]));
            }
        }
    }

    None
}
