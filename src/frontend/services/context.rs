//! Authentication context and state management.

use crate::frontend::services::user::UserConfig;
use dioxus::prelude::*;
use log::{error, info, warn};

#[derive(Clone, Copy)]
pub struct AuthState {
    pub is_authenticated: Signal<bool>,
    pub current_user: Signal<Option<UserConfig>>,
}

impl AuthState {
    /// Loads user from config and sets the authentication state.
    pub async fn load_saved_user(&mut self) {
        if let Some(user_config) = UserConfig::load().await {
            self.current_user.set(Some(user_config));
            self.is_authenticated.set(true);
        }
    }

    /// Logs in with a username and saves to config.
    pub async fn login(&mut self, username: String) -> Result<(), String> {
        info!("Starting login process for username: '{username}'");
        let trimmed_username = username.trim().to_string();
        info!("Trimmed username: '{trimmed_username}'");

        if !UserConfig::is_valid_username(&trimmed_username) {
            warn!("Invalid username: '{trimmed_username}'");
            return Err("Username must be 3-16 characters long and can only contain letters, numbers, and underscores".to_string());
        }

        info!("Username validation passed, creating user config");
        let user_config = UserConfig::new(trimmed_username);

        info!("Attempting to save user config");
        if let Err(e) = user_config.save().await {
            error!("Failed to save user config: {e}");
            return Err(format!("Failed to save user config: {e}"));
        }
        info!("User config saved successfully");

        self.current_user.set(Some(user_config));
        self.is_authenticated.set(true);
        info!("Login completed successfully");

        Ok(())
    }

    /// Logs out the current user.
    pub async fn logout(&mut self) {
        self.current_user.set(None);
        self.is_authenticated.set(false);
        let _ = UserConfig::delete().await;
    }

    /// Gets the current username or returns "Player" as default.
    #[must_use]
    pub fn get_username(&self) -> String {
        self.current_user
            .read()
            .as_ref()
            .map_or_else(|| "Player".to_string(), |user| user.username.clone())
    }
}
