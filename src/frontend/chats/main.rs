use crate::backend::utils::css_loader::CssLoader;
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct Message {
    pub id: u32,
    pub sender: String,
    pub content: String,
    pub timestamp: String,
    pub is_own: bool,
}

#[derive(Clone, PartialEq)]
pub struct ChatData {
    pub username: String,
    pub avatar_url: String,
    pub status: String,
    pub messages: Vec<Message>,
    pub last_message: String,
}

#[component]
pub fn Chat(username: String) -> Element {
    // TODO: use mock data
    let username_clone = username.clone();
    let chat_data = use_memo(move || get_chat_data(&username_clone));

    rsx! {
        style {
            dangerous_inner_html: CssLoader::get_chat()
        }

        div { class: "chat-container",
            div { class: "chat-header",
                div { class: "chat-user-info",
                    img {
                        class: "header-avatar",
                        src: "{chat_data().avatar_url}",
                        alt: "{chat_data().username}"
                    }
                    div { class: "header-user-details",
                        div { class: "header-username", "{chat_data().username}" }
                    }
                }
            }
        }
    }
}

// TODO: get chat data
fn get_chat_data(username: &str) -> ChatData {
    ChatData {
        username: username.to_string(),
        avatar_url: format!("https://example.com/avatar/{username}"),
        status: "Online".to_string(),
        messages: vec![
            Message {
                id: 1,
                sender: "Alice".to_string(),
                content: "Hello".to_string(),
                timestamp: "10:00 AM".to_string(),
                is_own: false,
            },
            Message {
                id: 2,
                sender: "Bob".to_string(),
                content: "Hi!".to_string(),
                timestamp: "10:01 AM".to_string(),
                is_own: false,
            },
        ],
        last_message: "Hi!".to_string(),
    }
}
