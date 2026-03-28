use crate::input::{create_keyboard_device, emit_key};
use crate::state::SharedState;
use evdev::{
    AbsInfo, AbsoluteAxisType, EventType, InputEvent, Key, RelativeAxisType, UinputAbsSetup,
    uinput::VirtualDevice, uinput::VirtualDeviceBuilder,
};
use image::GenericImageView;
use std::process::Command;
use std::thread;
use std::time::Duration;

const ABS_MAX: i32 = 65535;

pub fn run(state_arc: &SharedState) -> Result<(), String> {
    let qdbus = find_qdbus().ok_or("Neither qdbus6 nor qdbus found.")?;
    let mut kb_device = create_keyboard_device()?;
    let mut pointer = create_pointer_device()?;

    thread::sleep(Duration::from_millis(500));

    loop {
        let s = { state_arc.lock().unwrap().clone() };
        if !s.running || s.mode != 1 {
            if s.stealth {
                unminimize_all_target_windows(&qdbus);
            }
            break;
        }

        if s.user_safe && is_user_active_cursor(&qdbus, 3) {
            thread::sleep(Duration::from_secs(2));
            continue;
        }

        let instance_count = if s.multi_instance {
            get_target_window_count(&qdbus).max(1)
        } else {
            1
        };

        if get_target_window_count(&qdbus) == 0 {
            thread::sleep(Duration::from_secs(3));
            continue;
        }

        {
            let mut state = state_arc.lock().unwrap();
            state.action_active = true;
        }

        let initial_pos = get_current_cursor_pos(&qdbus);
        let initial_window = get_active_window_internal_id(&qdbus);

        for i in 0..instance_count {
            if s.user_safe && is_user_active_cursor(&qdbus, 1) {
                break;
            }

            let geo = focus_and_get_geometry(&qdbus, i);
            thread::sleep(Duration::from_millis(300));

            if let Some((target_x, target_y, _win_w, _win_h, screen_w, screen_h)) = geo {
                warp_cursor(&mut pointer, target_x, target_y, screen_w, screen_h);
                thread::sleep(Duration::from_millis(150));
            }

            if s.jump {
                let _ = emit_key(&mut kb_device, Key::KEY_SPACE, true);
                thread::sleep(Duration::from_millis(50));
                let _ = emit_key(&mut kb_device, Key::KEY_SPACE, false);
            }

            if s.walk {
                let _ = emit_key(&mut kb_device, Key::KEY_W, true);
                thread::sleep(Duration::from_millis(200));
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

            if s.auto_reconnect {
                if let Some((target_x, target_y, win_w, win_h, screen_w, screen_h)) = geo {
                    if let Some((r, g, b, _px, _py)) = get_pixel_color(win_w as i64, win_h as i64) {
                        let target_r = 57i16;
                        let target_g = 59i16;
                        let target_b = 61i16;
                        let diff = (r as i16 - target_r).abs()
                            + (g as i16 - target_g).abs()
                            + (b as i16 - target_b).abs();

                        if diff < 15 {
                            let click_x = target_x - (win_w / 2)
                                + (win_w - 400) / 2
                                + (400 - 161 - 27)
                                + (161 / 2);
                            let click_y = target_y - (win_h / 2)
                                + (win_h - 250) / 2
                                + (250 - 34 - 21)
                                + (34 / 2);
                            warp_cursor(&mut pointer, click_x, click_y, screen_w, screen_h);
                            thread::sleep(Duration::from_millis(200));
                            for _ in 0..3 {
                                let _ = pointer.emit(&[
                                    InputEvent::new(EventType::KEY, Key::BTN_LEFT.0, 1),
                                    InputEvent::new(EventType::SYNCHRONIZATION, 0, 0),
                                ]);
                                thread::sleep(Duration::from_millis(50));
                                let _ = pointer.emit(&[
                                    InputEvent::new(EventType::KEY, Key::BTN_LEFT.0, 0),
                                    InputEvent::new(EventType::SYNCHRONIZATION, 0, 0),
                                ]);
                                thread::sleep(Duration::from_millis(100));
                            }
                            warp_cursor(&mut pointer, target_x, target_y, screen_w, screen_h);
                        }
                    }
                }
            }

            if s.stealth {
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
            state_arc.lock().unwrap().action_active = false;
        }

        if responsive_sleep(state_arc, 1) {
            break;
        }
    }
    Ok(())
}

fn create_pointer_device() -> Result<VirtualDevice, String> {
    let abs_x = UinputAbsSetup::new(
        AbsoluteAxisType::ABS_X,
        AbsInfo::new(0, 0, ABS_MAX, 0, 0, 0),
    );
    let abs_y = UinputAbsSetup::new(
        AbsoluteAxisType::ABS_Y,
        AbsInfo::new(0, 0, ABS_MAX, 0, 0, 0),
    );

    let mut keys = evdev::AttributeSet::<Key>::new();
    keys.insert(Key::BTN_LEFT);

    let mut rel = evdev::AttributeSet::<RelativeAxisType>::new();
    rel.insert(RelativeAxisType::REL_X);
    rel.insert(RelativeAxisType::REL_Y);

    VirtualDeviceBuilder::new()
        .map_err(|e: std::io::Error| e.to_string())?
        .name("AntiAFK Virtual Pointer")
        .with_keys(&keys)
        .map_err(|e: std::io::Error| e.to_string())?
        .with_relative_axes(&rel)
        .map_err(|e: std::io::Error| e.to_string())?
        .with_absolute_axis(&abs_x)
        .map_err(|e: std::io::Error| e.to_string())?
        .with_absolute_axis(&abs_y)
        .map_err(|e: std::io::Error| e.to_string())?
        .build()
        .map_err(|e: std::io::Error| {
            format!("Pointer device creation failed: {e}. Run: sudo chmod 666 /dev/uinput")
        })
}

fn warp_cursor(device: &mut VirtualDevice, x: i32, y: i32, screen_w: i32, screen_h: i32) {
    if screen_w <= 0 || screen_h <= 0 {
        return;
    }
    let abs_x = (x as i64 * ABS_MAX as i64 / screen_w as i64).clamp(0, ABS_MAX as i64) as i32;
    let abs_y = (y as i64 * ABS_MAX as i64 / screen_h as i64).clamp(0, ABS_MAX as i64) as i32;

    let _ = device.emit(&[
        InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_X.0, abs_x),
        InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_Y.0, abs_y),
        InputEvent::new(EventType::SYNCHRONIZATION, 0, 0),
    ]);
}

