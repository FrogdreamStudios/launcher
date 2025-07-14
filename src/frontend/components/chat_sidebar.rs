use crate::backend::utils::route::Route;
use crate::frontend::chats::manager::provide_chat_manager;
use dioxus::prelude::*;
use dioxus_router::prelude::*;

#[component]
pub fn ChatSidebar() -> Element {
    let nav = navigator();
    let route = use_route::<Route>();
    let chat_manager = provide_chat_manager();

    let active_tab = match route {
        Route::Chat { .. } => "Chat",
        _ => "Main",
    };

    rsx! {
        aside { class: "chat-sidebar chat-animate",
            if active_tab == "Chat" {
                // Back button when in chat
                div {
                    class: "chat-item",
                    onclick: move |_| { nav.go_back(); },
                    div { class: "chat-avatar",
                        div { class: "chat-icon", "←" }
                    }
                    div { class: "chat-info",
                        div { class: "username", "Back" }
                        div { class: "account-type", "To main menu" }
                    }
                }
                div { class: "chat-separator" }
            } else {
                // User account
                div {
                    class: "chat-item",
                    onclick: move |_| {
                        // TODO: Account functionality
                    },
                    div { class: "chat-avatar",
                        img { class: "avatar-img", src: "https://minotar.net/avatar/cubelius/33.png", alt: "cubelius" }
                        div { class: "status-indicator online" }
                    }
                    div { class: "chat-info",
                        div { class: "username", "cubelius" }
                        div { class: "account-type", "Microsoft account" }
                    }
                }
                div { class: "chat-separator" }

                // Chat list
                for user in chat_manager.users.read().iter() {
                    div {
                        key: "{user.username}",
                        class: "chat-item",
                        onclick: {
                            let username = user.username.clone();
                            move |_| {
                                nav.push(format!("/chat/{}", username));
                            }
                        },
                        div { class: "chat-avatar",
                            img {
                                class: "avatar-img",
                                src: "{user.avatar_url}",
                                alt: "{user.username}"
                            }
                            div {
                                class: "status-indicator {user.status.css_class()}"
                            }
                        }
                        div { class: "chat-info",
                            div { class: "username", "{user.display_name}" }
                            div { class: "status-message", "{user.last_message}" }
                            if user.unread_count > 0 {
                                div {
                                    class: "unread-badge",
                                    "{user.unread_count}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
