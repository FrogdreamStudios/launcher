use crate::backend::utils::file_manager::FileManager;
use serde::{Deserialize, Serialize};

use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub launcher: LauncherConfig,
    pub minecraft: MinecraftConfig,
    pub chat: ChatConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub auto_update: bool,
    pub check_updates_on_startup: bool,
    pub close_launcher_on_game_start: bool,
    pub keep_launcher_open: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftConfig {
    pub game_directory: PathBuf,
    pub java_path: Option<PathBuf>,
    pub java_args: Vec<String>,
    pub memory_allocation: MemoryConfig,
    pub window_settings: WindowSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub min_memory: u32, // MB
    pub max_memory: u32, // MB
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub enabled: bool,
    pub auto_connect: bool,
    pub show_notifications: bool,
    pub sound_notifications: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub language: String,
    pub animations_enabled: bool,
    pub sidebar_collapsed: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            launcher: LauncherConfig::default(),
            minecraft: MinecraftConfig::default(),
            chat: ChatConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            auto_update: true,
            check_updates_on_startup: true,
            close_launcher_on_game_start: false,
            keep_launcher_open: true,
        }
    }
}

impl Default for MinecraftConfig {
    fn default() -> Self {
        let game_dir = FileManager::get_app_data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("DreamLauncher")
            .join("minecraft");

        Self {
            game_directory: game_dir,
            java_path: None,
            java_args: vec![
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+UseG1GC".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:MaxGCPauseMillis=50".to_string(),
                "-XX:G1HeapRegionSize=32M".to_string(),
            ],
            memory_allocation: MemoryConfig::default(),
            window_settings: WindowSettings::default(),
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            min_memory: 256,
            max_memory: 4096,
        }
    }
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            width: 854,
            height: 480,
            fullscreen: false,
        }
    }
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_connect: false,
            show_notifications: true,
            sound_notifications: false,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            language: "en".to_string(),
            animations_enabled: true,
            sidebar_collapsed: false,
        }
    }
}