fn get_current_cursor_pos(qdbus: &str) -> Option<(i32, i32, i32, i32)> {
    let script = r#"
        var cp = workspace.cursorPos;
        var vs = workspace.virtualScreenSize;
        print("ANTIAFK_POS:" + Math.round(cp.x) + "," + Math.round(cp.y) + "," + vs.width + "," + vs.height);
    "#;

    run_kwin_script(qdbus, script);
    thread::sleep(Duration::from_millis(200));

    if let Ok(output) = Command::new("journalctl")
        .args([
            "--user",
            "-n",
            "30",
            "--since",
            "10 seconds ago",
            "--no-pager",
            "-o",
            "cat",
        ])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines().rev() {
            if let Some(pos) = line.find("ANTIAFK_POS:") {
                let data = &line[pos + 12..];
                let parts: Vec<&str> = data.split(',').collect();
                if parts.len() >= 4 {
                    if let (Some(x), Some(y), Some(w), Some(h)) = (
                        parts[0].trim().parse::<i32>().ok(),
                        parts[1].trim().parse::<i32>().ok(),
                        parts[2].trim().parse::<i32>().ok(),
                        parts[3].trim().parse::<i32>().ok(),
                    ) {
                        return Some((x, y, w, h));
                    }
                }
            }
        }
    }
    None
}

fn get_active_window_internal_id(qdbus: &str) -> Option<String> {
    let script = r#"
        var w = workspace.activeWindow;
        if (w) {
            print("ANTIAFK_WIN_ID:" + w.internalId);
        }
    "#;
    run_kwin_script(qdbus, script);
    thread::sleep(Duration::from_millis(200));

    if let Ok(output) = Command::new("journalctl")
        .args([
            "--user",
            "-n",
            "20",
            "--since",
            "5 seconds ago",
            "--no-pager",
            "-o",
            "cat",
        ])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines().rev() {
            if let Some(pos) = line.find("ANTIAFK_WIN_ID:") {
                return Some(line[pos + 15..].trim().to_string());
            }
        }
    }
    None
}

