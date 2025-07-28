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

/*pub struct ConfigManager {
    config_path: PathBuf,
    config: AppConfig,
}

impl ConfigManager {
    pub fn new() -> Result<Self, ConfigError> {
        let config_dir = FileManager::get_app_data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("DreamLauncher");

        let config_path = config_dir.join("config.json");

        FileManager::ensure_dir_exists(&config_dir).map_err(ConfigError::IoError)?;

        let config = if FileManager::file_exists(&config_path) {
            Self::load_config(&config_path)?
        } else {
            let default_config = AppConfig::default();
            Self::save_config(&config_path, &default_config)?;
            default_config
        };

        Ok(Self {
            config_path,
            config,
        })
    }

    pub fn get_config(&self) -> &AppConfig {
        &self.config
    }

    pub fn get_config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        Self::save_config(&self.config_path, &self.config)
    }

    pub fn reload(&mut self) -> Result<(), ConfigError> {
        self.config = Self::load_config(&self.config_path)?;
        Ok(())
    }

    pub fn reset_to_default(&mut self) -> Result<(), ConfigError> {
        self.config = AppConfig::default();
        self.save()
    }

    fn load_config(path: &Path) -> Result<AppConfig, ConfigError> {
        let content = FileManager::read_file_to_string(path).map_err(ConfigError::IoError)?;

        serde_json::from_str(&content).map_err(ConfigError::SerdeError)
    }

    fn save_config(path: &Path, config: &AppConfig) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(config).map_err(ConfigError::SerdeError)?;

        FileManager::write_string_to_file(path, &content).map_err(ConfigError::IoError)
    }

    pub fn get_minecraft_dir(&self) -> &Path {
        &self.config.minecraft.game_directory
    }

    pub fn get_java_path(&self) -> Option<&Path> {
        self.config.minecraft.java_path.as_deref()
    }

    pub fn set_minecraft_dir(&mut self, path: PathBuf) {
        self.config.minecraft.game_directory = path;
    }

    pub fn set_java_path(&mut self, path: Option<PathBuf>) {
        self.config.minecraft.java_path = path;
    }

    pub fn get_memory_allocation(&self) -> (u32, u32) {
        (
            self.config.minecraft.memory_allocation.min_memory,
            self.config.minecraft.memory_allocation.max_memory,
        )
    }

    pub fn set_memory_allocation(&mut self, min: u32, max: u32) {
        self.config.minecraft.memory_allocation.min_memory = min;
        self.config.minecraft.memory_allocation.max_memory = max;
    }
}*/

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    SerdeError(serde_json::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(err) => write!(f, "IO error: {}", err),
            ConfigError::SerdeError(err) => write!(f, "Serialization error: {}", err),
        }
    }
}

impl std::error::Error for ConfigError {}
