//! Launcher service for managing Minecraft installation and launch.

use crate::backend::archon::Archon;
use crate::backend::python::python::{LaunchConfig, MinecraftLogMessage};
use crate::backend::services::instance::{Instance, InstanceService};
use crate::backend::utils::paths::get_launcher_dir;
use anyhow::Result;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub sha1: String,
    #[serde(rename = "complianceLevel")]
    pub compliance_level: u32,
}

#[derive(Debug, Clone)]
pub struct LaunchResult {
    pub success: bool,
    pub message: String,
    pub pid: Option<u32>,
}

#[derive(Clone)]
pub struct LauncherService {
    archon: Arc<Archon>,
    instance_service: Arc<Mutex<InstanceService>>,
    version_manifest: Arc<Mutex<Option<VersionManifest>>>,
}

impl LauncherService {
    pub fn new(instance_service: Arc<Mutex<InstanceService>>, archon: Arc<Archon>) -> Result<Self> {
        Ok(Self {
            archon,
            instance_service,
            version_manifest: Arc::new(Mutex::new(None)),
        })
    }

    /// Fetch the Minecraft version manifest.
    pub async fn fetch_version_manifest(&self) -> Result<VersionManifest> {
        let url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
        let response = reqwest::get(url).await?;
        let manifest: VersionManifest = response.json().await?;

        // Cache the manifest
        {
            let mut cached_manifest = self.version_manifest.lock().await;
            *cached_manifest = Some(manifest.clone());
        }

        info!("Fetched {} versions from manifest", manifest.versions.len());
        Ok(manifest)
    }

    /// Get cached version manifest or fetch if not available.
    pub async fn get_version_manifest(&self) -> Result<VersionManifest> {
        {
            let cached_manifest = self.version_manifest.lock().await;
            if let Some(ref manifest) = *cached_manifest {
                return Ok(manifest.clone());
            }
        }

        // Fetch if not cached
        self.fetch_version_manifest().await
    }

    /// Get available Minecraft versions.
    pub async fn get_available_versions(&self) -> Result<Vec<VersionInfo>> {
        let manifest = self.get_version_manifest().await?;
        Ok(manifest.versions)
    }

