//! Instance management service.

use crate::backend::utils::paths::{get_cache_dir, get_launcher_dir};

use dioxus::prelude::*;
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
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "1.21.8".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstancesData {
    instances: HashMap<u32, Instance>,
    next_id: u32,
}

impl Instance {
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

pub static INSTANCES: GlobalSignal<HashMap<u32, Instance>> = Signal::global(HashMap::new);
pub static NEXT_ID: GlobalSignal<u32> = Signal::global(|| 1);
pub static DEBUG_MODE: GlobalSignal<bool> = Signal::global(|| false);
pub static INSTANCES_LOADED: GlobalSignal<bool> = Signal::global(|| false);

#[derive(Clone, Copy)]
pub struct InstanceManager;

impl InstanceManager {
    pub fn initialize() {
        if !*INSTANCES_LOADED.read() {
            spawn(async move {
                if let Err(e) = load_instances().await {
                    log::error!("Failed to load instances: {e}, creating default instance");
                    // Create default instance if loading fails
                    let mut instances = INSTANCES.write();
                    instances.insert(1, Instance::new_with_version(1, "1.21.8".to_string()));
                    *NEXT_ID.write() = 2;

                    // Create instance directories for the default instance
                    if let Err(e) = create_instance_directories(1) {
                        log::warn!("Failed to create directories for default instance: {e}");
                    }
                }
                *INSTANCES_LOADED.write() = true;
            });
        }
    }

    pub fn create_instance_with_version(version: &str) -> Option<u32> {
        log::info!(
            "create_instance_with_version called with version: {}",
            version
        );
        let mut instances = INSTANCES.write();
        let current_id = *NEXT_ID.read();
        log::info!(
            "Current instances count: {}, next_id: {}",
            instances.len(),
            current_id
        );

        // Check if we can create more instances (max 14)
        if instances.len() >= 14 {
            log::warn!(
                "Cannot create more instances, limit reached: {}",
                instances.len()
            );
            return None;
        }

        let new_instance = Instance::new_with_version(current_id, version.to_string());
        let instance_id = new_instance.id;
        log::info!("Creating instance {instance_id} with version: {version}");
        log::info!(
            "Instance created: id={}, name={}, version={}",
            new_instance.id,
            new_instance.name,
            new_instance.version
        );
        instances.insert(instance_id, new_instance);

        // Update next_id
        *NEXT_ID.write() = current_id + 1;

        // Calculate directory path before releasing the lock
        let folder_name = generate_folder_name_for_version(version, instance_id, &instances)
            .unwrap_or_else(|| format!("instance_{instance_id}"));
        let instance_dir = get_launcher_dir()
            .unwrap_or_else(|_| PathBuf::from("Dream Launcher"))
            .join(format!("instances/{folder_name}"));

        // Clone data for saving before releasing the lock
        let instances_data = InstancesData {
            instances: instances.clone(),
            next_id: current_id + 1,
        };

        // Release the lock before creating directories to avoid blocking other operations
        drop(instances);

        // Create instance directories
        if let Err(e) = create_instance_directories_with_path(&instance_dir) {
            log::warn!("Failed to create directories for instance {instance_id}: {e}");
        }

        spawn(async move {
            if let Err(e) = save_instances_data(&instances_data).await {
                log::error!("Failed to save instances: {e}");
            }
        });

        Some(instance_id)
    }

    pub fn delete_instance(id: u32) -> bool {
        let mut instances = INSTANCES.write();

        // Get the instance directory path BEFORE removing the instance
        let instance_dir = if let Some(instance) = instances.get(&id) {
            let folder_name = generate_folder_name_for_version(&instance.version, id, &instances)
                .unwrap_or_else(|| format!("instance_{id}"));
            get_launcher_dir()
                .unwrap_or_else(|_| PathBuf::from("Dream Launcher"))
                .join(format!("instances/{folder_name}"))
        } else {
            // Fallback if instance doesn't exist
            get_launcher_dir()
                .unwrap_or_else(|_| PathBuf::from("Dream Launcher"))
                .join(format!("instances/instance_{id}"))
        };

        let removed = instances.remove(&id).is_some();

        if removed {
            // Try to delete the instance directory
            if instance_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&instance_dir) {
                    log::warn!("Failed to delete instance {id} directory: {e}");
                } else {
                    log::info!("Deleted instance {id} directory: {instance_dir:?}");
                }
            }

            // Save instances to disk
            let instances_data = InstancesData {
                instances: instances.clone(),
                next_id: *NEXT_ID.read(),
            };
            drop(instances); // Release the write lock
            spawn(async move {
                if let Err(e) = save_instances_data(&instances_data).await {
                    log::error!("Failed to save instances: {e}");
                }
            });
        }

