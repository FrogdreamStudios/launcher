use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct ChatUser {
    pub username: String,
    pub display_name: String,
    pub avatar_url: String,
    pub status: UserStatus,
    pub last_message: String,
    pub last_message_time: String,
    pub unread_count: u32,
}

#[derive(Clone, PartialEq)]
pub enum UserStatus {
    Online,
    Offline,
}

impl UserStatus {
    pub fn css_class(&self) -> &'static str {
        match self {
            UserStatus::Online => "online",
            UserStatus::Offline => "offline",
        }
    }
}

#[derive(Clone)]
pub struct Manager {
    pub users: Signal<Vec<ChatUser>>,
}

impl Manager {
    pub fn new() -> Self {
        let users = Signal::new(get_initial_chat_users());
        Self { users }
    }
}

fn get_initial_chat_users() -> Vec<ChatUser> {
    // TODO: use mock data
    vec![
        ChatUser {
            username: "Kolyakot33".to_string(),
            display_name: "Kolyakot33".to_string(),
            avatar_url: "https://minotar.net/avatar/Kolyakot33/33.png".to_string(),
            status: UserStatus::Online,
            last_message: truncate_message("Okay, thanks", 16),
            last_message_time: "15:20".to_string(),
            unread_count: 1,
        },
        ChatUser {
            username: "nerulex".to_string(),
            display_name: "nerulex".to_string(),
            avatar_url: "https://minotar.net/avatar/nerulex/33.png".to_string(),
            status: UserStatus::Online,
            last_message: truncate_message("Rollback it", 16),
            last_message_time: "16:45".to_string(),
            unread_count: 0,
        },
        ChatUser {
            username: "aor1keno".to_string(),
            display_name: "aor1keno".to_string(),
            avatar_url: "https://minotar.net/avatar/aor1keno/33.png".to_string(),
            status: UserStatus::Online,
            last_message: truncate_message("HELLLOOOOOOOOOOOOOOOOOOO", 16),
            last_message_time: "12:00".to_string(),
            unread_count: 2,
        },
        ChatUser {
            username: "Varfolomey".to_string(),
            display_name: "Varfolomey".to_string(),
            avatar_url: "https://minotar.net/avatar/Varfolomey/33.png".to_string(),
            status: UserStatus::Offline,
            last_message: truncate_message("Fired", 16),
            last_message_time: "18:00".to_string(),
            unread_count: 0,
        },
    ]
}

fn truncate_message(message: &str, max_length: usize) -> String {
    if message.chars().count() <= max_length {
        message.to_string()
    } else {
        let truncated: String = message.chars().take(max_length.saturating_sub(3)).collect();
        format!("{truncated}...")
    }
}

// Context for global chat state
pub fn provide_chat_manager() -> Manager {
    let manager = Manager::new();
    provide_context(manager.clone());
    manager
}
