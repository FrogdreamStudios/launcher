use dioxus::prelude::*;

#[component]
pub fn New() -> Element {
    rsx! {
        div { class: "new-content",
            h2 { "New Tab" }
        }
    }
}