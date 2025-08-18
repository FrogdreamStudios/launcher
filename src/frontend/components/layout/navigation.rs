use crate::backend::utils::app::main::Route;
use crate::backend::utils::css::main::ResourceLoader;
use dioxus::prelude::*;
use dioxus_router::{navigator, use_route};

#[component]
pub fn Navigation(animations_played: bool) -> Element {
    let nav = navigator();
    let route = use_route::<Route>();
    let mut last_active_tab = use_signal(|| "Main");

    // Only update the active tab for non-chat routes
    let current_tab = match route {
        Route::Auth {} => "Auth",
        Route::Home { .. } => "Main",
        Route::Packs { .. } => "Packs",
        Route::Settings { .. } => "Settings",
        Route::Cloud { .. } => "Cloud",
        Route::New { .. } => "New",
        Route::Chat { .. } => last_active_tab(), // Keep the last active tab when in chat
    };

    // Update the last active tab only for non-chat routes
    if !matches!(route, Route::Chat { .. }) {
        last_active_tab.set(current_tab);
    }

    let active_tab = current_tab;

    let home = ResourceLoader::get_asset("home");
    // let packs = ResourceLoader::get_asset("packs");
    let settings = ResourceLoader::get_asset("settings");
    // let cloud = ResourceLoader::get_asset("cloud");
    let add = ResourceLoader::get_asset("add");

    rsx! {
        nav { class: if !animations_played { "navigation nav-animate" } else { "navigation" },
            ul { class: if !animations_played { "nav-items nav-items-animate" } else { "nav-items" },
                li {
                    class: if active_tab == "Main" { "nav-item active nav-item-1" } else { "nav-item nav-item-1" },
                    onclick: move |_| { nav.push("/"); },
                    img { class: "nav-icon", src: "{home}", alt: "Home" }
                    span { class: "nav-text", "Home" }
                }
                // Temporarily hidden
                /*
                li {
                    class: if active_tab == "Packs" { "nav-item active nav-item-2" } else { "nav-item nav-item-2" },
                    onclick: move |_| { nav.push("/packs"); },
                    img { class: "nav-icon", src: "{packs}", alt: "Mods & Packs" }
                    span { class: "nav-text", "Mods & Packs" }
                }
                */
                li {
                    class: if active_tab == "Settings" { "nav-item active nav-item-2" } else { "nav-item nav-item-2" },
                    onclick: move |_| { nav.push("/settings"); },
                    img { class: "nav-icon", src: "{settings}", alt: "Settings" }
                    span { class: "nav-text", "Settings" }
                }
                // Temporarily hidden
                /*
                li {
                    class: if active_tab == "Cloud" { "nav-item active nav-item-4" } else { "nav-item nav-item-4" },
                    onclick: move |_| { nav.push("/cloud"); },
                    img { class: "nav-icon", src: "{cloud}", alt: "Cloud" }
                    span { class: "nav-text", "Cloud" }
                }
                */
                li {
                    class: if active_tab == "New" { "nav-item active nav-item-3" } else { "nav-item nav-item-3" },
                    onclick: move |_| { nav.push("/new"); },
                    img { class: "nav-icon", src: "{add}", alt: "New tab" }
                }
            }
        }
    }
}