    /// Install and launch a Minecraft instance.
    pub async fn install_and_launch_instance(
        &self,
        instance_id: u32,
        version: &str,
        log_sender: mpsc::UnboundedSender<MinecraftLogMessage>,
    ) -> Result<LaunchResult> {
        info!("Starting installation and launch for instance {instance_id} with version {version}");

        // Get instance information
        let instance = {
            let instance_service = self.instance_service.lock().await;
            instance_service
                .get_instance(instance_id)
                .ok_or_else(|| anyhow::anyhow!("Instance {} not found", instance_id))?
                .clone()
        };

        // Get instance directory
        let instance_dir = {
            let instance_service = self.instance_service.lock().await;
            instance_service.get_instance_directory(instance_id)
        };

        // Get minecraft directory
        let minecraft_dir = get_launcher_dir()?.join("minecraft");

        info!("Instance directory: {instance_dir:?}");
        info!("Minecraft directory: {minecraft_dir:?}");

        // Send initial launch result
        if let Err(e) = log_sender.send(MinecraftLogMessage::LaunchResult {
            success: true,
            message: format!(
                "Starting Minecraft {} for instance {}",
                version, instance.name
            ),
            pid: None,
        }) {
            warn!("Failed to send launch result: {e}");
        }

        // Get Python operations from Archon
        let archon = self.archon.clone();

        // First install the version using Archon
        info!("Installing Minecraft version {version} through Archon");
        if let Err(e) = log_sender.send(MinecraftLogMessage::LaunchResult {
            success: true,
            message: format!("Installing Minecraft {version}"),
            pid: None,
        }) {
            warn!("Failed to send installation status: {e}");
        }

        match archon
            .python_operation(
                "install_minecraft".to_string(),
                vec![
                    version.to_string(),
                    minecraft_dir.to_string_lossy().to_string(),
                ],
            )
            .await
        {
            Ok(response) => {
                if !response.success {
                    let error_msg = response.error.unwrap_or("Unknown error".to_string());
                    error!("Version installation failed: {error_msg}");
                    return Ok(LaunchResult {
                        success: false,
                        message: format!("Failed to install version {version}: {error_msg}"),
                        pid: None,
                    });
                }
                info!("Version {version} installed successfully");
            }
            Err(e) => {
                error!("Failed to install version through Archon: {e}");
                return Ok(LaunchResult {
                    success: false,
                    message: format!("Failed to install version {version}: {e}"),
                    pid: None,
                });
            }
        }

        // Create launch configuration
        let launch_config = LaunchConfig {
            username: "Player".to_string(),
            version: version.to_string(),
            java_path: None,
            jvm_args: vec!["-Xmx2G".to_string(), "-Xms1G".to_string()],
            game_args: vec![],
            access_token: "dummy_token".to_string(),
            uuid: "00000000-0000-0000-0000-000000000000".to_string(),
        };

        // Launch through Archon
        info!("Launching Minecraft through Archon");
        let launch_args = vec![
            launch_config.username,
            launch_config.version,
            minecraft_dir.to_string_lossy().to_string(),
            instance_dir.to_string_lossy().to_string(),
        ];

        match archon
            .python_operation("launch_minecraft".to_string(), launch_args)
            .await
        {
            Ok(response) => {
                if response.success {
                    let pid = response
                        .data
                        .as_ref()
                        .and_then(|d| d.get("pid"))
                        .and_then(serde_json::Value::as_u64)
                        .map(|p| p as u32);
                    info!("Minecraft launched successfully");
                    Ok(LaunchResult {
                        success: true,
                        message: "Minecraft launched successfully".to_string(),
                        pid,
                    })
                } else {
                    let error_msg = response.error.unwrap_or("Unknown error".to_string());
                    error!("Minecraft launch failed: {error_msg}");
                    Ok(LaunchResult {
                        success: false,
                        message: error_msg,
                        pid: None,
                    })
                }
            }
            Err(e) => {
                error!("Failed to launch Minecraft through Archon: {e}");
                Ok(LaunchResult {
                    success: false,
                    message: format!("Failed to launch Minecraft: {e}"),
                    pid: None,
                })
            }
        }
    }

    /// Check if a specific version is available.
    pub async fn is_version_available(&self, version: &str) -> Result<bool> {
        let versions = self.get_available_versions().await?;
        Ok(versions.iter().any(|v| v.id == version))
    }

    /// Create a new instance with a specific version.
    pub async fn create_instance_with_version(&self, version: &str) -> Result<Option<u32>> {
        // Verify version exists
        if !self.is_version_available(version).await? {
            return Err(anyhow::anyhow!("Version {} is not available", version));
        }

        let mut instance_service = self.instance_service.lock().await;
        instance_service.create_instance_with_version(version).await
    }

    /// Get all instances.
    pub async fn get_instances(&self) -> Result<Vec<Instance>> {
        let instance_service = self.instance_service.lock().await;
        Ok(instance_service.get_instances_sorted())
    }

    /// Delete an instance.
    pub async fn delete_instance(&self, instance_id: u32) -> Result<bool> {
        let mut instance_service = self.instance_service.lock().await;
        instance_service.delete_instance(instance_id).await
    }

    /// Rename an instance.
    pub async fn rename_instance(&self, instance_id: u32, new_name: &str) -> Result<bool> {
        let mut instance_service = self.instance_service.lock().await;
        instance_service
            .rename_instance(instance_id, new_name)
            .await
    }

    /// Open instance folder.
    pub async fn open_instance_folder(&self, instance_id: u32) -> Result<()> {
        let instance_service = self.instance_service.lock().await;
        instance_service.open_instance_folder(instance_id).await
    }
}
