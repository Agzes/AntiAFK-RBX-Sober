use crate::GNOME_EXTENSION_UUID;
use crate::environment::{InputMode, forced_desktop};
use crate::input::{create_keyboard_device, create_mouse_device};
use crate::inputs::common::{
    click_mouse, command_output, is_sober_metadata, perform_keyboard_actions,
    reconnect_pixel_match, responsive_sleep, set_action_active, set_status, should_continue,
};
use crate::state::{APP_SLUG, APP_TITLE, RuntimeStatus, SharedState};
use image::GenericImageView;
use serde::Deserialize;
use serde_json::Value;
use std::process::Command;
use std::thread;
use std::time::Duration;

const DBUS_DESTS: &[&str] = &["org.gnome.Shell", "dev.agzes.antiafk"];
const DBUS_PATH: &str = "/dev/agzes/antiafk";
const DBUS_INTERFACE: &str = "dev.agzes.antiafk.Backend";

#[derive(Clone, Debug, Deserialize)]
struct GnomeWindow {
    id: String,
    title: String,
    class: String,
    desktop: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    active: bool,
}

fn unquote_gvariant_string(text: &str) -> Result<String, String> {
    let trimmed = text.trim();
    let Some(start) = trimmed.find('\'') else {
        return Err(format!("unexpected GNOME helper response: {trimmed}"));
    };
    let Some(end) = trimmed.rfind('\'') else {
        return Err(format!("unexpected GNOME helper response: {trimmed}"));
    };
    if end <= start {
        return Err("GNOME helper returned an empty string".to_string());
    }

    let raw = &trimmed[start + 1..end];
    let mut result = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }
        let Some(escaped) = chars.next() else {
            result.push('\\');
            break;
        };
        match escaped {
            'n' => result.push('\n'),
            'r' => result.push('\r'),
            't' => result.push('\t'),
            '\\' => result.push('\\'),
            '\'' => result.push('\''),
            '"' => result.push('"'),
            'u' => {
                let hex: String = chars.by_ref().take(4).collect();
                let code = u32::from_str_radix(&hex, 16)
                    .map_err(|_| "invalid unicode escape in GNOME helper response".to_string())?;
                let character = char::from_u32(code).ok_or_else(|| {
                    "invalid unicode code point in GNOME helper response".to_string()
                })?;
                result.push(character);
            }
            other => {
                result.push('\\');
                result.push(other);
            }
        }
    }
    Ok(result)
}

fn call_helper_dest(dest: &str, method: &str, args: &[String]) -> Result<String, String> {
    let mut command_args = vec![
        "call".to_string(),
        "--session".to_string(),
        "--dest".to_string(),
        dest.to_string(),
        "--object-path".to_string(),
        DBUS_PATH.to_string(),
        "--method".to_string(),
        format!("{DBUS_INTERFACE}.{method}"),
    ];
    command_args.extend_from_slice(args);
    let command_args = command_args.iter().map(String::as_str).collect::<Vec<_>>();
    let output = command_output("gdbus", &command_args)?;
    let trimmed = output.trim();
    if trimmed.starts_with("('") {
        return unquote_gvariant_string(trimmed);
    }
    let value = trimmed
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(trimmed)
        .trim_end_matches(',')
        .trim();
    if value.is_empty() {
        Err("GNOME helper returned an empty response".to_string())
    } else {
        Ok(value.to_string())
    }
}

fn call_helper(method: &str, args: &[String]) -> Result<String, String> {
    let mut preferred_error = None;
    let mut last_error = String::new();
    for dest in DBUS_DESTS {
        match call_helper_dest(dest, method, args) {
            Ok(result) => return Ok(result),
            Err(error) => {
                if error.contains("UnknownObject") || error.contains("UnknownMethod") {
                    preferred_error = Some(error.clone());
                }
                last_error = error;
            }
        }
    }
    Err(preferred_error.unwrap_or(last_error))
}

fn json_argument(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string cannot fail")
}

fn list_windows() -> Result<Vec<GnomeWindow>, String> {
    let payload = call_helper("ListWindows", &[])?;
    serde_json::from_str(&payload)
        .map_err(|error| format!("invalid GNOME helper window list: {error}"))
}

fn focus_window(id: &str) -> Result<bool, String> {
    let value = call_helper("FocusWindow", &[json_argument(id)])?;
    value
        .parse::<bool>()
        .map_err(|_| "GNOME helper returned an invalid focus result".to_string())
}

