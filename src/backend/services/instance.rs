//! Instance management service.

use crate::backend::utils::paths::{get_cache_dir, get_launcher_dir};
use anyhow::Result;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tokio::fs as async_fs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    pub id: u32,
    pub name: String,
    pub color: String,
    pub level: u32,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstancesData {
    instances: HashMap<u32, Instance>,
    next_id: u32,
}

impl Instance {
    #[must_use]
    pub fn new_with_version(id: u32, version: String) -> Self {
        let colors = ["38FF10", "0077FF", "FF8C00", "F10246"];
        let color = colors[id as usize % colors.len()].to_string();

        Self {
            id,
            name: version.clone(),
            color,
            level: 28, // Default level
            version,
        }
    }
}

#[derive(Clone)]
pub struct InstanceService {
    instances: HashMap<u32, Instance>,
    next_id: u32,
}

impl InstanceService {
    #[must_use]
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            next_id: 1,
        }
    }

    /// Load instances from disk.
    pub async fn load_instances(&mut self) -> Result<()> {
        let config_path = self.get_instances_config_path();

        if !config_path.exists() {
            info!("No instances config found, creating default instance");
            self.create_default_instance().await?;
            return Ok(());
        }

        let json = async_fs::read_to_string(config_path).await?;
        let data: InstancesData = serde_json::from_str(&json)?;

        self.instances = data.instances;
        self.next_id = data.next_id;

        Ok(())
    }

    /// Create a default instance.
    async fn create_default_instance(&mut self) -> Result<()> {
        let instance = Instance::new_with_version(1, "1.21.8".to_string());
        self.instances.insert(1, instance);
        self.next_id = 2;

        // Create instance directories
        if let Err(e) = self.create_instance_directories(1) {
            warn!("Failed to create directories for default instance: {e}");
        }

        self.save_instances().await?;
        Ok(())
    }

    /// Get instances sorted by ID.
    #[must_use]
    pub fn get_instances_sorted(&self) -> Vec<Instance> {
        let mut sorted: Vec<Instance> = self.instances.values().cloned().collect();
        sorted.sort_by_key(|i| i.id);
        sorted
    }

    /// Get a specific instance by ID.
    #[must_use]
    pub fn get_instance(&self, id: u32) -> Option<&Instance> {
        self.instances.get(&id)
    }

    /// Create a new instance with the specified version.
    pub async fn create_instance_with_version(&mut self, version: &str) -> Result<Option<u32>> {
        info!("create_instance_with_version called with version: {version}");

        // Check if we can create more instances (max 14)
        if self.instances.len() >= 14 {
            warn!(
                "Cannot create more instances, limit reached: {}",
                self.instances.len()
            );
            return Ok(None);
        }

        let current_id = self.next_id;
        let new_instance = Instance::new_with_version(current_id, version.to_string());
        let instance_id = new_instance.id;

        info!("Creating instance {instance_id} with version: {version}");
        self.instances.insert(instance_id, new_instance);
        self.next_id = current_id + 1;

        // Create instance directories
        let folder_name = self
            .generate_folder_name_for_version(version, instance_id)
            .unwrap_or_else(|| format!("instance_{instance_id}"));
        let instance_dir = get_launcher_dir()
            .unwrap_or_else(|_| PathBuf::from("Dream Launcher"))
            .join(format!("instances/{folder_name}"));

        if let Err(e) = self.create_instance_directories_with_path(&instance_dir) {
            warn!("Failed to create directories for instance {instance_id}: {e}");
        }

        self.save_instances().await?;
        Ok(Some(instance_id))
    }

    /// Delete an instance.
    pub async fn delete_instance(&mut self, id: u32) -> Result<bool> {
        // Get the instance directory path BEFORE removing the instance
        let instance_dir = if let Some(instance) = self.instances.get(&id) {
            let folder_name = self
                .generate_folder_name_for_version(&instance.version, id)
                .unwrap_or_else(|| format!("instance_{id}"));
            get_launcher_dir()
                .unwrap_or_else(|_| PathBuf::from("Dream Launcher"))
                .join(format!("instances/{folder_name}"))
        } else {
            return Ok(false);
        };

        let removed = self.instances.remove(&id).is_some();

        if removed {
            // Try to delete the instance directory
            if instance_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&instance_dir) {
                    warn!("Failed to delete instance {id} directory: {e}");
                } else {
                    info!("Deleted instance {id} directory: {instance_dir:?}");
                }
            }

            self.save_instances().await?;
        }

        Ok(removed)
    }

    /// Rename an instance
    pub async fn rename_instance(&mut self, id: u32, new_name: &str) -> Result<bool> {
        let renamed = if let Some(instance) = self.instances.get_mut(&id) {
            instance.name = new_name.chars().take(8).collect();
            true
        } else {
            false
        };

        if renamed {
            self.save_instances().await?;
        }

        Ok(renamed)
    }

    /// Get the directory for a specific instance.
    pub fn get_instance_directory(&self, instance_id: u32) -> PathBuf {
        if let Some(instance) = self.instances.get(&instance_id) {
            let folder_name = self
                .generate_folder_name_for_version(&instance.version, instance_id)
                .unwrap_or_else(|| format!("instance_{instance_id}"));
            get_launcher_dir()
                .unwrap_or_else(|_| PathBuf::from("Dream Launcher"))
                .join(format!("instances/{folder_name}"))
        } else {
            get_launcher_dir()
                .unwrap_or_else(|_| PathBuf::from("Dream Launcher"))
                .join(format!("instances/instance_{instance_id}"))
        }
    }

    /// Open instance folder in system file explorer.
    pub async fn open_instance_folder(&self, instance_id: u32) -> Result<()> {
        use std::process::Command;

        let instance_dir = self.get_instance_directory(instance_id);

        // Ensure directory exists
        if !instance_dir.exists() {
            self.create_instance_directories(instance_id)?;
        }

        // Open in Finder on macOS
        let output = Command::new("open").arg(&instance_dir).output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to open instance folder"));
        }

        info!("Opened instance {instance_id} folder: {instance_dir:?}");
        Ok(())
    }

    /// Generate folder name for a version.
    fn generate_folder_name_for_version(&self, version: &str, instance_id: u32) -> Option<String> {
        let mut count = 1;
        let base_name = version.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");

        // Count how many instances with this version exist before this instance_id
        for (id, instance) in &self.instances {
            if *id < instance_id && instance.version == version {
                count += 1;
            }
        }

        if count > 1 {
            Some(format!("{base_name}_{count}"))
        } else {
            Some(base_name)
        }
    }

    /// Create all necessary directories for an instance.
    fn create_instance_directories(&self, instance_id: u32) -> std::io::Result<PathBuf> {
        let instance_dir = self.get_instance_directory(instance_id);
        self.create_instance_directories_with_path(&instance_dir)?;
        Ok(instance_dir)
    }

    /// Create directories with a specific path.
    fn create_instance_directories_with_path(&self, instance_dir: &PathBuf) -> std::io::Result<()> {
        // Create the main instance directory
        fs::create_dir_all(instance_dir)?;

        // Create subdirectories
        fs::create_dir_all(instance_dir.join("minecraft"))?;
        fs::create_dir_all(instance_dir.join("mods"))?;
        fs::create_dir_all(instance_dir.join("config"))?;
        fs::create_dir_all(instance_dir.join("saves"))?;
        fs::create_dir_all(instance_dir.join("resourcepacks"))?;
        fs::create_dir_all(instance_dir.join("shaderpacks"))?;
        fs::create_dir_all(instance_dir.join("crash-reports"))?;
        fs::create_dir_all(instance_dir.join("logs"))?;

        Ok(())
    }

    /// Get the path to the instances configuration file.
    fn get_instances_config_path(&self) -> PathBuf {
        get_cache_dir()
            .unwrap_or_else(|_| PathBuf::from("Dream Launcher/cache"))
            .join("launcher.json")
    }

    /// Save instances data to disk.
    async fn save_instances(&self) -> Result<()> {
        let config_path = self.get_instances_config_path();
        let data = InstancesData {
            instances: self.instances.clone(),
            next_id: self.next_id,
        };

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            async_fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string_pretty(&data)?;
        async_fs::write(config_path, json).await?;

        Ok(())
    }
}

impl Default for InstanceService {
    fn default() -> Self {
        Self::new()
    }
}
