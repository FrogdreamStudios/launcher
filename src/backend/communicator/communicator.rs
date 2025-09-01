//! Frontend-backend communication.

use crate::backend::archon::Archon;
use crate::backend::python::python::MinecraftLogMessage;
use crate::backend::services::{
    Instance, InstanceService, LaunchResult, LauncherService, VersionManifest,
};
use anyhow::Result;
use log::{error, info, warn};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

/// Main interface for frontend-backend communication.
#[derive(Clone)]
pub struct Communicator {
    instance_service: Arc<Mutex<InstanceService>>,
    launcher_service: Arc<Mutex<LauncherService>>,
    archon: Arc<Archon>,
}

impl Communicator {
    /// Create a new instance.
    pub async fn new(archon: Arc<Archon>) -> Result<Self> {
        let mut instance_service = InstanceService::new();
        instance_service.load_instances().await?;
        let instance_service = Arc::new(Mutex::new(instance_service));

        let launcher_service = LauncherService::new(instance_service.clone(), archon.clone())?;
        let launcher_service = Arc::new(Mutex::new(launcher_service));

        Ok(Self {
            instance_service,
            launcher_service,
            archon,
        })
    }

    // Instance management

    /// Get all instances.
    pub async fn get_instances(&self) -> Result<Vec<Instance>> {
        let launcher_service = self.launcher_service.lock().await;
        launcher_service.get_instances().await
    }

    /// Create a new instance with the specified version.
    pub async fn create_instance(&self, version: &str) -> Result<Option<u32>> {
        let launcher_service = self.launcher_service.lock().await;
        launcher_service.create_instance_with_version(version).await
    }

    /// Delete an instance.
    pub async fn delete_instance(&self, instance_id: u32) -> Result<bool> {
        let launcher_service = self.launcher_service.lock().await;
        launcher_service.delete_instance(instance_id).await
    }

    /// Rename an instance.
    pub async fn rename_instance(&self, instance_id: u32, new_name: &str) -> Result<bool> {
        let launcher_service = self.launcher_service.lock().await;
        launcher_service
            .rename_instance(instance_id, new_name)
            .await
    }

    /// Open instance folder in system file explorer.
    pub async fn open_instance_folder(&self, instance_id: u32) -> Result<()> {
        let launcher_service = self.launcher_service.lock().await;
        launcher_service.open_instance_folder(instance_id).await
    }

    // Version management

    /// Get version manifest.
    pub async fn get_version_manifest(&self) -> Result<VersionManifest> {
        let launcher_service = self.launcher_service.lock().await;
        launcher_service.get_version_manifest().await
    }

    // Launch management

    /// Install and launch a Minecraft instance.
    pub async fn install_and_launch_instance(
        &self,
        instance_id: u32,
        version: &str,
        log_sender: mpsc::UnboundedSender<MinecraftLogMessage>,
    ) -> Result<LaunchResult> {
        let launcher_service = self.launcher_service.lock().await;
        launcher_service
            .install_and_launch_instance(instance_id, version, log_sender)
            .await
    }

    /// Install Minecraft dependencies.
    pub async fn install_dependencies(&self) -> Result<()> {
        // Dependencies are installed during service initialization
        Ok(())
    }

    // User configuration

    /// Save user configuration to disk.
    pub async fn save_user_config(&self, config_json: &str) -> Result<()> {
        use crate::backend::utils::paths::get_cache_dir;

        let config_path = get_cache_dir()?.join("user_config.json");
        info!(
            "Attempting to save user config to: {}",
            config_path.display()
        );

        // Ensure parent directory exists using file thread
        if let Some(parent) = config_path.parent() {
            info!("Creating parent directory: {}", parent.display());
            match self
                .archon
                .file_operation(
                    "create_dir".to_string(),
                    parent.to_string_lossy().to_string(),
                    None,
                )
                .await
            {
                Ok(response) => {
                    if !response.success {
                        let error_msg = response.error.unwrap_or("Unknown error".to_string());
                        error!(
                            "Failed to create parent directory {}: {}",
                            parent.display(),
                            error_msg
                        );
                        return Err(anyhow::anyhow!(error_msg));
                    }
                    info!("Parent directory created successfully");
                }
                Err(e) => {
                    error!(
                        "Failed to create parent directory {}: {}",
                        parent.display(),
                        e
                    );
                    return Err(e);
                }
            }
        }

        // Write file using Archon
        info!("Writing config file with {} bytes", config_json.len());
        match self
            .archon
            .file_operation(
                "write".to_string(),
                config_path.to_string_lossy().to_string(),
                Some(config_json.as_bytes().to_vec()),
            )
            .await
        {
            Ok(response) => {
                if !response.success {
                    let error_msg = response.error.unwrap_or("Unknown error".to_string());
                    error!(
                        "Failed to write config file {}: {}",
                        config_path.display(),
                        error_msg
                    );
                    return Err(anyhow::anyhow!(error_msg));
                }
                info!(
                    "User config saved successfully to: {}",
                    config_path.display()
                );
            }
            Err(e) => {
                error!(
                    "Failed to write config file {}: {}",
                    config_path.display(),
                    e
                );
                return Err(e);
            }
        }

        Ok(())
    }

    /// Load user configuration from disk.
    pub async fn load_user_config(&self) -> Result<String> {
        use crate::backend::utils::paths::get_cache_dir;

        let config_path = get_cache_dir()?.join("user_config.json");

        match self
            .archon
            .file_operation(
                "read".to_string(),
                config_path.to_string_lossy().to_string(),
                None,
            )
            .await
        {
            Ok(response) => {
                if !response.success {
                    let error_msg = response
                        .error
                        .unwrap_or("User config file does not exist".to_string());
                    return Err(anyhow::anyhow!(error_msg));
                }

                if let Some(data) = response.data {
                    Ok(String::from_utf8(data)?)
                } else {
                    Err(anyhow::anyhow!("No data returned from file operation"))
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Delete user configuration file.
    pub async fn delete_user_config(&self) -> Result<()> {
        use crate::backend::utils::paths::get_cache_dir;

        let config_path = get_cache_dir()?.join("user_config.json");

        // Delete file
        match self
            .archon
            .file_operation(
                "delete".to_string(),
                config_path.to_string_lossy().to_string(),
                None,
            )
            .await
        {
            Ok(response) => {
                if !response.success {
                    let error_msg = response
                        .error
                        .unwrap_or("Failed to delete config file".to_string());
                    warn!("Failed to delete config file: {error_msg}");
                }
            }
            Err(e) => {
                warn!("Failed to delete config file: {e}");
            }
        }

        Ok(())
    }
}
