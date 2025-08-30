//! Instance management service.

use crate::backend::utils::paths::get_launcher_dir;

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
    pub fn new(id: u32) -> Self {
        let colors = ["38FF10", "0077FF", "FF8C00", "F10246"];
        let color = colors[id as usize % colors.len()].to_string();

        Self {
            id,
            name: "1.21.8".to_string(),
            color,
            level: 28, // Default level
            version: "1.21.8".to_string(),
        }
    }

    pub fn new_with_version(id: u32, version: String) -> Self {
        let colors = ["38FF10", "0077FF", "FF8C00", "F10246"];
        let color = colors[id as usize % colors.len()].to_string();

        Self {
            id,
            name: format!("Instance {id}"),
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
                    instances.insert(1, Instance::new(1));
                    *NEXT_ID.write() = 2;
                }
                *INSTANCES_LOADED.write() = true;
            });
        }
    }

    pub fn create_instance_with_version(version: &str) -> Option<u32> {
        let mut instances = INSTANCES.write();
        let current_id = *NEXT_ID.read();

        // Check if we can create more instances (max 14)
        if instances.len() >= 14 {
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

        // Create instance directories
        if let Err(e) = create_instance_directories(instance_id) {
            log::warn!("Failed to create directories for instance {instance_id}: {e}");
        }

        // Update next_id
        *NEXT_ID.write() = current_id + 1;

        // Save instances to disk
        let instances_data = InstancesData {
            instances: instances.clone(),
            next_id: current_id + 1,
        };
        drop(instances);

        spawn(async move {
            if let Err(e) = save_instances_data(&instances_data).await {
                log::error!("Failed to save instances: {e}");
            }
        });

        Some(instance_id)
    }

    pub fn delete_instance(id: u32) -> bool {
        let mut instances = INSTANCES.write();
        let removed = instances.remove(&id).is_some();

        if removed {
            // Try to delete the instance directory
            let instance_dir = get_instance_directory(id);
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
            instance.name = new_name.chars().take(7).collect();
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

/// Get the directory for a specific instance.
pub fn get_instance_directory(instance_id: u32) -> PathBuf {
    get_launcher_dir()
        .unwrap_or_else(|_| PathBuf::from("DreamLauncher"))
        .join(format!("instances/instance_{instance_id}"))
}

/// Get the path to the instances' configuration file.
pub fn get_instances_config_path() -> PathBuf {
    get_launcher_dir()
        .unwrap_or_else(|_| PathBuf::from("DreamLauncher"))
        .join("launcher.json")
}

/// Create all necessary directories for an instance.
pub fn create_instance_directories(instance_id: u32) -> std::io::Result<PathBuf> {
    let instance_dir = get_instance_directory(instance_id);

    // Create the main instance directory
    fs::create_dir_all(&instance_dir)?;

    // Create subdirectories
    fs::create_dir_all(instance_dir.join("mods"))?;
    fs::create_dir_all(instance_dir.join("config"))?;
    fs::create_dir_all(instance_dir.join("saves"))?;
    fs::create_dir_all(instance_dir.join("resourcepacks"))?;
    fs::create_dir_all(instance_dir.join("shaderpacks"))?;
    fs::create_dir_all(instance_dir.join("crash-reports"))?;
    fs::create_dir_all(instance_dir.join("logs"))?;

    Ok(instance_dir)
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
        let mut instances = INSTANCES.write();
        instances.insert(1, Instance::new(1));
        *NEXT_ID.write() = 2;
        return Ok(());
    }

    let json = async_fs::read_to_string(config_path).await?;
    let data: InstancesData = serde_json::from_str(&json)?;

    // Update global state
    *INSTANCES.write() = data.instances;
    *NEXT_ID.write() = data.next_id;

    log::info!("Loaded {} instances from config", INSTANCES.read().len());

    // Debug
    for (id, instance) in INSTANCES.read().iter() {
        log::info!(
            "Loaded instance {}: name='{}', version='{}'",
            id,
            instance.name,
            instance.version
        );
    }

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
