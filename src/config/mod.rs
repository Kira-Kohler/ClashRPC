pub mod setup;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub player_tag: Option<String>,
    #[serde(default)]
    pub clan_invite_link: Option<String>,
    #[serde(default)]
    pub clan_tag: Option<String>,
    #[serde(default)]
    pub clan_name: Option<String>,
    #[serde(default)]
    pub clash_royale_api_key: Option<String>,
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

pub fn config_path() -> PathBuf {
    let cwd_cfg = Path::new("config.json").to_path_buf();
    if cwd_cfg.exists() {
        return cwd_cfg;
    }

    if let Some(dir) = exe_dir() {
        let exe_cfg = dir.join("config.json");
        if exe_cfg.exists() {
            return exe_cfg;
        }
        return exe_cfg;
    }

    cwd_cfg
}

pub fn load_config() -> AppConfig {
    let path = config_path();

    match fs::read_to_string(&path) {
        Ok(s) => match serde_json::from_str::<AppConfig>(&s) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!(
                    "⚠️ El config.json existe pero no se pudo parsear ({}): {}",
                    path.display(),
                    e
                );
                AppConfig::default()
            }
        },
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(cfg: &AppConfig) {
    let path = config_path();

    if let Ok(s) = serde_json::to_string_pretty(cfg) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, s);
    }
}
