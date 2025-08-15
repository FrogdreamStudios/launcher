use crate::backend::utils::app::main::Route;
use crate::backend::utils::css::main::ResourceLoader;
use crate::frontend::{
    components::{
        chat_sidebar::ChatSidebar,
        context_menu::ContextMenu,
        debug_window::{DebugWindow, use_version_selection},
        minecraft_launcher::launch_minecraft,
        navigation::Navigation,
        news::News,
        standalone_logo::StandaloneLogo,
    },
    game_state::{GameStatus, use_game_state},
    instances::main::InstanceManager,
};
use dioxus::prelude::{Key, *};
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
    let mut context_menu_instance_id = use_signal(|| None::<u32>);

    // Initialize instance manager
    use_effect(move || {
        InstanceManager::initialize();
    });

    // Game state
    let game_status = use_game_state();
    let mut active_instance_id = use_signal(|| None::<u32>);

    // Debug window and version selection state
    let show_debug_window = use_signal(|| false);
    let version_selection = use_version_selection();

    // Inline editing state
    let mut editing_instance_id = use_signal(|| None::<u32>);
    let mut editing_text = use_signal(String::new);

    // Determine current page and update last active if not in chat
    let current_page = match route {
        Route::Home { .. } | Route::Auth { .. } => "Home",
        Route::Packs { .. } => "Packs",
        Route::Settings { .. } => "Settings",
        Route::Cloud { .. } => "Cloud",
        Route::New { .. } => "New",
        Route::Chat { .. } => last_active_page(), // Keep last active when in chat
    };

    // Update the last active page only for non-chat routes
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

    // Reset active instance when game stops
    use_effect(move || {
        if game_status() == GameStatus::Idle {
            active_instance_id.set(None);
        }
    });

    rsx! {
        style {
            dangerous_inner_html: ResourceLoader::get_combined_main_css()
        }

        div {
            class: if show_ui() { "desktop fade-in" } else { "desktop fade-out" },
            tabindex: "0",
            onkeydown: move |e| {
                if e.key() == Key::F12 {
                    e.prevent_default();
                    InstanceManager::toggle_debug_mode();
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
                            img { src: ResourceLoader::get_play(), class: "play-icon" }
                            div { class: "play-text", "Play" }
                        }
                        div { class: "last-connection-play last-connection-play-2",
                            img { src: ResourceLoader::get_play(), class: "play-icon" }
                            div { class: "play-text", "Play" }
                        }
                        div { class: "last-connection-play last-connection-play-3",
                            img { src: ResourceLoader::get_play(), class: "play-icon" }
                            div { class: "play-text", "Play" }
                        }

                        // Additional buttons
                        img { src: ResourceLoader::get_additional(), class: "additional-button additional-button-1" }
                        img { src: ResourceLoader::get_additional(), class: "additional-button additional-button-2" }
                        img { src: ResourceLoader::get_additional(), class: "additional-button additional-button-3" }

                        div { class: "instances-title", "Instances" }
                        div { class: "instances-divider" }

                        // Instance cards container
                        div {
                            class: "instances-container",

                            // Render existing instances
                            for (_index, instance) in InstanceManager::get_instances_sorted().iter().enumerate() {
                                div {
                                    key: "{instance.id}",
                                    class: {
                                        let mut classes = vec!["instance-card"];
                                        if editing_instance_id() == Some(instance.id) {
                                            classes.push("editing");
                                        } else if game_status().is_active() && active_instance_id() == Some(instance.id) {
                                            classes.push("instance-card-pulsing");
                                        }
                                        classes.join(" ")
                                    },
                                    style: "background-color: #{instance.color};",
                                    onclick: {
                                        let selected_version = version_selection().selected_version.read().clone();
                                        let instance_id = instance.id;
                                        move |_| {
                                            // Don't launch if this instance is being edited or the game is running
                                            if !game_status().is_active() && editing_instance_id() != Some(instance_id) {
                                                active_instance_id.set(Some(instance_id));
                                                launch_minecraft(game_status, &selected_version, instance_id);
                                            }
                                        }
                                    },
                                    oncontextmenu: {
                                        let instance_id = instance.id;
                                        move |e| {
                                            e.prevent_default();

                                            // Don't show the context menu if this instance is being edited
                                            if editing_instance_id() != Some(instance_id) {
                                                let client_x = e.client_coordinates().x;
                                                let client_y = e.client_coordinates().y;
                                                context_menu_x.set(client_x);
                                                context_menu_y.set(client_y);
                                                context_menu_instance_id.set(Some(instance_id));
                                                show_context_menu.set(true);
                                            }
                                        }
                                    },

                                    if editing_instance_id() == Some(instance.id) {
                                        input {
                                            r#type: "text",
                                            class: "instance-name-input",
                                            value: "{editing_text()}",
                                            maxlength: "7",
                                            autofocus: true,
                                            style: {
                                                let text_len = editing_text().len();
                                                let font_size = match text_len {
                                                    0..=3 => "36px",
                                                    4 => "30px",
                                                    5 => "26px",
                                                    6 => "22px",
                                                    _ => "18px",
                                                };
                                                format!("background: transparent; border: none; color: #ffffff !important; text-align: center; font-size: {font_size}; font-weight: 700; font-family: 'Gilroy-Bold', Helvetica, Arial, sans-serif; width: 100%; outline: none; z-index: 1000; padding: 0 8px; margin: 0; box-sizing: border-box; -webkit-text-fill-color: #ffffff !important; position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%);")
                                            },
                                            oninput: move |e| {
                                                editing_text.set(e.value().chars().take(7).collect());
                                            },
                                            onkeydown: {
                                                let instance_id = instance.id;
                                                move |e| {
                                                    match e.key() {
                                                        Key::Enter => {
                                                            InstanceManager::rename_instance(instance_id, &editing_text());
                                                            editing_instance_id.set(None);
                                                            editing_text.set(String::new());
                                                        },
                                                        Key::Escape => {
                                                            editing_instance_id.set(None);
                                                            editing_text.set(String::new());
                                                        },
                                                        _ => {}
                                                    }
                                                }
                                            },
                                            onblur: {
                                                let instance_id = instance.id;
                                                move |_| {
                                                    if !editing_text().is_empty() {
                                                        InstanceManager::rename_instance(instance_id, &editing_text());
                                                    }
                                                    editing_instance_id.set(None);
                                                    editing_text.set(String::new());
                                                }
                                            }
                                        }
                                    } else {
                                        div {
                                            class: "instance-level-text",
                                            style: {
                                                let text_len = instance.name.len();
                                                let font_size = match text_len {
                                                    0..=3 => "36px",
                                                    4 => "30px",
                                                    5 => "26px",
                                                    6 => "22px",
                                                    _ => "18px",
                                                };
                                                format!("font-size: {font_size}; padding: 0 16px;")
                                            },
                                            ondoubleclick: {
                                                let instance_id = instance.id;
                                                let instance_name = instance.name.clone();
                                                move |_| {
                                                    editing_instance_id.set(Some(instance_id));
                                                    editing_text.set(instance_name.clone());
                                                }
                                            },
                                            "{instance.name}"
                                        }
                                    }
                                }
                            }

                            // Add a new instance card (+ button)
                            if InstanceManager::can_create_instance() {
                                div {
                                    class: "instance-card instance-card-add",
                                    onclick: move |_| {
                                        InstanceManager::create_instance();
                                    },

                                    div {
                                        class: "instance-add-icon",
                                        img { src: ResourceLoader::get_plus() }
                                    }
                                }
                            }
                        }
                    }
                }

                // Temporary
                /*
                div {
                    class: if !animations_played() { "play-together play-animate" } else { "play-together" }
                }
                */

                // Temporary
                div {
                    style: "
                        position: absolute;
                        width: 192px;
                        height: 339px;
                        left: 32px;
                        top: 413px;
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        font-family: 'Gilroy-Medium', Helvetica, Arial, sans-serif;
                        font-size: 14px;
                        color: #6f6f6f;
                        user-select: none;
                    ",
                    "No shared connections."
                }

                News { animations_played: animations_played() }
            }

            // Context menu
            ContextMenu {
                show: show_context_menu,
                x: context_menu_x,
                y: context_menu_y,
                game_status: game_status,
                instance_id: context_menu_instance_id,
                show_debug_window: show_debug_window,
                editing_instance_id: editing_instance_id,
                editing_text: editing_text
            }

            // Debug window
            DebugWindow {
                show: show_debug_window,
                version_selection: version_selection,
                game_status: game_status,
                instance_id: context_menu_instance_id
            }
        }

    }
}
