use crate::{backend::utils::launcher::starter, frontend::states::GameStatus};
use crate::{log_error, log_info};
use dioxus::prelude::*;

/// Launch Minecraft.
pub fn launch_minecraft(game_status: Signal<GameStatus>, version: &str, instance_id: u32) {
    let version_owned = version.to_string();
    let mut game_status_signal = game_status;

    spawn(async move {
        println!("Received version: {version_owned}");
        println!("Received instance_id: {instance_id}");
        log_info!("Starting Minecraft launch for version: {version_owned}");

        // Set the initial launching state
        game_status_signal.set(GameStatus::Launching {
            progress: 0.0,
            message: "Initializing launch...".to_string(),
        });

        // Use the simplified launch function from starter.rs
        let launch_result = tokio::task::spawn_blocking({
            let version_owned = version_owned.clone();
            move || {
                // Create a new Tokio runtime for this blocking thread
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("Failed to create runtime: {e}"))?;

                println!(
                    "Calling starter::launch_minecraft with version: {}",
                    version_owned
                );
                rt.block_on(async { starter::launch_minecraft(version_owned, instance_id).await })
            }
        })
        .await;

        // Handle the result
        match launch_result {
            Ok(Ok(())) => {
                log_info!("Minecraft {version_owned} launched and completed successfully");
            }
            Ok(Err(e)) => {
                log_error!("Failed to launch Minecraft {version_owned}: {e}");
                game_status_signal.set(GameStatus::Idle);
            }
            Err(e) => {
                log_error!("Minecraft launch task failed: {e}");
                game_status_signal.set(GameStatus::Idle);
            }
        }
        log_info!("Minecraft launch process completed");
    });
}
