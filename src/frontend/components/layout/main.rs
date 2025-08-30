use crate::backend::services::VisitTracker;
use crate::backend::utils::application::Route;
use crate::backend::utils::css::ResourceLoader;
use crate::frontend::components::common::titlebar::TitleBar;
use crate::frontend::services::launcher;
use crate::frontend::{
    components::{
        common::{News, Logo, Selector},
        launcher::{ContextMenu, RenameDialog},
        layout::Navigation,
    },
    services::instances::InstanceManager,
    services::states::{GameStatus, use_game_state},
};
use dioxus::prelude::{Key, *};
use dioxus_router::{components::Outlet, navigator, use_route};
use webbrowser;
use crate::frontend::services::context::AuthState;

#[component]
pub fn Layout() -> Element {
    let mut show_ui = use_signal(|| false);
    let mut initial_load = use_signal(|| true);
    let mut animations_played = use_signal(|| false);
    let route = use_route::<Route>();
    let mut last_active_page = use_signal(|| "Home");
    let nav = navigator();
    let auth = use_context::<AuthState>();

    // Visit the tracker with reactive signals
    let visit_tracker = use_signal(VisitTracker::new);
    let mut sites = use_signal(Vec::new);
    let refresh_trigger = use_signal(|| 0);

    // Initialize sites on the first render
    use_effect(move || {
        let initial_sites = visit_tracker.with(|tracker| tracker.get_sorted_sites());
        sites.set(initial_sites);
    });

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

    // Version selector state
    let mut show_version_selector = use_signal(|| false);

    // Rename dialog state
    let show_rename_dialog = use_signal(|| false);
    let rename_instance_id = use_signal(|| None::<u32>);
    let rename_current_name = use_signal(String::new);

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
    let is_settings = current_page == "Settings";
    let is_new = current_page == "New";

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
            dangerous_inner_html: ResourceLoader::get_embedded_css_with_fonts()
        }

        TitleBar {}

        div {
            class: if show_ui() { "desktop fade-in" } else { "desktop fade-out" },
            tabindex: "0",
            onkeydown: move |e| {
                if e.key() == Key::F12 {
                    e.prevent_default();
                    InstanceManager::toggle_debug_mode();
                }
            },

            Logo { animations_played: animations_played() }

            Navigation { animations_played: animations_played() }

            div { class: if show_ui() && !animations_played() { "main-layout main-layout-animate" } else { "main-layout" },

                // Temporarily hidden
                // ChatSidebar { animations_played: animations_played() }

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
                            img { src: ResourceLoader::get_asset("play"), class: "play-icon" }
                            div { class: "play-text", "Play" }
                        }
                        div { class: "last-connection-play last-connection-play-2",
                            img { src: ResourceLoader::get_asset("play"), class: "play-icon" }
                            div { class: "play-text", "Play" }
                        }
                        div { class: "last-connection-play last-connection-play-3",
                            img { src: ResourceLoader::get_asset("play"), class: "play-icon" }
                            div { class: "play-text", "Play" }
                        }

                        // Additional buttons
                        img { src: ResourceLoader::get_asset("additional"), class: "additional-button additional-button-1" }
                        img { src: ResourceLoader::get_asset("additional"), class: "additional-button additional-button-2" }
                        img { src: ResourceLoader::get_asset("additional"), class: "additional-button additional-button-3" }

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
                                        let mut classes = vec!["instance-card", "instance-card-dynamic"];
                                        if active_instance_id() == Some(instance.id) {
                                            classes.push("instance-card-pulsing");
                                        }
                                        classes.join(" ")
                                    },
                                    style: format!("--instance-color: #{}", instance.color),
                                    onclick: {
                                        let instance_version = instance.version.clone();
                                        let instance_id = instance.id;
                                        let auth = auth.clone();
                                        move |_| {
                                                active_instance_id.set(Some(instance_id));
                                                let username = auth.get_username();

                                                // Start installation and launch process
                                                spawn(install_and_launch_instance(
                                                    instance_version.clone(),
                                                    username
                                                ));
                                        }
                                    },
                                    oncontextmenu: {
                                        let instance_id = instance.id;
                                        move |e| {
                                            e.prevent_default();
                                            let client_x = e.client_coordinates().x;
                                            let client_y = e.client_coordinates().y;
                                            context_menu_x.set(client_x);
                                            context_menu_y.set(client_y);
                                            context_menu_instance_id.set(Some(instance_id));
                                            show_context_menu.set(true);
                                        }
                                    },

                                    div {
                                        class: "instance-card-title",
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
                                        "{instance.name}"
                                    }
                                }
                            }

                            // Add a new instance card (+ button)
                            if InstanceManager::can_create_instance() {
                                div {
                                    class: "instance-card instance-card-add",
                                    onclick: move |_| {
                                        show_version_selector.set(true);
                                    },

                                    div {
                                        class: "instance-add-icon",
                                        img { src: ResourceLoader::get_asset("plus") }
                                    }
                                }
                            }
                        }
                    }
                }

                // Temporarily hidden
                /*
                div {
                    class: if !animations_played() { "play-together play-animate" } else { "play-together" }
                }
                */

                // Temporarily hidden
                /*
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
                */

                News { animations_played: animations_played() }
            }

            if is_settings {
                div {
                    class: "settings-title",
                    "Settings"
                }

                div {
                    class: "settings-divider",
                }

                div {
                    class: "settings-panel",
                }

                div {
                    class: "settings-server-icon",
                }

                div {
                    class: "settings-server-name",
                    "Account"
                }

                div {
                    class: "settings-server-last-played",
                    "Change your account"
                }

                div {
                    class: "settings-change-button",
                    onclick: {
                        let mut auth = auth.clone();
                        let nav = nav.clone();
                        move |_| {
                            spawn(async move {
                                auth.logout().await;
                                nav.push("/auth");
                            });
                        }
                    },
                    img { src: ResourceLoader::get_asset("change"), class: "change-icon" }
                    div { class: "change-text", "Change" }
                }
            }

            if is_new {
                div {
                    class: "new-title new-title-fixed",
                    "What will you jump into?"
                }

                div {
                    class: "new-divider new-divider-fixed",
                }

                {
                    // Trigger refresh if needed
                    let _ = refresh_trigger();

                    sites().into_iter().take(5).enumerate().map(|(i, site)| {
                        let site_key = match site.url.as_str() {
                            "https://www.minecraft.net" => "minecraft",
                            "https://minecraft.wiki/" => "minecraft_wiki",
                            "https://www.planetminecraft.com" => "planet_minecraft",
                            "https://www.curseforge.com/minecraft" => "curseforge",
                            "https://namemc.com" => "namemc",
                            _ => "unknown",
                        };

                        rsx! {
                            div {
                                key: "{site.url}",
                                class: "new-panel new-panel-dynamic",
                                style: format!("--panel-top: {}px;", 137 + (i * 81)),
                            }

                            div {
                                class: "new-server-icon new-server-icon-dynamic",
                                style: format!("--icon-top: {}px;", 145 + (i * 81)),
                                img {
                                    src: ResourceLoader::get_asset(&site.icon_key),
                                    class: "server-icon-img server-icon-img-fixed"
                                }
                            }

                            div {
                                class: "new-server-name new-server-name-dynamic",
                                style: format!("--name-top: {}px;", 149 + (i * 81)),
                                "{site.name}"
                            }

                            div {
                                class: "new-server-last-played",
                                style: format!("top: {}px;", 168 + (i * 81)),
                                {
                                    if site.visit_count == 0 {
                                        "You haven't visited this website yet".to_string()
                                    } else {
                                        VisitTracker::format_time_ago(site.last_visited)
                                    }
                                }
                            }

                            div {
                                class: "new-open-button",
                                style: format!("top: {}px;", 153 + (i * 81)),
                                onclick: {
                                    let url = site.url.clone();
                                    let site_key = site_key.to_string();
                                    let mut tracker = visit_tracker;
                                    let mut sites_signal = sites;
                                    let mut refresh = refresh_trigger;
                                    move |_| {
                                        // Record visit
                                        tracker.with_mut(|t| t.record_visit(&site_key));

                                        // Update sites list
                                        let updated_sites = tracker.with(|t| t.get_sorted_sites());
                                        sites_signal.set(updated_sites);

                                        // Trigger refresh
                                        refresh.set(refresh() + 1);

                                        let url_clone = url.clone();
                                        spawn(async move {
                                            if let Err(e) = webbrowser::open(&url_clone) {
                                                log::error!("Failed to open browser: {e}");
                                            }
                                        });
                                    }
                                },
                                img { src: ResourceLoader::get_asset("open"), class: "open-icon" }
                                div { class: "open-text", "Open" }
                            }

                            img {
                                src: ResourceLoader::get_asset("additional"),
                                class: "new-additional-button",
                                style: format!("top: {}px;", 153 + (i * 81)),
                            }
                        }
                    })
                }
            }

            // Context menu
            ContextMenu {
                show: show_context_menu,
                x: context_menu_x,
                y: context_menu_y,
                game_status: game_status,
                instance_id: context_menu_instance_id,
                show_debug_window: show_debug_window,
                show_rename_dialog: show_rename_dialog,
                rename_instance_id: rename_instance_id,
                rename_current_name: rename_current_name
            }

            RenameDialog {
                show: show_rename_dialog,
                instance_id: rename_instance_id,
                current_name: rename_current_name
            }

            // Version selector
            Selector {
                show: show_version_selector
            }
        }
    }
}

async fn install_and_launch_instance(version: String, username: String) {
    // Get Python bridge
    let bridge = match launcher::get_python_bridge() {
        Ok(bridge) => bridge,
        Err(e) => {
            log::error!("Failed to get Python bridge: {e}");
            return;
        }
    };

    match bridge.install_version(&version).await {
        Ok(_) => {
            let config = crate::backend::bridge::LaunchConfig {
                username,
                version: version.clone(),
            };

            match bridge.launch_minecraft(config).await {
                Ok(result) => {
                    if result.success {
                    } else {
                        log::error!("Failed to launch Minecraft: {}", result.message);
                    }
                }
                Err(e) => {
                    log::error!("Error launching Minecraft: {e}");
                }
            }
        }
        Err(e) => {
            log::error!("Failed to install version: {e}");
        }
    }
}
