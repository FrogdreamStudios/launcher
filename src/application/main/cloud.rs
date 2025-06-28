use dioxus::prelude::*;

#[component]
pub fn Cloud() -> Element {
    rsx! {
        div { class: "cloud-content",
            h2 { "Cloud" }
        }
    }
}