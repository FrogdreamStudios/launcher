use crate::backend::utils::route::Route;
use dioxus::prelude::*;
use dioxus_router::{navigator, use_route};

#[component]
pub fn Navigation() -> Element {
    let nav = navigator();
    let route = use_route::<Route>();

    let active_tab = match route {
        Route::Auth {} => "Auth",
        Route::Home { .. } => "Main",
        Route::Packs { .. } => "Packs",
        Route::Settings { .. } => "Settings",
        Route::Cloud { .. } => "Cloud",
        Route::New { .. } => "New",
        Route::Chat { .. } => "Chat",
    };

    const LOGO: Asset = asset!("/public/assets/images/other/logo.png");
    const HOME: Asset = asset!("/public/assets/images/buttons/home.png");
    const PACKS: Asset = asset!("/public/assets/images/buttons/packs.png");
    const SETTINGS: Asset = asset!("/public/assets/images/buttons/settings.png");
    const CLOUD: Asset = asset!("/public/assets/images/buttons/cloud.png");
    const PLUS: Asset = asset!("/public/assets/images/buttons/plus.png");

    rsx! {
        nav { class: "navigation nav-animate",
            div { class: "logo-wrapper logo-animate",
                div { class: "logo",
                    img { src: "{LOGO}", alt: "Logo", class: "logo-img" }
                }
                h1 { class: "app-name", "Dream Launcher" }
            }

            ul { class: "nav-items nav-items-animate",
                li {
                    class: if active_tab == "Main" { "nav-item active nav-item-1" } else { "nav-item nav-item-1" },
                    onclick: move |_| { nav.push("/"); },
                    img { class: "nav-icon", src: "{HOME}", alt: "Home" }
                    span { class: "nav-text", "Home" }
                }
                li {
                    class: if active_tab == "Packs" { "nav-item active nav-item-2" } else { "nav-item nav-item-2" },
                    onclick: move |_| { nav.push("/packs"); },
                    img { class: "nav-icon", src: "{PACKS}", alt: "Mods & Packs" }
                    span { class: "nav-text", "Mods & Packs" }
                }
                li {
                    class: if active_tab == "Settings" { "nav-item active nav-item-3" } else { "nav-item nav-item-3" },
                    onclick: move |_| { nav.push("/settings"); },
                    img { class: "nav-icon", src: "{SETTINGS}", alt: "Settings" }
                    span { class: "nav-text", "Settings" }
                }
                li {
                    class: if active_tab == "Cloud" { "nav-item active nav-item-4" } else { "nav-item nav-item-4" },
                    onclick: move |_| { nav.push("/cloud"); },
                    img { class: "nav-icon", src: "{CLOUD}", alt: "Cloud" }
                    span { class: "nav-text", "Cloud" }
                }
                li {
                    class: if active_tab == "New" { "nav-item active nav-item-5" } else { "nav-item nav-item-5" },
                    onclick: move |_| { nav.push("/new"); },
                    img { class: "nav-icon", src: "{PLUS}", alt: "New tab" }
                }
            }
        }
    }
}
