//! User configuration.

use crate::backend::communicator::communicator::Communicator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub username: String,
}

impl UserConfig {
    /// Creates a new user config with the given username.
    #[must_use]
    pub fn new(username: String) -> Self {
        Self { username }
    }

    /// Validates if a username meets the requirements.
    #[must_use]
    pub fn is_valid_username(username: &str) -> bool {
        let trimmed = username.trim();
        (3..=16).contains(&trimmed.len())
            && trimmed
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    /// Saves the user config to a file.
    pub async fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let archon = crate::get_archon().ok_or("Archon not available")?;
        let communicator = Communicator::new(archon)
            .await
            .map_err(|e| format!("Failed to initialize communicator: {e}"))?;

        let json = serde_json::to_string(self)?;
        communicator
            .save_user_config(&json)
            .await
            .map_err(|e| format!("Failed to save config: {e}"))?;

        Ok(())
    }

    /// Loads the user config from a file.
    pub async fn load() -> Option<Self> {
        let archon = crate::get_archon()?;
        let communicator = Communicator::new(archon).await.ok()?;

        match communicator.load_user_config().await {
            Ok(json) => serde_json::from_str(&json).ok(),
            Err(_) => None,
        }
    }

    /// Deletes the user config file.
    pub async fn delete() -> Result<(), Box<dyn std::error::Error>> {
        let archon = crate::get_archon().ok_or("Archon not available")?;
        let communicator = Communicator::new(archon)
            .await
            .map_err(|e| format!("Failed to initialize communicator: {e}"))?;

        communicator
            .delete_user_config()
            .await
            .map_err(|e| format!("Failed to delete config: {e}"))?;

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
