use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AppConfig {
    pub export_dir: Option<PathBuf>,
}


impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "musprodev", "sage") {
            proj_dirs.config_dir().join("config.json")
        } else {
            PathBuf::from("config.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, content);
        }
    }

    pub fn get_export_dir(&self) -> PathBuf {
        if let Some(dir) = &self.export_dir {
            dir.clone()
        } else if let Some(user_dirs) = directories::UserDirs::new() {
            if let Some(downloads) = user_dirs.download_dir() {
                downloads.to_path_buf()
            } else {
                PathBuf::from(".")
            }
        } else {
            PathBuf::from(".")
        }
    }
}