fn set_minimized(id: &str, minimized: bool) -> Result<bool, String> {
    let value = call_helper("SetMinimized", &[json_argument(id), minimized.to_string()])?;
    value
        .parse::<bool>()
        .map_err(|_| "GNOME helper returned an invalid visibility result".to_string())
}

fn cursor_position() -> Result<(i32, i32), String> {
    let payload = call_helper("PointerPosition", &[])?;
    let value: Value = serde_json::from_str(&payload)
        .map_err(|error| format!("invalid GNOME helper pointer response: {error}"))?;
    let array = value
        .as_array()
        .ok_or_else(|| "GNOME helper pointer response is not an array".to_string())?;
    if array.len() < 2 {
        return Err("GNOME helper pointer response is incomplete".to_string());
    }
    Ok((
        array[0].as_i64().unwrap_or_default() as i32,
        array[1].as_i64().unwrap_or_default() as i32,
    ))
}

fn warp_cursor(x: i32, y: i32) -> Result<(), String> {
    let value = call_helper("WarpPointer", &[x.to_string(), y.to_string()])?;
    if value == "true" {
        Ok(())
    } else {
        Err("GNOME helper rejected the pointer request".to_string())
    }
}

fn user_active() -> Result<bool, String> {
    if crate::inputs::common::has_recent_input(3) {
        return Ok(true);
    }
    let before = cursor_position()?;
    thread::sleep(Duration::from_secs(3));
    if crate::inputs::common::has_recent_input(3) {
        return Ok(true);
    }
    let after = cursor_position()?;
    Ok((before.0 - after.0).abs() > 2 || (before.1 - after.1).abs() > 2)
}

