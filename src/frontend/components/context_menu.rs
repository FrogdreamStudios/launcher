use crate::{
    backend::utils::assets::AssetLoader,
    frontend::{
        components::minecraft_launcher::launch_minecraft,
        game_state::GameStatus,
        instances::main::{INSTANCES, open_instance_folder, use_instance_manager},
    },
};
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ContextMenuProps {
    pub show: Signal<bool>,
    pub x: Signal<f64>,
    pub y: Signal<f64>,
    pub game_status: Signal<GameStatus>,
    pub instance_id: Signal<Option<u32>>,
    pub show_debug_window: Signal<bool>,
    pub editing_instance_id: Signal<Option<u32>>,
    pub editing_text: Signal<String>,
}

#[component]
pub fn ContextMenu(props: ContextMenuProps) -> Element {
    let mut show = props.show;
    let x = props.x;
    let y = props.y;
    let game_status = props.game_status;
    let instance_id = props.instance_id;
    let mut show_debug_window = props.show_debug_window;
    let mut editing_instance_id = props.editing_instance_id;
    let mut editing_text = props.editing_text;
    let mut is_hiding = use_signal(|| false);
    let mut should_render = use_signal(|| false);

    let instance_manager = use_instance_manager();

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
            println!("Run clicked, launching Minecraft for instance {id}");
            show.set(false);
            // Start Minecraft launch after menu closes
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(160)).await;
                launch_minecraft(game_status, "1.21.8", id);
            });
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

            // Get the current instance name and set up editing
            let instances = INSTANCES.read();
            if let Some(instance) = instances.get(&id) {
                editing_text.set(instance.name.clone());
                editing_instance_id.set(Some(id));
            }
        }
        show.set(false);
    };

    let handle_delete_click = move |e: Event<MouseData>| {
        e.stop_propagation();
        if let Some(id) = instance_id() {
            println!("Delete clicked for instance {id}");
            instance_manager.delete_instance(id);
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
                        img { src: AssetLoader::get_play() }
                    }
                    div { class: "context-menu-text", "Run" }
                }

                button {
                    class: "context-menu-button",
                    onclick: handle_folder_click,
                    div { class: "context-menu-icon",
                        img { src: AssetLoader::get_folder() }
                    }
                    div { class: "context-menu-text", "Folder" }
                }

                if has_instance {
                    button {
                        class: "context-menu-button",
                        onclick: handle_change_click,
                        div { class: "context-menu-icon",
                            img { src: AssetLoader::get_change() }
                        }
                        div { class: "context-menu-text", "Change" }
                    }

                    button {
                        class: "context-menu-button",
                        onclick: handle_delete_click,
                        div { class: "context-menu-icon",
                            img { src: AssetLoader::get_delete() }
                        }
                        div { class: "context-menu-text", "Delete" }
                    }
                }

                if instance_manager.is_debug_mode() {
                    button {
                        class: "context-menu-button",
                        onclick: handle_debug_click,
                        div { class: "context-menu-icon",
                            img { src: AssetLoader::get_debug() }
                        }
                        div { class: "context-menu-text", "Debug" }
                    }
                }
            }
        }
    }
}
