use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub const APP_ID: &str = "dev.agzes.antiafk-rbx-sober";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RuntimeStatus {
    #[default]
    Stopped,
    WaitingForSober,
    Ready,
    Paused,
    PerformingAction,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppState {
    pub running: bool,
    pub jump: bool,
    pub walk: bool,
    pub spin_jiggle: bool,
    pub interval_seq: u64,
    pub mode: usize,

    pub auto_start: bool,
    pub multi_instance: bool,
    pub user_safe: bool,
    pub auto_reconnect: bool,
    pub fps_capper: bool,
    pub fps_limit: u32,
    pub stop_limit_on_focus: bool,
    #[serde(default, alias = "hides_game")]
    pub stealth: bool,
    pub last_run_version: Option<String>,
    pub shown_warning: bool,
    #[serde(skip)]
    pub manually_stopped: bool,
    #[serde(skip)]
    pub action_active: bool,
    #[serde(skip)]
    pub runtime_status: RuntimeStatus,
    #[serde(skip)]
    pub error_message: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            running: false,
            jump: true,
            walk: false,
            spin_jiggle: false,
            interval_seq: 240,
            mode: 0,
            auto_start: false,
            multi_instance: false,
            user_safe: false,
            auto_reconnect: false,
            fps_capper: false,
            fps_limit: 30,
            stop_limit_on_focus: false,
            stealth: false,
            last_run_version: None,
            shown_warning: false,
            manually_stopped: false,
            action_active: false,
            runtime_status: RuntimeStatus::Stopped,
            error_message: None,
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;

pub fn set_runtime_status(state: &SharedState, status: RuntimeStatus) {
    let mut state = state.lock().unwrap();
    state.runtime_status = status;
    if status != RuntimeStatus::Error {
        state.error_message = None;
    }
}

pub fn set_runtime_error(state: &SharedState, message: impl Into<String>) {
    let mut state = state.lock().unwrap();
    state.running = false;
    state.action_active = false;
    state.manually_stopped = true;
    state.runtime_status = RuntimeStatus::Error;
    state.error_message = Some(message.into());
}

impl AppState {
    fn get_config_path() -> Option<PathBuf> {
        let mut path = dirs::config_dir()?;
        path.push("antiafk-rbx-sober");
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        path.push("config.json");
        Some(path)
    }

    fn deserialize_config(data: &str) -> Result<Self, serde_json::Error> {
        match serde_json::from_str::<Self>(data) {
            Ok(state) => Ok(state),
            Err(original_error) => {
                let mut value = serde_json::from_str::<serde_json::Value>(data)?;
                let Some(object) = value.as_object_mut() else {
                    return Err(original_error);
                };
                if object.contains_key("stealth") && object.contains_key("hides_game") {
                    let stealth_enabled = object
                        .get("stealth")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false)
                        || object
                            .get("hides_game")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false);
                    object.remove("stealth");
                    object.insert(
                        "hides_game".to_string(),
                        serde_json::Value::Bool(stealth_enabled),
                    );
                }
                serde_json::from_value(value)
            }
        }
    }

    pub fn load() -> Self {
        let mut state = if let Some(path) = Self::get_config_path()
            && let Ok(data) = fs::read_to_string(path)
            && let Ok(mut state) = Self::deserialize_config(&data)
        {
            state.running = false;
            state
        } else {
            Self::default()
        };

        let detected = Self::detect_de_mode();
        state.mode = detected;

        state
    }

    fn detect_de_mode() -> usize {
        if Self::is_hyprland() {
            0
        } else if Self::is_kde() {
            1
        } else {
            2
        }
    }

    pub fn is_hyprland() -> bool {
        std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
    }

    pub fn is_kde() -> bool {
        std::env::var("XDG_CURRENT_DESKTOP").map_or(false, |v| {
            let v = v.to_uppercase();
            v.contains("KDE") || v.contains("PLASMA")
        }) || std::env::var("KDE_FULL_SESSION").is_ok()
    }

    pub fn save(&self) {
        if let Some(path) = Self::get_config_path() {
            let mut state_to_save = self.clone();
            state_to_save.running = false;
            if let Ok(data) = serde_json::to_string_pretty(&state_to_save) {
                let _ = fs::write(path, data);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_hides_game_maps_to_stealth() {
        let mut config = serde_json::to_value(AppState::default()).unwrap();
        let object = config.as_object_mut().unwrap();
        object.remove("stealth");
        object.insert("hides_game".to_string(), serde_json::Value::Bool(true));

        let state: AppState = serde_json::from_value(config).unwrap();
        assert!(state.stealth);
    }

    #[test]
    fn legacy_duplicate_hides_game_is_tolerated() {
        let mut config = serde_json::to_value(AppState::default()).unwrap();
        config
            .as_object_mut()
            .unwrap()
            .insert("hides_game".to_string(), serde_json::Value::Bool(true));

        let state = AppState::deserialize_config(&config.to_string()).unwrap();
        assert!(state.stealth);
    }

    #[test]
    fn non_error_status_clears_previous_error() {
        let state = SharedState::new(Mutex::new(AppState::default()));
        set_runtime_error(&state, "preflight failed");
        set_runtime_status(&state, RuntimeStatus::WaitingForSober);

        let state = state.lock().unwrap();
        assert_eq!(state.runtime_status, RuntimeStatus::WaitingForSober);
        assert!(state.error_message.is_none());
    }
}
