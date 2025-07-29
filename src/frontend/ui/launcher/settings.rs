use dioxus::prelude::*;

#[component]
pub fn Settings() -> Element {
    rsx! {
        div { class: "settings-content",
            h2 { "Settings" }
            p { "Lorem ipsum dolor sit amet" }
        }
    }
}
