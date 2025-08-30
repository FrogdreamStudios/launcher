//! Minecraft command building.

use anyhow::Result;

/// Launch Minecraft with the specified version and instance using Python bridge.
pub async fn launch_minecraft(version: String, _instance_id: u32, username: String) -> Result<()> {
    use crate::backend::bridge::{LaunchConfig, PythonMinecraftBridge};

    log::info!("Starting Minecraft launch for version: {version} using Python bridge");

    // Create Python bridge
    let bridge = PythonMinecraftBridge::new().map_err(|e| {
        log::error!("Failed to create Python bridge: {e}");
        anyhow::anyhow!("Failed to create Python bridge: {e}")
    })?;

    log::info!("Python bridge created successfully");

    // Create launch configuration
    let config = LaunchConfig {
        username: username.clone(),
        version: version.clone(),
    };

    // Launch Minecraft through Python
    let result = bridge.launch_minecraft(config).await.map_err(|e| {
        log::error!("Failed to launch Minecraft through Python bridge: {e}");
        anyhow::anyhow!("Failed to launch Minecraft: {e}")
    })?;

    if result.success {
        log::info!(
            "Minecraft {version} launched successfully: {}",
            result.message
        );
        if let Some(pid) = result.pid {
            log::info!("Minecraft process PID: {pid}");
        }

        Ok(())
    } else {
        log::error!("Failed to launch Minecraft {version}: {}", result.message);
        Err(anyhow::anyhow!(
            "Failed to launch Minecraft: {}",
            result.message
        ))
    }
}
