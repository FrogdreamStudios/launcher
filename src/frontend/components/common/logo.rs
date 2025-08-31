//! Logo component.

use crate::backend::utils::css::ResourceLoader;
use dioxus::prelude::*;

#[component]
pub fn Logo(animations_played: bool) -> Element {
    let logo = ResourceLoader::get_asset("logo");

    let handle_click = move |_| {
        if let Err(e) = webbrowser::open("https://github.com/FrogdreamStudios/launcher") {
            log::error!("Failed to open GitHub link: {e}");
        }
    };

    rsx! {
        div {
            class: if !animations_played { "standalone-logo-wrapper logo-animate" } else { "standalone-logo-wrapper" },
            style: "cursor: pointer;",
            onclick: handle_click,
            div { class: "standalone-logo",
                img { src: "{logo}", alt: "Logo", class: "standalone-logo-img" }
            }
            h1 { class: "standalone-app-name", "Dream Launcher" }
        }
    }
}
