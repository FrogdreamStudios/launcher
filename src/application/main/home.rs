use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    rsx! {
        div { class: "home-content",
            h2 { "Home" }
        }
    }
}