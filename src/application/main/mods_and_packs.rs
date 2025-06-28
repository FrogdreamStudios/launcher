use dioxus::prelude::*;

#[component]
pub fn ModsAndPacks() -> Element {
    rsx! {
        div { class: "mods-and-packs-content",
            h2 { "Mods and Packs" }
        }
    }
}