fn capture_gnome_dbus(window: &GnomeWindow, path_str: &str) -> bool {
    let path = std::path::Path::new(path_str);

    if call_helper(
        "CaptureWindowToFile",
        &[window.id.clone(), path_str.to_string()],
    )
    .map(|res| res == "true")
    .unwrap_or(false)
    {
        for _ in 0..10 {
            if path.exists()
                && std::fs::metadata(path)
                    .map(|m| m.len() > 100)
                    .unwrap_or(false)
            {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
    }

    let args_screen = [
        "call",
        "--session",
        "--dest",
        "org.gnome.Shell.Screenshot",
        "--object-path",
        "/org/gnome/Shell/Screenshot",
        "--method",
        "org.gnome.Shell.Screenshot.Screenshot",
        "false",
        "false",
        path_str,
    ];
    command_output("gdbus", &args_screen).is_ok() && path.exists()
}

fn capture_window(window: &GnomeWindow) -> Option<std::path::PathBuf> {
    let path = std::env::temp_dir().join(format!("{APP_SLUG}-gnome-{}.png", window.id));
    let _ = std::fs::remove_file(&path);
    let path_str = path.to_str()?;

    if window.width > 0
        && window.height > 0
        && capture_gnome_dbus(window, path_str)
        && path.exists()
    {
        return Some(path);
    }

    if Command::new("gnome-screenshot")
        .args(["-f", path_str])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && path.exists()
    {
        return Some(path);
    }

    if crate::inputs::common::capture_wayland_screen(&path).is_ok() && path.exists() {
        return Some(path);
    }

    None
}

fn reconnect_click(window: &GnomeWindow) -> Result<bool, String> {
    let Some(path) = capture_window(window) else {
        return Ok(false);
    };
    let result = image::open(&path)
        .ok()
        .map(|image| {
            let (width, height) = image.dimensions();
            if width < 50 || height < 50 {
                return false;
            }
            let is_fullscreen =
                width > window.width as u32 + 20 || height > window.height as u32 + 20;
            let (cx, cy) = if is_fullscreen {
                (window.x + window.width / 2, window.y + window.height / 2)
            } else {
                ((width / 2) as i32, (height / 2) as i32)
            };

            let (probe_x, probe_y) = if is_fullscreen {
                crate::inputs::common::reconnect_probe(
                    window.x,
                    window.y,
                    window.width,
                    window.height,
                )
            } else {
                crate::inputs::common::reconnect_probe(0, 0, width as i32, height as i32)
            };

            let mut probe_matches = 0;
            let mut probe_total = 0;
            for dy in -5..=5 {
                for dx in -5..=5 {
                    let px = (probe_x + dx).clamp(0, width as i32 - 1) as u32;
                    let py = (probe_y + dy).clamp(0, height as i32 - 1) as u32;
                    let pixel = image.get_pixel(px, py);
                    if reconnect_pixel_match(pixel[0], pixel[1], pixel[2]) {
                        probe_matches += 1;
                    }
                    probe_total += 1;
                }
            }

            let mut center_matches = 0;
            let mut center_total = 0;
            for dy in (-30..=30).step_by(5) {
                for dx in (-30..=30).step_by(5) {
                    let px = (cx + dx).clamp(0, width as i32 - 1) as u32;
                    let py = (cy + dy).clamp(0, height as i32 - 1) as u32;
                    let pixel = image.get_pixel(px, py);
                    if reconnect_pixel_match(pixel[0], pixel[1], pixel[2]) {
                        center_matches += 1;
                    }
                    center_total += 1;
                }
            }

            (probe_total > 0 && probe_matches >= probe_total * 30 / 100)
                || (center_total > 0 && center_matches >= center_total * 25 / 100)
        })
        .unwrap_or(false);
    let _ = std::fs::remove_file(path);
    if !result {
        return Ok(false);
    }
    let (click_x, click_y) =
        crate::inputs::common::reconnect_button(window.x, window.y, window.width, window.height);
    warp_cursor(click_x, click_y)?;
    Ok(true)
}

pub fn run(state: &SharedState) -> Result<(), String> {
    probe()?;
    let mut keyboard = create_keyboard_device()?;
    let mut mouse = create_mouse_device()?;
    let mut hidden_ids: Vec<String> = Vec::new();

    loop {
        if !should_continue(state, InputMode::Gnome) {
            break;
        }
        let settings = { state.lock().unwrap().clone() };
        if !settings.stealth {
            for id in hidden_ids.drain(..) {
                let _ = set_minimized(&id, false);
            }
        }
        if settings.user_safe {
            match user_active() {
                Ok(true) => {
                    set_status(state, RuntimeStatus::Paused);
                    if responsive_sleep(state, InputMode::Gnome, 5) {
                        break;
                    }
                    continue;
                }
                Ok(false) => {}
                Err(error) => return Err(error),
            }
        }

        let mut targets = list_windows()?
            .into_iter()
            .filter(|window| {
                is_sober_metadata(
                    Some(&window.desktop),
                    Some(&window.class),
                    Some(&window.title),
                )
            })
            .collect::<Vec<_>>();
        if !settings.multi_instance {
            targets.truncate(1);
        }
        if targets.is_empty() {
            set_status(state, RuntimeStatus::WaitingForSober);
            if responsive_sleep(state, InputMode::Gnome, 2) {
                break;
            }
            continue;
        }

        let previous = list_windows().ok().and_then(|windows| {
            windows
                .into_iter()
                .find(|window| window.active)
                .map(|window| window.id)
        });
        let cursor = cursor_position().ok();
        set_action_active(state, true);
        set_status(state, RuntimeStatus::PerformingAction);
        thread::sleep(Duration::from_millis(500));

        let mut error = None;
        for target in targets {
            if !should_continue(state, InputMode::Gnome) {
                break;
            }
            match focus_window(&target.id) {
                Ok(true) => {}
                Ok(false) => {
                    error = Some("GNOME target disappeared before activation".to_string());
                    break;
                }
                Err(err) => {
                    error = Some(err);
                    break;
                }
            }
            thread::sleep(Duration::from_millis(150));
            if let Err(err) = warp_cursor(target.x + target.width / 2, target.y + target.height / 2)
            {
                error = Some(err);
                break;
            }
            if let Err(err) = click_mouse(&mut mouse) {
                error = Some(err);
                break;
            }
            if let Err(err) = perform_keyboard_actions(&mut keyboard, &settings) {
                error = Some(err);
                break;
            }
            if settings.auto_reconnect {
                set_status(state, RuntimeStatus::ReconnectChecking);
                match reconnect_click(&target) {
                    Ok(true) => {
                        set_status(state, RuntimeStatus::Reconnecting);
                        thread::sleep(Duration::from_millis(100));
                        for _ in 0..3 {
                            if let Err(err) = click_mouse(&mut mouse) {
                                error = Some(err);
                                break;
                            }
                            thread::sleep(Duration::from_millis(80));
                        }
                    }
                    Ok(false) => {}
                    Err(err) => {
                        error = Some(err);
                        break;
                    }
                }
            }
            if settings.stealth {
                match set_minimized(&target.id, true) {
                    Ok(true) => {
                        if !hidden_ids.contains(&target.id) {
                            hidden_ids.push(target.id.clone());
                        }
                    }
                    Ok(false) => {
                        error = Some("GNOME target disappeared while hiding".to_string());
                        break;
                    }
                    Err(err) => {
                        error = Some(err);
                        break;
                    }
                }
            }
        }

        if let Some(previous) = previous
            && !hidden_ids.contains(&previous)
        {
            let _ = focus_window(&previous);
        }
        if let Some((x, y)) = cursor {
            let _ = warp_cursor(x, y);
        }
        set_action_active(state, false);

        if let Some(error) = error {
            for id in hidden_ids.drain(..) {
                let _ = set_minimized(&id, false);
            }
            return Err(error);
        }
        set_status(state, RuntimeStatus::Ready);
        if responsive_sleep(state, InputMode::Gnome, settings.interval_seq) {
            break;
        }
    }

    for id in hidden_ids.drain(..) {
        let _ = set_minimized(&id, false);
    }
    set_action_active(state, false);
    set_status(state, RuntimeStatus::Stopped);
    Ok(())
}

fn has_gnome_shell() -> bool {
    forced_desktop() == Some(InputMode::Gnome)
        || std::env::var_os("GNOME_DESKTOP_SESSION_ID").is_some()
        || std::env::var("XDG_CURRENT_DESKTOP")
            .map(|value| {
                value
                    .split(':')
                    .any(|token| token.eq_ignore_ascii_case("gnome"))
            })
            .unwrap_or(false)
}

pub fn probe() -> Result<(), String> {
    match call_helper("Probe", &[]) {
        Ok(value) if value == "true" => Ok(()),
        Ok(_) => Err(format!(
            "the GNOME helper of {APP_TITLE} is disabled or incompatible. Enable it with gnome-extensions enable {GNOME_EXTENSION_UUID}"
        )),
        Err(error) => {
            if !has_gnome_shell() {
                Err(format!("GNOME Shell was not detected ({error})"))
            } else if error.contains("UnknownObject") || error.contains("UnknownMethod") {
                Err(format!(
                    "the GNOME helper is disabled. Enable it with: gnome-extensions enable {GNOME_EXTENSION_UUID}"
                ))
            } else {
                Err(format!(
                    "the GNOME helper of {APP_TITLE} is not available: {error}"
                ))
            }
        }
    }
}

pub fn show_sober() -> Result<(), String> {
    let target = list_windows()?
        .into_iter()
        .find(|window| {
            is_sober_metadata(
                Some(&window.desktop),
                Some(&window.class),
                Some(&window.title),
            )
        })
        .ok_or_else(|| "Sober window was not found.".to_string())?;
    if focus_window(&target.id)? {
        Ok(())
    } else {
        Err("GNOME could not activate Sober.".to_string())
    }
}

pub fn hide_sober() -> Result<(), String> {
    let target = list_windows()?
        .into_iter()
        .find(|window| {
            is_sober_metadata(
                Some(&window.desktop),
                Some(&window.class),
                Some(&window.title),
            )
        })
        .ok_or_else(|| "Sober window was not found.".to_string())?;
    if set_minimized(&target.id, true)? {
        Ok(())
    } else {
        Err("GNOME could not hide Sober.".to_string())
    }
}

pub fn focused_is_sober() -> bool {
    list_windows()
        .map(|windows| {
            windows.into_iter().any(|window| {
                window.active
                    && is_sober_metadata(
                        Some(&window.desktop),
                        Some(&window.class),
                        Some(&window.title),
                    )
            })
        })
        .unwrap_or(false)
}

pub fn supports_capture() -> bool {
    probe().is_ok() || crate::inputs::common::wayland_capture_available()
}

pub fn supports_hide() -> bool {
    probe().is_ok()
}

#[cfg(test)]
mod tests {
    use super::unquote_gvariant_string;

    #[test]
    fn unquotes_gnome_helper_payloads() {
        assert_eq!(
            unquote_gvariant_string("('[{\"id\":\"42\"}]',)").unwrap(),
            "[{\"id\":\"42\"}]"
        );
        assert_eq!(unquote_gvariant_string("('a\\'b')").unwrap(), "a'b");
        assert_eq!(
            unquote_gvariant_string("('line\\nbreak')").unwrap(),
            "line\nbreak"
        );
    }
}
