use crate::{
    frontend::assets::main::ResourceLoader,
    frontend::{
        components::launcher::minecraft_launcher::launch_minecraft,
        pages::auth::AuthState,
        services::instances::main::{INSTANCES, InstanceManager, open_instance_folder},
        states::GameStatus,
    },
};
use dioxus::prelude::*;

// Define async logic outside the component (because of closure issues)
async fn spawn_launch_minecraft(
    game_status: Signal<GameStatus>,
    version: String,
    id: u32,
    username: String,
) {
    tokio::time::sleep(std::time::Duration::from_millis(160)).await;
    println!("About to call launch_minecraft with version: {version}");
    launch_minecraft(game_status, &version, id, &username);
}

#[derive(Props, Clone, PartialEq, Eq)]
pub struct ContextMenuProps {
    pub show: Signal<bool>,
    pub x: Signal<f64>,
    pub y: Signal<f64>,
    pub game_status: Signal<GameStatus>,
    pub instance_id: Signal<Option<u32>>,
    pub show_debug_window: Signal<bool>,
    pub show_rename_dialog: Signal<bool>,
    pub rename_instance_id: Signal<Option<u32>>,
    pub rename_current_name: Signal<String>,
}

#[component]
pub fn ContextMenu(props: ContextMenuProps) -> Element {
    let mut show = props.show;
    let x = props.x;
    let y = props.y;
    let game_status = props.game_status;
    let instance_id = props.instance_id;
    let mut show_debug_window = props.show_debug_window;
    let mut show_rename_dialog = props.show_rename_dialog;
    let mut rename_instance_id = props.rename_instance_id;
    let auth = use_context::<AuthState>();
    let mut rename_current_name = props.rename_current_name;
    let mut is_hiding = use_signal(|| false);
    let mut should_render = use_signal(|| false);

    // Watch for show changes and handle animation
    use_effect(move || {
        if show() {
            should_render.set(true);
            is_hiding.set(false);
        } else if should_render() {
            // Small delay before starting hide animation
            spawn(async move {
                is_hiding.set(true);
                // Hide after animation completes (150 ms)
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                should_render.set(false);
                is_hiding.set(false);
            });
        }
    });

    // Handle clicks outside the menu
    let handle_backdrop_click = move |_| {
        show.set(false);
    };

    let handle_run_click = move |e: Event<MouseData>| {
        e.stop_propagation();
        if let Some(id) = instance_id() {
            println!("Instance ID: {}", id);

            // Get the instance version
            let version = {
                let instances = INSTANCES.read();
                println!("Total instances loaded: {}", instances.len());

                // Debug
                for (inst_id, inst) in instances.iter() {
                    println!(
                        "Instance {}: name='{}', version='{}'",
                        inst_id, inst.name, inst.version
                    );
                }

                if let Some(instance) = instances.get(&id) {
                    println!(
                        "Found target instance {}: name='{}', version='{}'",
                        id, instance.name, instance.version
                    );
                    instance.version.clone()
                } else {
                    println!("ERROR: Instance {} not found in loaded instances!", id);
                    println!(
                        "Available instance IDs: {:?}",
                        instances.keys().collect::<Vec<_>>()
                    );
                    "1.21.8".to_string() // Fallback
                }
            };

            let username = auth.get_username();
            show.set(false);
            // Start Minecraft launch after menu closes
            spawn(spawn_launch_minecraft(game_status, version, id, username));
        }
    };

    let handle_folder_click = move |e: Event<MouseData>| {
        e.stop_propagation();
        if let Some(id) = instance_id() {
            println!("Folder clicked - opening instance {id} folder");
            show.set(false);
            // Open instance folder after menu closes
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(160)).await;
                if let Err(e) = open_instance_folder(id).await {
                    println!("Failed to open instance {id} folder: {e}");
                }
            });
        }
    };

    let handle_change_click = move |e: Event<MouseData>| {
        e.stop_propagation();
        if let Some(id) = instance_id() {
            println!("Change clicked for instance {id}");

            // Get the current instance name and set up the rename dialog
            let instances = INSTANCES.read();
            if let Some(instance) = instances.get(&id) {
                rename_current_name.set(instance.name.clone());
                rename_instance_id.set(Some(id));
                show_rename_dialog.set(true);
            }
        }
        show.set(false);
    };

    let handle_delete_click = move |e: Event<MouseData>| {
        e.stop_propagation();
        if let Some(id) = instance_id() {
            println!("Delete clicked for instance {id}");
            InstanceManager::delete_instance(id);
        }
        show.set(false);
    };

    let handle_debug_click = move |e: Event<MouseData>| {
        e.stop_propagation();
        if let Some(id) = instance_id() {
            println!("Debug clicked for instance {id}");
            show.set(false);
            show_debug_window.set(true);
        }
    };

    if !should_render() {
        return rsx! {};
    }

    // Check if we have an instance selected and if debug mode is enabled
    let has_instance = instance_id().is_some();

    rsx! {
        div {
            class: "context-menu-backdrop",
            onclick: handle_backdrop_click,

            div {
                class: if is_hiding() { "context-menu context-menu-hide" } else { "context-menu context-menu-show" },
                style: "left: {x()}px; top: {y()}px;",
                onclick: |e| e.stop_propagation(),

                button {
                    class: "context-menu-button",
                    onclick: handle_run_click,
                    div { class: "context-menu-icon",
                        img { src: ResourceLoader::get_asset("play") }
                    }
                    div { class: "context-menu-text", "Run" }
                }

                button {
                    class: "context-menu-button",
                    onclick: handle_folder_click,
                    div { class: "context-menu-icon",
                        img { src: ResourceLoader::get_asset("folder") }
                    }
                    div { class: "context-menu-text", "Folder" }
                }

                if has_instance {
                    button {
                        class: "context-menu-button",
                        onclick: handle_change_click,
                        div { class: "context-menu-icon",
                            img { src: ResourceLoader::get_asset("change") }
                        }
                        div { class: "context-menu-text", "Change" }
                    }

                    button {
                        class: "context-menu-button",
                        onclick: handle_delete_click,
                        div { class: "context-menu-icon",
                            img { src: ResourceLoader::get_asset("delete") }
                        }
                        div { class: "context-menu-text", "Delete" }
                    }
                }

                if InstanceManager::is_debug_mode() {
                    button {
                        class: "context-menu-button",
                        onclick: handle_debug_click,
                        div { class: "context-menu-icon",
                            img { src: ResourceLoader::get_asset("debug") }
                        }
                        div { class: "context-menu-text", "Debug" }
                    }
                }
            }
        }
    }
}
