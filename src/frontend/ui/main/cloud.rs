use dioxus::prelude::*;

#[component]
pub fn Cloud() -> Element {
    rsx! {
        div { class: "cloud-content",
            h2 { "Cloud" }
            p { "Lorem ipsum dolor sit amet" }
        }
    }
}
