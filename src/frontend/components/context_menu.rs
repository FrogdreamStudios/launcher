use crate::backend::utils::assets::AssetLoader;
use crate::frontend::components::minecraft_launcher::launch_minecraft;
use crate::frontend::game_state::GameStatus;
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ContextMenuProps {
    pub show: Signal<bool>,
    pub x: Signal<f64>,
    pub y: Signal<f64>,
    pub game_status: Signal<GameStatus>,
}

#[component]
pub fn ContextMenu(props: ContextMenuProps) -> Element {
    let mut show = props.show;
    let x = props.x;
    let y = props.y;
    let game_status = props.game_status;
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
                // tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                is_hiding.set(true);
                // Hide after animation completes (150ms)
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
        println!("Run clicked - launching Minecraft 1.21.8");
        show.set(false);
        // Start Minecraft launch after menu closes
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(160)).await;
            launch_minecraft(game_status, "1.21.8");
        });
    };

    let handle_folder_click = move |e: Event<MouseData>| {
        e.stop_propagation();
        println!("Folder clicked - opening game folder");
        show.set(false);
        // Open game folder after menu closes
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(160)).await;
            if let Err(e) = open_game_folder().await {
                println!("Failed to open game folder: {}", e);
            }
        });
    };

    let handle_change_click = move |e: Event<MouseData>| {
        e.stop_propagation();
        println!("Change clicked");
        show.set(false);
    };

    let handle_delete_click = move |e: Event<MouseData>| {
        e.stop_propagation();
        println!("Delete clicked");
        show.set(false);
    };

    if !should_render() {
        return rsx! {};
    }

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
        }
    }
}

/// Open the Minecraft game folder in the system file explorer
async fn open_game_folder() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    // Get the Minecraft directory path
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/Users/unknown".to_string());
    let minecraft_dir = format!("{}/Library/Application Support/minecraft", home_dir);

    // Check if directory exists
    if !std::path::Path::new(&minecraft_dir).exists() {
        return Err("Minecraft directory not found".into());
    }

    // Open in Finder on macOS
    let output = Command::new("open").arg(&minecraft_dir).output()?;

    if !output.status.success() {
        return Err("Failed to open folder".into());
    }

    println!("Opened Minecraft folder: {}", minecraft_dir);
    Ok(())
}
