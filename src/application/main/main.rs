use std::time::Duration;
use dioxus::prelude::*;
use dioxus_router::components::Outlet;
use dioxus_router::prelude::{navigator, use_route};
use tokio::time::sleep;
use crate::application::auth::auth_context::AuthState;
use crate::Route;

#[component]
pub fn Main() -> Element {
    let mut show_ui = use_signal(|| false);
    let nav = navigator();
    let auth = use_context::<AuthState>();

    if !(auth.is_authenticated)() {
        nav.replace("/auth");
        return rsx! { div {} };
    }

    let route = use_route::<Route>();
    let active_tab = match route {
        Route::Auth {  } => "Auth",
        Route::Home { .. } => "Main",
        Route::ModsAndPacks { .. } => "ModsAndPacks",
        Route::Settings { .. } => "Settings",
        Route::Cloud { .. } => "Cloud",
        Route::New { .. } => "New",
    };

    use_effect(move || {
        spawn(async move {
            sleep(Duration::from_millis(100)).await;
            show_ui.set(true);
        });
    });

    const LOGO: Asset = asset!("/public/assets/images/other/logo.png");
    const HOME: Asset = asset!("/public/assets/images/buttons/home.png");
    const MODS_AND_PACKS: Asset = asset!("/public/assets/images/buttons/mods_and_packs.png");
    const SETTINGS: Asset = asset!("/public/assets/images/buttons/settings.png");
    const CLOUD: Asset = asset!("/public/assets/images/buttons/cloud.png");
    const PLUS: Asset = asset!("/public/assets/images/buttons/plus.png");

    rsx! {
        style {
            dangerous_inner_html: include_str!("/Users/cubelius/RustroverProjects/Launcher/DreamLauncher/public/assets/styles/style_main.css")
        }

        div {
            class: if show_ui() { "desktop fade-in" } else { "desktop fade-out" },
            nav { class: "navigation",
                div { class: "logo-wrapper",
                    div { class: "logo",
                        img { src: "{LOGO}", alt: "Logo", class: "logo-img" }
                    }
                    h1 { class: "app-name", "Dream Launcher" }
                }

                ul { class: "nav-items",
                    li {
                        class: if active_tab == "Main" { "nav-item active" } else { "nav-item" },
                        onclick: move |_| {
                            nav.push("/");
                        },
                        img { class: "nav-icon", src: "{HOME}", alt: "Home" }
                        span { class: "nav-text", "Home" }
                    }
                    li {
                        class: if active_tab == "ModsAndPacks" { "nav-item active" } else { "nav-item" },
                        onclick: move |_| {
                            nav.push("/mods_and_packs");
                        },
                        img { class: "nav-icon", src: "{MODS_AND_PACKS}", alt: "Mods & Packs" }
                        span { class: "nav-text", "Mods & Packs" }
                    }
                    li {
                        class: if active_tab == "Settings" { "nav-item active" } else { "nav-item" },
                        onclick: move |_| {
                            nav.push("/settings");
                        },
                        img { class: "nav-icon", src: "{SETTINGS}", alt: "Settings" }
                        span { class: "nav-text", "Settings" }
                    }
                    li {
                        class: if active_tab == "Cloud" { "nav-item active" } else { "nav-item" },
                        onclick: move |_| {
                            nav.push("/cloud");
                        },
                        img { class: "nav-icon", src: "{CLOUD}", alt: "Cloud" }
                        span { class: "nav-text", "Cloud" }
                    }
                    li {
                        class: if active_tab == "New" { "nav-item active" } else { "nav-item" },
                        onclick: move |_| {
                            nav.push("/new");
                        },
                        img { class: "nav-icon", src: "{PLUS}", alt: "New tab" }
                    }
                }
            }

            main { class: "content",
                Outlet::<Route> {}
            }
        }
    }
}