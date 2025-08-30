use crate::backend::utils::paths::get_launcher_dir;
use crate::{
    backend::utils::css::ResourceLoader,
    frontend::{services::instances::main::INSTANCES, states::GameStatus},
};

use dioxus::prelude::*;
use std::fs;

#[derive(Clone)]
pub struct VersionSelection {
    pub is_loading: Signal<bool>,
    pub is_deleting: Signal<bool>,
    pub system_info: Signal<String>,
}

impl Default for VersionSelection {
    fn default() -> Self {
        Self {
            is_loading: Signal::new(false),
            is_deleting: Signal::new(false),
            system_info: Signal::new(String::new()),
        }
    }
}

#[component]
pub fn DebugWindow(
    show: Signal<bool>,
    version_selection: Signal<VersionSelection>,
    game_status: Signal<GameStatus>,
    instance_id: Signal<Option<u32>>,
) -> Element {
    if !show() {
        return rsx! { div { display: "none" } };
    }

    let vs = version_selection();
    let (loading, deleting) = (*vs.is_loading.read(), *vs.is_deleting.read());
    let busy = loading || deleting;

    // Get instance information outside of rsx!
    let (title, subtitle) = if let Some(id) = instance_id() {
        let instances = INSTANCES.read();
        if let Some(instance) = instances.get(&id) {
            (
                format!("Debug Panel - Instance {}", instance.name),
                format!("Instance ID: {} | Color: #{}", id, instance.color),
            )
        } else {
            (
                format!("Debug Panel - Instance {id}"),
                "Instance not found".to_string(),
            )
        }
    } else {
        (
            "Debug Panel".to_string(),
            "No instance selected".to_string(),
        )
    };

    rsx! {
        div {
            class: "debug-window-overlay",
            onclick: move |_| show.set(false),

            div {
                class: "debug-window",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "debug-header",
                    div {
                        h3 { class: "debug-title", "{title}" }
                        div { class: "debug-subtitle", "{subtitle}" }
                    }
                    button {
                        class: "debug-close",
                        onclick: move |_| show.set(false),
                        img { src: ResourceLoader::get_asset("close") }
                    }
                }

                // Content
                div {
                    class: "debug-content",

                    // Loading indicator
                    if busy {
                        div {
                            class: "debug-loading",
                            if loading { "Updating manifest..." } else { "Deleting launcher files..." }
                        }
                    }

                    // System Info
                    if !vs.system_info.read().is_empty() {
                        div {
                            class: "debug-system-info",
                            div { class: "debug-section-title", "System Info" }
                            pre { "{vs.system_info.read()}" }
                        }
                    }


                }

                // Actions
                div {
                    class: "debug-actions",

                    button {
                        class: if busy { "debug-btn debug-btn-disabled" } else { "debug-btn debug-btn-secondary" },
                        disabled: busy,
                        onclick: {
                            let mut system_info = vs.system_info;
                            let current_instance_id = instance_id();
                            move |_| {
                                spawn(async move {
                                    let result = if let Some(id) = current_instance_id {
                                        Ok(get_instance_info(id))
                                    } else {
                                        get_system_info().await
                                    };
                                    match result {
                                        Ok(info) => system_info.set(info),
                                        Err(e) => log::error!("Failed to get system info: {e}"),
                                    }
                                });
                            }
                        },
                        "System Info"
                    }

                    button {
                        class: if busy { "debug-btn debug-btn-disabled" } else { "debug-btn debug-btn-primary" },
                        disabled: busy,
                        onclick: {
                            let mut is_loading = vs.is_loading;
                            let is_deleting = vs.is_deleting;
                            move |_| {
                                if !*is_loading.read() && !*is_deleting.read() {
                                    is_loading.set(true);
                                    spawn(async move {
                                        match update_manifest().await {
                                            Ok(()) => log::info!("Manifest updated"),
            Err(e) => log::error!("Failed to update manifest: {e}"),
                                        }
                                        is_loading.set(false);
                                    });
                                }
                            }
                        },
                        if loading { "Updating..." } else { "Update Manifest" }
                    }



                    button {
                        class: if busy { "debug-btn debug-btn-disabled" } else { "debug-btn debug-btn-danger" },
                        disabled: busy,
                        onclick: {
                            let mut is_deleting = vs.is_deleting;
                            let is_loading = vs.is_loading;
                            move |_| {
                                if !*is_deleting.read() && !*is_loading.read() {
                                    is_deleting.set(true);
                                    spawn(async move {
                                        match delete_launcher_files().await {
                                            Ok(()) => log::info!("Launcher files deleted"),
            Err(e) => log::error!("Failed to delete files: {e}"),
                                        }
                                        is_deleting.set(false);
                                    });
                                }
                            }
                        },
                        if deleting { "Deleting..." } else { "Delete Files" }
                    }
                }
            }
        }
    }
}

