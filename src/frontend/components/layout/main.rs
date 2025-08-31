//! Main layout component.

use crate::backend::services::VisitTracker;
use crate::backend::utils::application::Route;
use crate::backend::utils::css::ResourceLoader;
use crate::frontend::components::common::titlebar::TitleBar;
use crate::frontend::components::common::{ContextMenu, DebugWindow, RenameDialog};
use crate::frontend::services::context::AuthState;
use crate::frontend::services::launcher::get_python_bridge;
use crate::frontend::{
    components::{
        common::{GameProgress, Logo, News, Selector, UpdateProgress},
        layout::Navigation,
    },
    services::instances::InstanceManager,
    services::states::{
        GameStatus, ProgressStatus, set_game_progress_state, set_game_progress_state_simple,
        set_instance_running, use_game_progress_state, use_game_state, use_update_state,
    },
};
use dioxus::prelude::{Key, *};
use dioxus_router::{components::Outlet, navigator, use_route};
use webbrowser;

#[component]
pub fn Layout() -> Element {
    let mut show_ui = use_signal(|| false);
    let mut initial_load = use_signal(|| true);
    let mut animations_played = use_signal(|| false);
    let route = use_route::<Route>();
    let mut last_active_page = use_signal(|| "Home");
    let nav = navigator();
    let auth = use_context::<AuthState>();

    // Update progress state
    let (show_update, progress, status) = use_update_state();

    // Game progress state
    let (show_game_progress, game_progress, game_status_text, game_status_type, _game_instance_id) =
        use_game_progress_state();

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

    // Watch for instance deletions and clear context menu if needed
    use_effect(move || {
        use crate::frontend::services::instances::INSTANCES;
        let instances = INSTANCES.read();
        if let Some(selected_id) = context_menu_instance_id()
            && !instances.contains_key(&selected_id)
        {
            context_menu_instance_id.set(None);
            show_context_menu.set(false);
        }
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

        UpdateProgress {
            show: show_update(),
            progress: progress(),
            status: status()
        }

        GameProgress {
            show: show_game_progress(),
            progress: game_progress(),
            status: game_status_text(),
            status_type: game_status_type()
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
                        // Temporarily hidden last connections section
                        /*
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
                        */

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
                                        move |_| {
                                            // Immediately mark as running to prevent race conditions
                                            set_instance_running(instance_id, true);

                                            active_instance_id.set(Some(instance_id));
                                            let username = auth.get_username();

                                            // Start installation and launch process
                                            spawn(install_and_launch_instance(
                                                instance_version.clone(),
                                                username,
                                                instance_id,
                                                active_instance_id
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
                                                0..=3 => "34px",
                                                4 => "28px",
                                                5 => "24px",
                                                6 => "20px",
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
                    {
                        let username = auth.get_username();
                        let display_name = if username.is_empty() { "cubelius" } else { &username };
                        rsx! {
                            img {
                                src: format!("https://minotar.net/helm/{}/49.png", display_name),
                                style: "width: 49px; height: 49px; border-radius: 8px; object-fit: cover;"
                            }
                        }
                    }
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
                        let mut auth = auth;
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
                rename_current_name: rename_current_name,
                active_instance_id: active_instance_id
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

            // Debug window
            DebugWindow {
                show: show_debug_window,
                instance_id: active_instance_id
            }
        }
    }
}

pub async fn install_and_launch_instance(
    version: String,
    username: String,
    instance_id: u32,
    mut active_instance_id: Signal<Option<u32>>,
) {
    set_game_progress_state_simple(
        true,
        10.0,
        format!("Preparing {version}"),
        Some(instance_id),
    );

    // Get Python bridge
    let bridge = match get_python_bridge() {
        Ok(bridge) => bridge,
        Err(e) => {
            log::error!("Failed to get Python bridge: {e}");
            set_game_progress_state(
                true,
                0.0,
                "Failed to initialize launcher".to_string(),
                ProgressStatus::Failed,
                Some(instance_id),
            );
            set_instance_running(instance_id, false);
            // Hide failed status after 5 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            set_game_progress_state_simple(false, 0.0, String::new(), None);
            return;
        }
    };

    // Install version (will skip if already installed)
    set_game_progress_state_simple(
        true,
        30.0,
        format!("Preparing {version}"),
        Some(instance_id),
    );

    match bridge.install_version(&version).await {
        Ok(_) => {
            set_game_progress_state_simple(
                true,
                70.0,
                format!("Minecraft is starting..."),
                Some(instance_id),
            );

            // Launch Minecraft with log streaming
            let config = crate::backend::launcher::bridge::LaunchConfig {
                username,
                version: version.clone(),
            };

            // Create a channel for log messages
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let version_clone = version.clone();

            // Handle log messages
            spawn(async move {
                let mut game_started = false;

                while let Some(log_message) = rx.recv().await {
                    use crate::backend::launcher::bridge::MinecraftLogMessage;

                    match log_message {
                        MinecraftLogMessage::LaunchResult {
                            success,
                            pid,
                            message,
                        } => {
                            if success {
                                log::info!("Minecraft launched with PID: {pid}");
                                crate::frontend::services::states::add_debug_log(
                                    "INFO".to_string(),
                                    format!("Minecraft launched with PID: {pid}"),
                                    Some(instance_id),
                                );
                                set_game_progress_state(
                                    true,
                                    90.0,
                                    "Minecraft is starting...".to_string(),
                                    ProgressStatus::InProgress,
                                    Some(instance_id),
                                );
                            } else {
                                log::error!("Failed to launch Minecraft: {message}");
                                crate::frontend::services::states::add_debug_log(
                                    "ERROR".to_string(),
                                    format!("Failed to launch Minecraft: {message}"),
                                    Some(instance_id),
                                );
                                set_game_progress_state(
                                    true,
                                    100.0,
                                    format!("Failed to start {version_clone}"),
                                    ProgressStatus::Failed,
                                    Some(instance_id),
                                );
                            }
                        }
                        MinecraftLogMessage::Log { line, pid: _ } => {
                            log::info!("Minecraft log: {line}");

                            // Add to debug console
                            crate::frontend::services::states::add_debug_log(
                                "INFO".to_string(),
                                line.clone(),
                                Some(instance_id),
                            );

                            // Check for successful game start indicators
                            if !game_started
                                && (line.contains("[main/INFO]: Setting user:")
                                    || line.contains("[main/INFO]: Environment:")
                                    || line.contains("[Render thread/INFO]: OpenGL")
                                    || line.contains("[Render thread/INFO]: Created:")
                                    || line.contains("[main/INFO]: Loaded")
                                        && line.contains("recipes")
                                    || line
                                        .contains("[Render thread/INFO]: Stopping worker threads"))
                            {
                                game_started = true;
                                set_game_progress_state(
                                    true,
                                    100.0,
                                    "Minecraft has started successfully".to_string(),
                                    ProgressStatus::Success,
                                    Some(instance_id),
                                );

                                // Hide success status after 3 seconds
                                spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                                    set_game_progress_state_simple(false, 0.0, String::new(), None);
                                });
                            }
                        }
                        MinecraftLogMessage::Exit {
                            pid,
                            exit_code,
                            message,
                        } => {
                            log::info!(
                                "Minecraft process {pid} exited with code {exit_code}: {message}"
                            );
                            crate::frontend::services::states::add_debug_log(
                                "INFO".to_string(),
                                format!(
                                    "Minecraft process {pid} exited with code {exit_code}: {message}"
                                ),
                                Some(instance_id),
                            );

                            if !game_started {
                                // Game failed to start properly
                                set_game_progress_state(
                                    true,
                                    100.0,
                                    "Minecraft failed to start".to_string(),
                                    ProgressStatus::Failed,
                                    Some(instance_id),
                                );

                                // Hide failed status after 5 seconds
                                spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                    set_game_progress_state_simple(false, 0.0, String::new(), None);
                                });
                            }

                            // Mark instance as not running
                            set_instance_running(instance_id, false);
                            active_instance_id.set(None);
                            break; // Exit the loop when process exits
                        }
                        MinecraftLogMessage::Error {
                            success: _,
                            message,
                        } => {
                            log::error!("Minecraft error: {message}");
                            crate::frontend::services::states::add_debug_log(
                                "ERROR".to_string(),
                                format!("Minecraft error: {message}"),
                                Some(instance_id),
                            );
                            set_game_progress_state(
                                true,
                                100.0,
                                format!("Failed to start {version_clone}"),
                                ProgressStatus::Failed,
                                Some(instance_id),
                            );

                            // Hide failed status after 5 seconds
                            spawn(async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                set_game_progress_state_simple(false, 0.0, String::new(), None);
                            });

                            set_instance_running(instance_id, false);
                            active_instance_id.set(None);
                            break; // Exit the loop on error
                        }
                    }
                }
            });

            match bridge
                .launch_minecraft(config, instance_id, move |log_message| {
                    let _ = tx.send(log_message);
                })
                .await
            {
                Ok(exit_code) => {
                    log::info!("Minecraft process completed with exit code: {exit_code}");
                }
                Err(e) => {
                    log::error!("Error launching Minecraft with logs: {e}");
                    set_game_progress_state(
                        true,
                        100.0,
                        format!("Failed to start {version}"),
                        ProgressStatus::Failed,
                        Some(instance_id),
                    );
                    // Hide the failed status after 5 seconds
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    set_game_progress_state_simple(false, 0.0, String::new(), None);
                    set_instance_running(instance_id, false);
                    active_instance_id.set(None);
                }
            }

            // Log handler task will be cleaned up automatically

            return; // Don't mark as not running immediately
        }
        Err(e) => {
            log::error!("Failed to verify/install version: {e}");
            set_game_progress_state(
                true,
                0.0,
                format!("Failed to install {version}"),
                ProgressStatus::Failed,
                Some(instance_id),
            );
            set_instance_running(instance_id, false);
            // Hide failed status after 5 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            set_game_progress_state_simple(false, 0.0, String::new(), None);
        }
    }

    // Mark the instance as no longer running
    set_instance_running(instance_id, false);
}
