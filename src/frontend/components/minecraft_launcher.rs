use crate::{backend::launcher::launcher::MinecraftLauncher, frontend::game_state::GameStatus};
use crate::{log_error, log_info, log_warn, simple_error};
use dioxus::prelude::*;

/// Launch Minecraft
pub fn launch_minecraft(game_status: Signal<GameStatus>, version: &str, instance_id: u32) {
    let version_owned = version.to_string();
    let mut game_status_signal = game_status;

    spawn(async move {
        log_info!("Starting Minecraft launch for version: {version_owned}");

        // Set launching state
        game_status_signal.set(GameStatus::Launching);

        // Run all operations in spawn_blocking to avoid blocking UI
        let launch_result = tokio::task::spawn_blocking({
            let version_owned = version_owned.clone();
            move || {
                // Create a new Tokio runtime for this blocking thread
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| simple_error!("Failed to create runtime: {}", e))?;

                rt.block_on(async {
                    let mut launcher = MinecraftLauncher::new(None, Some(instance_id))
                        .await
                        .map_err(|e| {
                            log_error!("Failed to create launcher: {e}");
                            e
                        })?;

                    log_info!("Launcher created successfully");

                    // Check if version exists locally
                    let game_dir = launcher.get_game_dir();
                    let version_dir = game_dir.join("versions").join(&version_owned);
                    let jar_file = version_dir.join(format!("{version_owned}.jar"));
                    let json_file = version_dir.join(format!("{version_owned}.json"));

                    let version_exists = jar_file.exists() && json_file.exists();

                    if !version_exists {
                        log_info!(
                            "Version {version_owned} not found locally, attempting to install..."
                        );

                        // Update manifest first
                        match launcher.update_manifest().await {
                            Ok(_) => log_info!("Manifest updated successfully"),
                            Err(e) => {
                                log_warn!("Failed to update manifest: {e}, continuing anyway...")
                            }
                        }

                        // Install/prepare the version
                        launcher
                            .prepare_version(&version_owned)
                            .await
                            .map_err(|e| {
                                log_error!("Failed to install version {version_owned}: {e}");
                                e
                            })?;

                        log_info!("Version {version_owned} installed successfully");
                    }

                    // Check Java availability
                    let java_available = launcher.is_java_available(&version_owned);

                    if !java_available {
                        log_info!("Java not available for version {version_owned}, installing...");

                        launcher.install_java(&version_owned).await.map_err(|e| {
                            log_error!("Failed to install Java for version {version_owned}: {e}");
                            e
                        })?;

                        log_info!("Java installed successfully for version {version_owned}");
                    }

                    Ok::<(), crate::utils::Error>(())
                })
            }
        })
        .await;

        // Handle the result of preparation
        match launch_result {
            Ok(Ok(())) => {
                log_info!("Minecraft preparation completed successfully");

                // Set running state
                game_status_signal.set(GameStatus::Running);
                log_info!("Starting Minecraft {version_owned}...");

                // Launch Minecraft in another spawn_blocking
                let launch_result = tokio::task::spawn_blocking({
                    let version_owned = version_owned.clone();
                    move || {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .map_err(|e| simple_error!("Failed to create runtime: {}", e))?;

                        rt.block_on(async {
                            let mut launcher =
                                MinecraftLauncher::new(None, Some(instance_id)).await?;
                            launcher.launch(&version_owned).await
                        })
                    }
                })
                .await;

                match launch_result {
                    Ok(Ok(_)) => {
                        log_info!("Minecraft {version_owned} launched and exited successfully");
                    }
                    Ok(Err(e)) => {
                        log_error!("Failed to launch Minecraft {version_owned}: {e}");
                    }
                    Err(e) => {
                        log_error!("Minecraft launch task failed: {e}");
                    }
                }
            }
            Ok(Err(e)) => {
                log_error!("Minecraft preparation failed: {e}");
            }
            Err(e) => {
                log_error!("Minecraft preparation task failed: {e}");
            }
        }

        // Set back to idle state
        game_status_signal.set(GameStatus::Idle);
        log_info!("Minecraft launch completed");
    });
}
