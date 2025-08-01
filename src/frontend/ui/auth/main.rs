use crate::frontend::components::auth_layout::AuthLayout;
use crate::frontend::ui::auth::auth_context::AuthState;
use dioxus::events::KeyboardEvent;
use dioxus::hooks::use_signal;
use dioxus::prelude::*;
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

    // Validation function for the username
    let is_valid = move || {
        let name = username.read();
        (3..=16).contains(&name.len())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };

    const LOGO: Asset = asset!("/public/assets/images/other/logo.png");
    const MICROSOFT: Asset = asset!("/public/assets/images/other/microsoft.png");

    // Function to handle keypress events
    let on_keypress = move |e: KeyboardEvent| {
        if e.key() == "Enter".parse().unwrap() && is_valid() {
            hide_ui.set(true);
            spawn(async move {
                sleep(Duration::from_millis(700)).await;
                auth.is_authenticated.set(true);
                nav.push("/home");
            });
        }
    };

    rsx! {
        AuthLayout {
            main {
                class: if hide_ui() { "container fade-out" } else { "desktop" },
                div {
                    class: "content",
                    img {
                        class: "logo logo-animate",
                        src: "{LOGO}",
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
                                class: "microsoft-icon",
                                src: "{MICROSOFT}",
                                alt: "Microsoft Logo"
                            }
                            span {
                                class: "microsoft-login-text",
                                "Login with Microsoft"
                            }
                        }
                        button {
                            class: "login-button offline-login",
                            onclick: move |_| input_visible.set(true),
                            div {
                                class: "offline-content",
                                if input_visible() {
                                    input {
                                        class: "inline-input",
                                        r#type: "text",
                                        value: "{username()}",
                                        maxlength: "16",
                                        oninput: move |e| username.set(e.value().clone()),
                                        onkeypress: on_keypress,
                                        placeholder: "Offline account",
                                        autofocus: true
                                    }
                                } else {
                                    span { "Offline account" }
                                }
                            }
                        }
                        // Error message for invalid username
                        if input_visible() && !is_valid() && username().len() >= 3 {
                        // TODO: implement error message for invalid username
                        } else {
                            div {
                                class: "error-message-placeholder",
                                style: "height: 1.5em;"
                            }
                        }
                    }
                }
            }
        }
    }
}
