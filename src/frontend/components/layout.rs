use crate::backend::utils::css_loader::CssLoader;
use crate::backend::utils::route::Route;
use crate::frontend::components::{
    chat_sidebar::ChatSidebar, navigation::Navigation, news::News, standalone_logo::StandaloneLogo,
};
use dioxus::prelude::*;
use dioxus_router::{components::Outlet, use_route};

#[component]
pub fn Layout() -> Element {
    let mut show_ui = use_signal(|| false);
    let mut initial_load = use_signal(|| true);
    let mut animations_played = use_signal(|| false);
    let route = use_route::<Route>();
    let mut last_active_page = use_signal(|| "Home");

    // Determine current page and update last active if not in chat
    let current_page = match route {
        Route::Home { .. } => "Home",
        Route::Packs { .. } => "Packs",
        Route::Settings { .. } => "Settings",
        Route::Cloud { .. } => "Cloud",
        Route::New { .. } => "New",
        Route::Chat { .. } => last_active_page(), // Keep last active when in chat
        _ => "Home",
    };

    // Update last active page only for non-chat routes
    if !matches!(route, Route::Chat { .. }) {
        last_active_page.set(current_page);
    }

    let is_home = current_page == "Home";

    use_effect(move || {
        if initial_load() {
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                show_ui.set(true);
                tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                animations_played.set(true);
                initial_load.set(false);
            });
        }
    });

    rsx! {
        style {
            dangerous_inner_html: CssLoader::get_combined_main()
        }

        div {
            class: if show_ui() { "desktop fade-in" } else { "desktop fade-out" },

            StandaloneLogo { animations_played: animations_played() }

            Navigation { animations_played: animations_played() }

            div { class: if show_ui() && !animations_played() { "main-layout main-layout-animate" } else { "main-layout" },
                ChatSidebar { animations_played: animations_played() }

                main { class: "content",
                    Outlet::<Route> {}
                }

                div { class: if !animations_played() { "center-block center-animate" } else { "center-block" },
                    if is_home {
                        div { class: "last-connections-title", "Last connections" }
                        div { class: "last-connections-divider" }

                        // Connection cards
                        div { class: "connection-card connection-card-1" }
                        div { class: "connection-card connection-card-2" }
                        div { class: "connection-card connection-card-3" }

                        div { class: "instances-title", "Instances" }
                        div { class: "instances-divider" }

                        // Instance card
                        div { class: "instance-card" }
                    }
                }

                div {
                    class: if !animations_played() { "play-together play-animate" } else { "play-together" }
                }

                News { animations_played: animations_played() }
            }
        }
    }
}
