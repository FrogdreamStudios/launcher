use crate::backend::utils::css::main::ResourceLoader;
use crate::frontend::{components::layout::AuthLayout, pages::auth::AuthState};
use dioxus::{events::KeyboardEvent, prelude::*};
use dioxus_router::use_navigator;
use std::time::Duration;
use tokio::time::sleep;

#[component]
pub fn Auth() -> Element {
    let nav = use_navigator();
    let mut auth = use_context::<AuthState>();
    let mut input_visible = use_signal(|| false);
    let mut username = use_signal(String::new);
    let mut hide_ui = use_signal(|| false);
    let mut input_ref = use_signal(|| None as Option<std::rc::Rc<MountedData>>);
    let mut show_error = use_signal(|| false);

    // Validation function for the username
    let is_valid = move || {
        let name = username.read();
        (3..=16).contains(&name.len())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };

    let logo = ResourceLoader::get_asset("logo");
    let microsoft = ResourceLoader::get_asset("microsoft");

    // Function to handle keypress events
    let on_keypress = move |e: KeyboardEvent| {
        if e.key() == Key::Enter {
            if is_valid() {
                show_error.set(false);
                hide_ui.set(true);
                spawn(async move {
                    sleep(Duration::from_millis(700)).await;
                    auth.is_authenticated.set(true);
                    nav.push("/home");
                });
            } else {
                show_error.set(true);
            }
        }
    };

    // Auto-focus input when component mounts or when input becomes visible
    use_effect(move || {
        if input_visible() {
            spawn(async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                if let Some(element) = input_ref.read().as_ref() {
                    std::mem::drop(element.set_focus(true));
                }
            });
        }
    });

    // Reset state and show input when component mounts (for Change button navigation)
    use_effect(move || {
        spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            input_visible.set(true);
        });
    });

    rsx! {
        AuthLayout {
            style { dangerous_inner_html: ResourceLoader::get_css("error_message") }
            main {
                class: if hide_ui() { "container fade-out" } else { "desktop" },
                div {
                    class: "content",
                    img {
                        class: "logo logo-animate",
                        src: "{logo}",
                        alt: "Dream Launcher Logo"
                    }
                    h1 {
                        class: "welcome-text",
                        "Welcome to Dream Launcher!"
                    }
                    div {
                        class: "login-options",
                        button {
                            class: "login-button microsoft-login",
                            img {
                                src: "{microsoft}",
                                alt: "Microsoft Logo",
                                class: "microsoft-icon"
                            }
                            span {
                                class: "microsoft-login-text",
                                "Login with Microsoft"
                            }
                        }
                        button {
                            class: "login-button offline-login",
                            onclick: move |_| {
                                input_visible.set(true);
                                show_error.set(false);
                                // Focus the input after it becomes visible
                                spawn(async move {
                                    tokio::time::sleep(Duration::from_millis(10)).await;
                                    if let Some(element) = input_ref.read().as_ref() {
                                        let _ = element.set_focus(true);
                                    }
                                });
                            },
                            div {
                                class: "offline-content",
                                if input_visible() {
                                    input {
                                        class: "inline-input",
                                        r#type: "text",
                                        value: "{username()}",
                                        maxlength: "16",
                                        oninput: move |e| {
                                            username.set(e.value());
                                            show_error.set(false);
                                        },
                                        onkeypress: on_keypress,
                                        placeholder: "Offline account",
                                        autofocus: true,
                                        onmounted: move |element| {
                                            input_ref.set(Some(element.data()));
                                            // Ensure focus when mounted
                                            spawn(async move {
                                                tokio::time::sleep(Duration::from_millis(50)).await;
                                                let _ = element.set_focus(true);
                                            });
                                        }
                                    }
                                } else {
                                    span { "Offline account" }
                                }
                            }
                        }
                        div {
                            class: if show_error() { "error-message error-visible" } else { "error-message error-hidden" },
                            "Username must be 3-16 characters long and can only contain letters, numbers, and underscores"
                        }
                    }
                }
            }
        }
    }
}
