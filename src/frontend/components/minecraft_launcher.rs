use crate::backend::creeper::launcher::MinecraftLauncher;
use crate::frontend::game_state::GameStatus;
use dioxus::prelude::*;
use tracing::{error, info, warn};

/// Launch Minecraft
pub fn launch_minecraft(game_status: Signal<GameStatus>, version: &str) {
    let version_owned = version.to_string();
    let mut game_status_signal = game_status;

    spawn(async move {
        info!("Starting Minecraft launch for version: {}", version_owned);

        // Set launching state
        game_status_signal.set(GameStatus::Launching);

        // Create launcher
        let mut launcher = match MinecraftLauncher::new(None).await {
            Ok(launcher) => {
                info!("Launcher created successfully");
                launcher
            }
            Err(e) => {
                error!("Failed to create launcher: {}", e);
                game_status_signal.set(GameStatus::Idle);
                return;
            }
        };

        // Check if version exists locally
        let game_dir = launcher.get_game_dir();
        let version_dir = game_dir.join("versions").join(&version_owned);
        let jar_file = version_dir.join(format!("{}.jar", version_owned));
        let json_file = version_dir.join(format!("{}.json", version_owned));

        let version_exists = jar_file.exists() && json_file.exists();

        if !version_exists {
            info!("Version {version_owned} not found locally, attempting to install...");

            // Update manifest first
            match launcher.update_manifest().await {
                Ok(_) => info!("Manifest updated successfully"),
                Err(e) => warn!("Failed to update manifest: {e}, continuing anyway..."),
            }

            // Install/prepare the version
            match launcher.prepare_version(&version_owned).await {
                Ok(_) => {
                    info!("Version {version_owned} installed successfully");
                }
                Err(e) => {
                    error!("Failed to install version {version_owned}: {e}");
                    game_status_signal.set(GameStatus::Idle);
                    return;
                }
            }
        }

        // Check Java availability
        let java_available = launcher
            .is_java_available(&version_owned)
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to check Java availability: {e}, continuing anyway...");
                true
            });

        if !java_available {
            info!("Java not available for version {version_owned}, installing...");

            match launcher.install_java(&version_owned).await {
                Ok(_) => {
                    info!("Java installed successfully for version {version_owned}");
                }
                Err(e) => {
                    error!("Failed to install Java for version {version_owned}: {e}",);
                    game_status_signal.set(GameStatus::Idle);
                    return;
                }
            }
        }

        // Set running state
        info!("Starting Minecraft {}...", version_owned);
        game_status_signal.set(GameStatus::Running);

        // Launch Minecraft
        match launcher.launch(&version_owned).await {
            Ok(_) => {
                info!("Minecraft {version_owned} launched and exited successfully");
            }
            Err(e) => {
                error!("Failed to launch Minecraft {version_owned}: {e}");
            }
        }

        // Set back to idle state
        game_status_signal.set(GameStatus::Idle);
        info!("Minecraft launch completed");
    });
}