async fn update_manifest() -> anyhow::Result<()> {
    log::info!("Update manifest functionality moved to Python bridge");
    // TODO: Implement manifest update through Python bridge if needed
    Ok(())
}

async fn delete_launcher_files() -> anyhow::Result<()> {
    log::info!("Starting launcher files deletion...");

    // Use standard Minecraft directory instead of launcher.get_game_dir()
    let game_dir = crate::backend::utils::paths::get_game_dir(None, None)?;

    log::info!("Game directory: {game_dir:?}");

    let directories = [
        ("versions", game_dir.join("versions")),
        ("libraries", game_dir.join("libraries")),
        ("assets", game_dir.join("assets")),
        ("natives", game_dir.join("natives")),
    ];

    let mut deleted_count = 0;
    let mut total_found = 0;

    // Count existing directories first
    for (name, path) in &directories {
        if path.exists() {
            total_found += 1;
            log::info!("Found {name} directory: {path:?}");
        }
    }

    if total_found == 0 {
        log::info!("No launcher files found to delete");
        return Ok(());
    }

    log::info!("Found {total_found} directories to delete");

    // Delete directories with progress
    for (name, path) in &directories {
        if path.exists() {
            log::info!(
                "Deleting {} directory... ({}/{})",
                name,
                deleted_count + 1,
                total_found
            );

            match fs::remove_dir_all(path) {
                Ok(()) => {
                    deleted_count += 1;
                    log::info!("✓ Successfully deleted {name} directory");
                }
                Err(e) => {
                    log::error!("✗ Failed to delete {name} directory: {e}");
                    return Err(anyhow::anyhow!("Failed to delete {name} directory: {e}"));
                }
            }
        }
    }

    log::info!("Deletion complete! Removed {deleted_count} directories");
    Ok(())
}

async fn get_system_info() -> anyhow::Result<String> {
    // Use standard Minecraft directory instead of launcher.get_game_dir()
    let game_dir = crate::backend::utils::paths::get_game_dir(None, None)?;

    let mut info = String::new();
    info.push_str(&format!("Game Directory: {game_dir:?}\n"));
    info.push_str(&format!("OS: {}\n", std::env::consts::OS));
    info.push_str(&format!("Architecture: {}\n", std::env::consts::ARCH));

    // Check directory sizes
    if game_dir.exists() {
        let versions_dir = game_dir.join("versions");
        let libraries_dir = game_dir.join("libraries");
        let assets_dir = game_dir.join("assets");

        if versions_dir.exists() {
            info.push_str(&format!("Versions directory exists: {versions_dir:?}\n"));
        }
        if libraries_dir.exists() {
            info.push_str(&format!("Libraries directory exists: {libraries_dir:?}\n"));
        }
        if assets_dir.exists() {
            info.push_str(&format!("Assets directory exists: {assets_dir:?}\n"));
        }
    }

    Ok(info)
}

fn get_instance_info(instance_id: u32) -> String {
    use crate::frontend::services::instances::main::get_instance_directory;

    let mut info = String::new();

    // Instance-specific info
    info.push_str(&format!("Instance ID: {instance_id}\n"));

    let instance_dir = get_instance_directory(instance_id);
    info.push_str(&format!("Instance directory: {instance_dir:?}\n"));

    let base_dir = get_launcher_dir().unwrap_or_else(|_| std::path::PathBuf::from("DreamLauncher"));
    info.push_str(&format!("Base Dream Launcher directory: {base_dir:?}\n"));

    // Check if the instance directory exists
    if instance_dir.exists() {
        info.push_str("Instance directory exists: Yes\n");

        // Check subdirectories
        let subdirs = [
            "mods",
            "config",
            "saves",
            "resourcepacks",
            "shaderpacks",
            "crash-reports",
            "logs",
        ];
        for subdir in subdirs {
            let dir_path = instance_dir.join(subdir);
            if dir_path.exists() {
                // Count files in a directory
                if let Ok(entries) = fs::read_dir(&dir_path) {
                    let count = entries.count();
                    info.push_str(&format!("{subdir}/: {count} items\n"));
                } else {
                    info.push_str(&format!("{subdir}/: exists but can't read\n"));
                }
            } else {
                info.push_str(&format!("{subdir}/: missing\n"));
            }
        }
    } else {
        info.push_str("Instance directory exists: No\n");
    }

    // System info
    info.push_str(&format!("\nOS: {}\n", std::env::consts::OS));
    info.push_str(&format!("Architecture: {}\n", std::env::consts::ARCH));

    info
}

pub fn use_version_selection() -> Signal<VersionSelection> {
    use_signal(VersionSelection::default)
}
