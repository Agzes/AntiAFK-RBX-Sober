use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub const APP_ID: &str = "dev.agzes.antiafk-rbx-sober";

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AppState {
    pub running: bool,
    pub jump: bool,
    pub walk: bool,
    pub spin_jiggle: bool,
    pub interval_seq: u64,
    pub mode: usize,

    pub auto_start: bool,
    pub multi_instance: bool,
    pub hides_game: bool,
    pub user_safe: bool,
    pub auto_reconnect: bool,
    pub fps_capper: bool,
    pub fps_limit: u32,
    pub stop_limit_on_focus: bool,
    pub last_run_version: Option<f32>,
    pub shown_warning: bool,
    #[serde(skip)]
    pub manually_stopped: bool,
    #[serde(skip)]
    pub action_active: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            running: false,
            jump: true,
            walk: false,
            spin_jiggle: false,
            interval_seq: 60,
            mode: 0,
            auto_start: false,
            multi_instance: false,
            hides_game: false,
            user_safe: false,
            auto_reconnect: false,
            fps_capper: false,
            fps_limit: 30,
            stop_limit_on_focus: false,
            last_run_version: None,
            shown_warning: false,
            manually_stopped: false,
            action_active: false,
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;

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

    pub fn load() -> Self {
        if let Some(path) = Self::get_config_path()
            && let Ok(data) = fs::read_to_string(path)
            && let Ok(mut state) = serde_json::from_str::<AppState>(&data)
        {
            state.running = false;
            return state;
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::get_config_path() {
            let mut state_to_save = *self;
            state_to_save.running = false;
            if let Ok(data) = serde_json::to_string_pretty(&state_to_save) {
                let _ = fs::write(path, data);
            }
        }
    }
}
