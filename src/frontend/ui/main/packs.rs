use dioxus::prelude::*;

#[component]
pub fn Packs() -> Element {
    rsx! {
        div { class: "mods-and-packs-content",
            h2 { "Mods & Resource Packs" }
            p { "Lorem ipsum dolor sit amet" }
        }
    }
}
