use crate::{backend::utils::launcher::starter, frontend::game_state::GameStatus};
use crate::{log_error, log_info};
use dioxus::prelude::*;

/// Launch Minecraft
pub fn launch_minecraft(game_status: Signal<GameStatus>, version: &str, instance_id: u32) {
    let version_owned = version.to_string();
    let mut game_status_signal = game_status;

    spawn(async move {
        log_info!("Starting Minecraft launch for version: {version_owned}");

        // Set the launching state
        game_status_signal.set(GameStatus::Launching);

        // Use the simplified launch function from starter.rs
        let launch_result = tokio::task::spawn_blocking({
            let version_owned = version_owned.clone();
            move || {
                // Create a new Tokio runtime for this blocking thread
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("Failed to create runtime: {}", e))?;

                rt.block_on(async { starter::launch_minecraft(version_owned, instance_id).await })
            }
        })
        .await;

        // Handle the result
        match launch_result {
            Ok(Ok(_)) => {
                log_info!("Minecraft {version_owned} launched and completed successfully");
            }
            Ok(Err(e)) => {
                log_error!("Failed to launch Minecraft {version_owned}: {e}");
            }
            Err(e) => {
                log_error!("Minecraft launch task failed: {e}");
            }
        }

        // Set back to idle state
        game_status_signal.set(GameStatus::Idle);
        log_info!("Minecraft launch completed");
    });
}
