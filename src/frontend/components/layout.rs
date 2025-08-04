use crate::backend::utils::css_loader::CssLoader;
use crate::backend::utils::route::Route;
use crate::frontend::components::{chat_sidebar::ChatSidebar, navigation::Navigation, news::News};
use dioxus::prelude::*;
use dioxus_router::components::Outlet;

#[component]
pub fn Layout() -> Element {
    let mut show_ui = use_signal(|| false);
    let mut show_play_together = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            show_ui.set(true);
            show_play_together.set(true);
        });
    });

    rsx! {
        style {
            dangerous_inner_html: CssLoader::get_combined_main()
        }

        div {
            class: if show_ui() { "desktop fade-in" } else { "desktop fade-out" },

            Navigation {}

            div { class: if show_ui() { "main-layout main-layout-animate" } else { "main-layout" },
                ChatSidebar {}

                main { class: "content",
                    Outlet::<Route> {}
                }

                div { class: "center-block" }

                div {
                    class: if show_play_together() { "play-together play-animate" } else { "play-together" }
                }

                News {}
            }
        }
    }
}