        removed
    }

    pub fn rename_instance(id: u32, new_name: &str) -> bool {
        let mut instances = INSTANCES.write();
        let renamed = if let Some(instance) = instances.get_mut(&id) {
            instance.name = new_name.chars().take(8).collect();
            true
        } else {
            false
        };

        if renamed {
            // Save instances to disk
            let instances_data = InstancesData {
                instances: instances.clone(),
                next_id: *NEXT_ID.read(),
            };
            drop(instances); // Release the write lock
            spawn(async move {
                if let Err(e) = save_instances_data(&instances_data).await {
                    log::error!("Failed to save instances: {e}");
                }
            });
        }

        renamed
    }

    pub fn toggle_debug_mode() {
        let current = *DEBUG_MODE.read();
        *DEBUG_MODE.write() = !current;
    }

    pub fn is_debug_mode() -> bool {
        *DEBUG_MODE.read()
    }

    pub fn can_create_instance() -> bool {
        INSTANCES.read().len() < 14
    }

    pub fn get_instances_sorted() -> Vec<Instance> {
        let instances = INSTANCES.read();
        let mut sorted: Vec<Instance> = instances.values().cloned().collect();
        sorted.sort_by_key(|i| i.id);
        sorted
    }
}

/// Helper function to generate folder name for existing instance.
fn generate_folder_name_for_version(
    version: &str,
    instance_id: u32,
    instances: &HashMap<u32, Instance>,
) -> Option<String> {
    let mut count = 1;
    // Sanitize version name for filesystem compatibility
    let base_name = version.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");

    // Count how many instances with this version exist before this instance_id
    for (id, instance) in instances {
        if *id < instance_id && instance.version == version {
            count += 1;
        }
    }

    if count > 1 {
        Some(format!("{}_{}", base_name, count))
    } else {
        Some(base_name)
    }
}

/// Get the directory for a specific instance.
pub fn get_instance_directory(instance_id: u32) -> PathBuf {
    let instances = INSTANCES.read();
    if let Some(instance) = instances.get(&instance_id) {
        let folder_name =
            generate_folder_name_for_version(&instance.version, instance_id, &instances)
                .unwrap_or_else(|| format!("instance_{instance_id}"));
        get_launcher_dir()
            .unwrap_or_else(|_| PathBuf::from("Dream Launcher"))
            .join(format!("instances/{folder_name}"))
    } else {
        // Fallback for instances that don't exist yet
        get_launcher_dir()
            .unwrap_or_else(|_| PathBuf::from("Dream Launcher"))
            .join(format!("instances/instance_{instance_id}"))
    }
}

/// Get the path to the instances' configuration file.
pub fn get_instances_config_path() -> PathBuf {
    get_cache_dir()
        .unwrap_or_else(|_| PathBuf::from("Dream Launcher/cache"))
        .join("launcher.json")
}

/// Create all necessary directories for an instance.
pub fn create_instance_directories(instance_id: u32) -> std::io::Result<PathBuf> {
    let instance_dir = get_instance_directory(instance_id);
    create_instance_directories_with_path(&instance_dir)?;
    Ok(instance_dir)
}

pub fn create_instance_directories_with_path(instance_dir: &PathBuf) -> std::io::Result<()> {
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

/// Save instances data to the disk.
async fn save_instances_data(data: &InstancesData) -> anyhow::Result<()> {
    let config_path = get_instances_config_path();

    // Debug: Print what we're about to save
    log::info!("Saving {} instances:", data.instances.len());
    for (id, instance) in &data.instances {
        log::info!(
            "  Instance {}: name='{}', version='{}'",
            id,
            instance.name,
            instance.version
        );
    }

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        async_fs::create_dir_all(parent).await?;
    }

    let json = serde_json::to_string_pretty(data)?;
    log::debug!("JSON to save: {json}");
    async_fs::write(config_path, json).await?;

    Ok(())
}

/// Load instances from disk.
async fn load_instances() -> anyhow::Result<()> {
    let config_path = get_instances_config_path();

    if !config_path.exists() {
        log::info!("No instances config found, creating default instance");
        return Ok(());
    }

    let json = async_fs::read_to_string(config_path).await?;
    let data: InstancesData = serde_json::from_str(&json)?;

    // Update global state
    *INSTANCES.write() = data.instances;
    *NEXT_ID.write() = data.next_id;

    log::info!("Loaded {} instances from config", INSTANCES.read().len());

    Ok(())
}

/// Open instance folder in system file explorer.
pub async fn open_instance_folder(instance_id: u32) -> anyhow::Result<()> {
    use std::process::Command;

    let instance_dir = get_instance_directory(instance_id);

    // Ensure directory exists
    if !instance_dir.exists() {
        create_instance_directories(instance_id)?;
    }

    // Open in Finder on macOS
    let output = Command::new("open").arg(&instance_dir).output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to open instance folder"));
    }

    log::info!("Opened instance {instance_id} folder: {instance_dir:?}");
    Ok(())
}
