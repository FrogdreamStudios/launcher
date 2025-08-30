//! Minecraft command building.

use crate::utils::Result;
use crate::{log_error, log_info, simple_error};

/// Launch Minecraft with the specified version and instance using Python bridge.
pub async fn launch_minecraft(version: String, _instance_id: u32, username: String) -> Result<()> {
    use crate::backend::utils::progress_bridge::{send_progress_stage, send_progress_custom};
    use crate::backend::launcher::progress::ProgressStage;
    use crate::backend::bridge::{PythonMinecraftBridge, LaunchConfig};

    log_info!("Starting Minecraft launch for version: {version} using Python bridge");

    // Step 1: Preparing
    send_progress_stage(ProgressStage::Preparing, &version);

    // Create Python bridge
    let bridge = PythonMinecraftBridge::new().map_err(|e| {
        log_error!("Failed to create Python bridge: {e}");
        send_progress_custom(ProgressStage::Failed, 0.0, format!("Error: {}", e));
        simple_error!("Failed to create Python bridge: {e}")
    })?;

    log_info!("Python bridge created successfully");
    
    // Step 2: Launching
    send_progress_stage(ProgressStage::Launching, &version);

    // Create launch configuration
    let config = LaunchConfig {
        username: username.clone(),
        version: version.clone(),
    };

    // Launch Minecraft through Python
    let result = bridge.launch_minecraft(config).await.map_err(|e| {
        log_error!("Failed to launch Minecraft through Python bridge: {e}");
        send_progress_custom(ProgressStage::Failed, 0.0, format!("Error: {}", e));
        simple_error!("Failed to launch Minecraft: {e}")
    })?;

    if result.success {
        log_info!("Minecraft {version} launched successfully: {}", result.message);
        if let Some(pid) = result.pid {
            log_info!("Minecraft process PID: {pid}");
        }
        
        // Step 3: Running
        send_progress_stage(ProgressStage::Running, &version);
        
        // Wait a bit to show "running" status, then complete
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        send_progress_stage(ProgressStage::Completed, &version);
        
        Ok(())
    } else {
        log_error!("Failed to launch Minecraft {version}: {}", result.message);
        send_progress_custom(ProgressStage::Failed, 0.0, format!("Error: {}", result.message));
        Err(simple_error!("Failed to launch Minecraft: {}", result.message))
    }
}
