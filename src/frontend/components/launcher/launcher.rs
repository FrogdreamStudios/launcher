//! Communicator between frontend and backend.

use dioxus::prelude::*;
use crate::backend::services::starter;
use crate::frontend::services::states::GameStatus;

/// Launching Minecraft. Note that the process of launching Minecraft is in the
/// backend, in `starter.rs`.
    pub fn launch_minecraft(
    _game_status: Signal<GameStatus>,
    version: &str,
    instance_id: u32,
    username: &str,
) {
    let version_owned = version.to_string();
    let username_owned = username.to_string();

    spawn(async move {
        log::info!("Launching Minecraft...");
        let launch_result = tokio::task::spawn_blocking({
            let version_owned = version_owned.clone();
            let username_owned = username_owned.clone();
            move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| anyhow::anyhow!("Failed to create runtime: {e}"))?;

                rt.block_on(async {
                    starter::launch_minecraft(version_owned, instance_id, username_owned).await
                })
            }
        })
        .await;

        match launch_result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                log::error!("Failed to launch Minecraft: {e}");
            }
            Err(e) => {
                log::error!("Launch task failed: {e}");
            }
        }
    });
}