fn restore_active_window_by_id(qdbus: &str, win_id: &str) {
    let script = format!(
        r#"
        var windows = workspace.windowList();
        for (var i = 0; i < windows.length; i++) {{
            if (windows[i].internalId == "{id}") {{
                workspace.activeWindow = windows[i];
                break;
            }}
        }}
    "#,
        id = win_id
    );
    run_kwin_script(qdbus, &script);
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

fn focus_and_get_geometry(qdbus: &str, index: usize) -> Option<(i32, i32, i32, i32, i32, i32)> {
    let script = format!(
        r#"
        var windows = workspace.windowList();
        var targets = [];
        for (var i = 0; i < windows.length; i++) {{
            var w = windows[i];
            var cls = (w.resourceClass || "").toLowerCase();
            var title = (w.caption || "").toLowerCase();
            var app = (w.desktopFileName || "").toLowerCase();
            if ((cls.indexOf("sober") !== -1 || cls.indexOf("roblox") !== -1 ||
                cls.indexOf("vinegar") !== -1 || app.indexOf("sober") !== -1) &&
                title.indexOf("antiafk") === -1) {{
                targets.push(w);
            }}
        }}
        if (targets.length > {idx}) {{
            var target = targets[{idx}];
            if (target.minimized) {{
                target.minimized = false;
            }}
            workspace.activeWindow = target;
            var geo = target.frameGeometry;
            var vs = workspace.virtualScreenSize;
            var cx = Math.round(geo.x + geo.width / 2);
            var cy = Math.round(geo.y + geo.height / 2);
            print("ANTIAFK_GEO:" + cx + "," + cy + "," + Math.round(geo.width) + "," + Math.round(geo.height) + "," + vs.width + "," + vs.height);
        }}
    "#,
        idx = index
    );

    run_kwin_script(qdbus, &script);
    thread::sleep(Duration::from_millis(300));

    if let Ok(output) = Command::new("journalctl")
        .args([
            "--user",
            "-n",
            "50",
            "--since",
            "10 seconds ago",
            "--no-pager",
            "-o",
            "cat",
        ])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines().rev() {
            if let Some(pos) = line.find("ANTIAFK_GEO:") {
                let data = &line[pos + 12..];
                let parts: Vec<&str> = data.split(',').collect();
                if parts.len() >= 6 {
                    if let (Some(cx), Some(cy), Some(ww), Some(wh), Some(sw), Some(sh)) = (
                        parts[0].trim().parse::<i32>().ok(),
                        parts[1].trim().parse::<i32>().ok(),
                        parts[2].trim().parse::<i32>().ok(),
                        parts[3].trim().parse::<i32>().ok(),
                        parts[4].trim().parse::<i32>().ok(),
                        parts[5].trim().parse::<i32>().ok(),
                    ) {
                        return Some((cx, cy, ww, wh, sw, sh));
                    }
                }
            }
        }
    }
    None
}

fn minimize_window_by_index(qdbus: &str, index: usize) {
    let script = format!(
        r#"
        var windows = workspace.windowList();
        var targets = [];
        for (var i = 0; i < windows.length; i++) {{
            var w = windows[i];
            var cls = (w.resourceClass || "").toLowerCase();
            var title = (w.caption || "").toLowerCase();
            var app = (w.desktopFileName || "").toLowerCase();
            if ((cls.indexOf("sober") !== -1 || cls.indexOf("roblox") !== -1 ||
                cls.indexOf("vinegar") !== -1 || app.indexOf("sober") !== -1) &&
                title.indexOf("antiafk") === -1) {{
                targets.push(w);
            }}
        }}
        if (targets.length > {idx}) {{
            targets[{idx}].minimized = true;
        }}
    "#,
        idx = index
    );
    run_kwin_script(qdbus, &script);
}

fn find_qdbus() -> Option<String> {
    if Command::new("qdbus6").arg("--version").output().is_ok() {
        Some("qdbus6".to_string())
    } else if Command::new("qdbus").arg("--version").output().is_ok() {
        Some("qdbus".to_string())
    } else {
        None
    }
}

