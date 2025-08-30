//! User configuration.

use crate::backend::utils::paths::get_launcher_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub username: String,
}

impl UserConfig {
    /// Creates a new user config with the given username.
    pub fn new(username: String) -> Self {
        Self { username }
    }

    /// Validates if a username meets the requirements.
    pub fn is_valid_username(username: &str) -> bool {
        (3..=16).contains(&username.len())
            && username
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    /// Gets the path to the user config file.
    pub fn get_config_path() -> PathBuf {
        get_launcher_dir()
            .unwrap_or_else(|_| PathBuf::from("DreamLauncher"))
            .join("user_config.json")
    }

    /// Saves the user config to disk.
    pub async fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path();

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(config_path, json).await?;

        Ok(())
    }

    /// Loads the user config from disk.
    pub async fn load() -> Option<Self> {
        let config_path = Self::get_config_path();

        if !config_path.exists() {
            return None;
        }

        match fs::read_to_string(config_path).await {
            Ok(json) => serde_json::from_str(&json).ok(),
            Err(_) => None,
        }
    }

    /// Deletes the user config file.
    pub async fn delete() -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path();
        if config_path.exists() {
            fs::remove_file(config_path).await?;
        }
        Ok(())
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            username: "Player".to_string(),
        }
    }
}
