//! Authentication context and state management.

use crate::frontend::services::user::UserConfig;
use dioxus::prelude::*;

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
        if !UserConfig::is_valid_username(&username) {
            return Err("Username must be 3-16 characters long and can only contain letters, numbers, and underscores".to_string());
        }

        let user_config = UserConfig::new(username);

        if let Err(e) = user_config.save().await {
            return Err(format!("Failed to save user config: {e}"));
        }

        self.current_user.set(Some(user_config));
        self.is_authenticated.set(true);

        Ok(())
    }

    /// Logs out the current user.
    pub async fn logout(&mut self) {
        self.current_user.set(None);
        self.is_authenticated.set(false);
        let _ = UserConfig::delete().await;
    }

    /// Gets the current username or returns "Player" as default.
    pub fn get_username(&self) -> String {
        self.current_user
            .read()
            .as_ref()
            .map_or_else(|| "Player".to_string(), |user| user.username.clone())
    }
}
