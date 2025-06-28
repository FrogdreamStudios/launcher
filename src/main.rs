mod creeper;
mod application;
mod play_together;
mod chats;
mod route;
use crate::route::Route;

use dioxus::prelude::*;
use dioxus_desktop::{use_window, Config, LogicalSize, WindowBuilder};
use dioxus::LaunchBuilder;
use dioxus_router::prelude::*;

fn main() {
    let size = LogicalSize::new(1280.0, 832.0);

    let config = Config::default().with_window(
        WindowBuilder::new()
            .with_title("Dream Launcher")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_resizable(false),
    );

    LaunchBuilder::new()
        .with_cfg(config)
        .launch(ModeSelector);
}

#[component]
fn ModeSelector() -> Element {
    let mut mode = use_signal(|| None::<bool>);
    let window = use_window();

    match *mode.read() {
        None => rsx! {
            div {
                style: "display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100vh;",
                h2 { "What do you want to launch?" }
                div {
                    style: "display: flex; gap: 24px; margin-top: 24px;",
                    button {
                        style: "padding: 12px 32px; font-size: 1.1rem;",
                        onclick: move |_| mode.set(Some(true)),
                        "UI in dev"
                    }
                    button {
                        style: "padding: 12px 32px; font-size: 1.1rem;",
                        onclick: move |_| mode.set(Some(false)),
                        "CLI in dev"
                    }
                }
            }
        },
        Some(true) => rsx! { AppRoot {} },
        Some(false) => {
            use_effect({
                let _window = window.clone();
                move || {
                    std::thread::spawn(|| {
                        let _ = creeper::creeper::main();
                    });
                }
            });
            rsx!({})
        }
    }
}

#[component]
fn AppRoot() -> Element {
    let is_authenticated = use_signal(|| false);
    provide_context(application::auth::auth_context::AuthState { is_authenticated: is_authenticated.clone() });
    rsx! { Router::<Route> {} }
}