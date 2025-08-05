use crate::backend::utils::assets::AssetLoader;
use dioxus::prelude::*;

#[component]
pub fn StandaloneLogo(animations_played: bool) -> Element {
    let logo = AssetLoader::get_logo();

    rsx! {
        div { class: if !animations_played { "standalone-logo-wrapper logo-animate" } else { "standalone-logo-wrapper" },
            div { class: "standalone-logo",
                img { src: "{logo}", alt: "Logo", class: "standalone-logo-img" }
            }
            h1 { class: "standalone-app-name", "Dream Launcher" }
        }
    }
}
