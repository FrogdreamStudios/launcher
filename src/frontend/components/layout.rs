use crate::backend::utils::assets::AssetLoader;
use crate::backend::utils::css_loader::CssLoader;
use crate::backend::utils::route::Route;
use crate::frontend::components::{
    chat_sidebar::ChatSidebar,
    context_menu::ContextMenu,
    debug_window::{DebugWindow, use_version_selection},
    minecraft_launcher::launch_minecraft,
    navigation::Navigation,
    news::News,
    standalone_logo::StandaloneLogo,
};
use crate::frontend::game_state::use_game_state;
use dioxus::prelude::Key;
use dioxus::prelude::*;
use dioxus_router::{components::Outlet, use_route};

#[component]
pub fn Layout() -> Element {
    let mut show_ui = use_signal(|| false);
    let mut initial_load = use_signal(|| true);
    let mut animations_played = use_signal(|| false);
    let route = use_route::<Route>();
    let mut last_active_page = use_signal(|| "Home");

    // Context menu state
    let mut show_context_menu = use_signal(|| false);
    let mut context_menu_x = use_signal(|| 0.0);
    let mut context_menu_y = use_signal(|| 0.0);

    // Game state
    let game_status = use_game_state();

    // Debug window and version selection state
    let mut show_debug_window = use_signal(|| false);
    let version_selection = use_version_selection();

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
            tabindex: "0",
            onkeydown: move |e| {
                if e.key() == Key::F12 {
                    e.prevent_default();
                    show_debug_window.set(!show_debug_window());
                }
            },

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

                        // Server icons
                        div { class: "server-icon server-icon-1" }
                        div { class: "server-icon server-icon-2" }
                        div { class: "server-icon server-icon-3" }

                        // Server names
                        div { class: "server-name server-name-1", "Server 1" }
                        div { class: "server-name server-name-2", "Server 2" }
                        div { class: "server-name server-name-3", "Server 3" }

                        // Server last played
                        div { class: "server-last-played server-last-played-1", "Last played: 15m ago" }
                        div { class: "server-last-played server-last-played-2", "Last played: 15m ago" }
                        div { class: "server-last-played server-last-played-3", "Last played: 15m ago" }

                        // Last connection play buttons
                        div { class: "last-connection-play last-connection-play-1",
                            img { src: AssetLoader::get_play(), class: "play-icon" }
                            div { class: "play-text", "Play" }
                        }
                        div { class: "last-connection-play last-connection-play-2",
                            img { src: AssetLoader::get_play(), class: "play-icon" }
                            div { class: "play-text", "Play" }
                        }
                        div { class: "last-connection-play last-connection-play-3",
                            img { src: AssetLoader::get_play(), class: "play-icon" }
                            div { class: "play-text", "Play" }
                        }

                        // Additional buttons
                        img { src: AssetLoader::get_additional(), class: "additional-button additional-button-1" }
                        img { src: AssetLoader::get_additional(), class: "additional-button additional-button-2" }
                        img { src: AssetLoader::get_additional(), class: "additional-button additional-button-3" }

                        div { class: "instances-title", "Instances" }
                        div { class: "instances-divider" }

                        // Instance card
                        div {
                            class: if game_status().is_active() {
                                "instance-card instance-card-pulsing"
                            } else {
                                "instance-card"
                            },
                            onclick: {
                                let selected_version = version_selection().selected_version.read().clone();
                                move |_| {
                                    launch_minecraft(game_status, &selected_version);
                                }
                            },
                            oncontextmenu: move |e| {
                                e.prevent_default();
                                let client_x = e.client_coordinates().x;
                                let client_y = e.client_coordinates().y;
                                context_menu_x.set(client_x);
                                context_menu_y.set(client_y);
                                show_context_menu.set(true);
                            },

                            div { class: "instance-level-text", "28" }

                            // Version indicator
                            div {
                                class: "instance-version-indicator",
                                "{version_selection().selected_version.read()}"
                            }

                            // Debug button overlay
                            div {
                                class: "instance-debug-button",
                                onclick: move |e| {
                                    e.stop_propagation();
                                    show_debug_window.set(true);
                                },
                                "DEBUG"
                            }
                        }
                    }
                }

                div {
                    class: if !animations_played() { "play-together play-animate" } else { "play-together" }
                }

                News { animations_played: animations_played() }
            }

            // Context menu
            ContextMenu {
                show: show_context_menu,
                x: context_menu_x,
                y: context_menu_y,
                game_status: game_status
            }

            // Debug window
            DebugWindow {
                show: show_debug_window,
                version_selection: version_selection,
                game_status: game_status
            }
        }
    }
}