fn run_kwin_script(qdbus: &str, script: &str) {
    let script_path = "/tmp/antiafk_kwin_focus.js";
    if std::fs::write(script_path, script).is_err() {
        return;
    }

    let _ = Command::new(qdbus)
        .args([
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting.unloadScript",
            "antiafk_focus",
        ])
        .output();

    let output = Command::new(qdbus)
        .args([
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting.loadScript",
            script_path,
            "antiafk_focus",
        ])
        .output();

    if let Ok(out) = output {
        let id_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if let Ok(id) = id_str.parse::<i32>() {
            let script_obj = format!("/Scripting/Script{}", id);

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
            "antiafk_focus",
        ])
        .output();
    let _ = std::fs::remove_file(script_path);
}

fn responsive_sleep(state_arc: &SharedState, mode: usize) -> bool {
    let interval = { state_arc.lock().unwrap().interval_seq };
    if interval == 0 {
        return false;
    }
    for _ in 0..interval {
        thread::sleep(Duration::from_secs(1));
        let s = { state_arc.lock().unwrap().clone() };
        if !s.running || s.mode != mode {
            return true;
        }
    }
    false
}

fn get_target_window_count(qdbus: &str) -> usize {
    let script = r#"
        var windows = workspace.windowList();
        var count = 0;
        for (var i = 0; i < windows.length; i++) {
            var w = windows[i];
            var cls = (w.resourceClass || "").toLowerCase();
            var title = (w.caption || "").toLowerCase();
            var app = (w.desktopFileName || "").toLowerCase();
            if ((cls.indexOf("sober") !== -1 || cls.indexOf("roblox") !== -1 ||
                cls.indexOf("vinegar") !== -1 || app.indexOf("sober") !== -1) &&
                title.indexOf("antiafk") === -1) {
                count++;
            }
        }
        print("ANTIAFK_COUNT:" + count);
    "#;
    run_kwin_script(qdbus, script);
    thread::sleep(Duration::from_millis(200));

    if let Ok(output) = Command::new("journalctl")
        .args([
            "--user",
            "-n",
            "20",
            "--since",
            "5 seconds ago",
            "--no-pager",
            "-o",
            "cat",
        ])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines().rev() {
            if let Some(pos) = line.find("ANTIAFK_COUNT:") {
                return line[pos + 14..].trim().parse::<usize>().unwrap_or(0);
            }
        }
    }
    0
}

fn unminimize_all_target_windows(qdbus: &str) {
    let script = r#"
        var windows = workspace.windowList();
        for (var i = 0; i < windows.length; i++) {
            var w = windows[i];
            var cls = (w.resourceClass || "").toLowerCase();
            var title = (w.caption || "").toLowerCase();
            var app = (w.desktopFileName || "").toLowerCase();
            if ((cls.indexOf("sober") !== -1 || cls.indexOf("roblox") !== -1 ||
                cls.indexOf("vinegar") !== -1 || app.indexOf("sober") !== -1) &&
                title.indexOf("antiafk") === -1) {
                if (w.minimized) {
                    w.minimized = false;
                }
            }
        }
    "#;
    run_kwin_script(qdbus, script);
}

fn get_pixel_color(_log_w: i64, _log_h: i64) -> Option<(u8, u8, u8, u32, u32)> {
    let tmp_path = "/tmp/antiafk_win_pixel.png";
    let _ = std::fs::remove_file(tmp_path);

    let status = Command::new("spectacle")
        .args(["-b", "-n", "-a", "-o", tmp_path])
        .status()
        .ok()?;

    if !status.success() {
        return None;
    }

    let img = image::open(tmp_path).ok()?;
    let _ = std::fs::remove_file(tmp_path);

    let (phys_w, phys_h) = img.dimensions();

    let start_x = (phys_w as f64 * 0.3) as u32;
    let end_x = (phys_w as f64 * 0.7) as u32;
    let start_y = (phys_h as f64 * 0.3) as u32;
    let end_y = (phys_h as f64 * 0.7) as u32;

    let step = 15;

    for y in (start_y..end_y).step_by(step) {
        for x in (start_x..end_x).step_by(step) {
            let pixel = img.get_pixel(x, y);
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];

            let diff = (r as i16 - 57).abs() + (g as i16 - 59).abs() + (b as i16 - 61).abs();
            if diff < 15 {
                return Some((r, g, b, x, y));
            }
        }
    }

    let cx = phys_w / 2;
    let cy = phys_h / 2;
    let cp = img.get_pixel(cx, cy);
    Some((cp[0], cp[1], cp[2], cx, cy))
}
