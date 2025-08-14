use crate::simple_error;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    pub id: u32,
    pub name: String,
    pub color: String,
    pub level: u32,
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
    pub fn initialize(&self) {
        if *INSTANCES_LOADED.read() {
            spawn(async move {
                if let Err(e) = load_instances().await {
                    println!("Failed to load instances: {e}, creating default instance");
                    // Create default instance if loading fails
                    let mut instances = INSTANCES.write();
                    instances.insert(1, Instance::new(1));
                    *NEXT_ID.write() = 2;
                }
                *INSTANCES_LOADED.write() = true;
            });
        }
    }

    pub fn create_instance(&self) -> Option<u32> {
        let mut instances = INSTANCES.write();
        let current_id = *NEXT_ID.read();

        // Check if we can create more instances (max 14)
        if instances.len() >= 14 {
            return None;
        }

        let new_instance = Instance::new(current_id);
        let instance_id = new_instance.id;
        instances.insert(instance_id, new_instance);

        // Create instance directories
        if let Err(e) = create_instance_directories(instance_id) {
            println!("Warning: Failed to create directories for instance {instance_id}: {e}");
        } else {
            println!("Created directories for instance {instance_id}");
        }

        // Update next_id
        *NEXT_ID.write() = current_id + 1;

        // Save instances to disk
        let instances_data = InstancesData {
            instances: instances.clone(),
            next_id: current_id + 1,
        };
        drop(instances); // Release the write lock
        spawn(async move {
            if let Err(e) = save_instances_data(&instances_data).await {
                println!("Failed to save instances: {e}");
            }
        });

        Some(instance_id)
    }

    pub fn delete_instance(&self, id: u32) -> bool {
        let mut instances = INSTANCES.write();
        let removed = instances.remove(&id).is_some();

        if removed {
            // Try to delete the instance directory
            let instance_dir = get_instance_directory(id);
            if instance_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&instance_dir) {
                    println!("Warning: Failed to delete instance {id} directory: {e}");
                } else {
                    println!("Deleted instance {id} directory: {instance_dir:?}");
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
                    println!("Failed to save instances: {e}");
                }
            });
        }

        removed
    }

    pub fn rename_instance(&self, id: u32, new_name: &str) -> bool {
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
                    println!("Failed to save instances: {e}");
                }
            });
        }

        renamed
    }

    pub fn toggle_debug_mode(&self) {
        let current = *DEBUG_MODE.read();
        *DEBUG_MODE.write() = !current;
    }

    pub fn is_debug_mode(&self) -> bool {
        *DEBUG_MODE.read()
    }

    pub fn can_create_instance(&self) -> bool {
        INSTANCES.read().len() < 14
    }

    pub fn get_instances_sorted(&self) -> Vec<Instance> {
        let instances = INSTANCES.read();
        let mut sorted: Vec<Instance> = instances.values().cloned().collect();
        sorted.sort_by_key(|i| i.id);
        sorted
    }
}

pub fn use_instance_manager() -> InstanceManager {
    let manager = InstanceManager;
    manager.initialize();
    manager
}

/// Get the base `DreamLauncher` directory.
pub fn get_base_directory() -> PathBuf {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/Users/unknown".to_string());
    Path::new(&home_dir).join("Library/Application Support/DreamLauncher")
}

/// Get the directory for a specific instance.
pub fn get_instance_directory(instance_id: u32) -> PathBuf {
    get_base_directory().join(format!("instances/instance_{instance_id}"))
}

/// Get the path to the instances' configuration file.
pub fn get_instances_config_path() -> PathBuf {
    get_base_directory().join("instances.json")
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
async fn save_instances_data(data: &InstancesData) -> crate::utils::Result<()> {
    let config_path = get_instances_config_path();

    // Ensure the parent directory exists
    if let Some(parent) = config_path.parent() {
        async_fs::create_dir_all(parent).await?;
    }

    let json = serde_json::to_string_pretty(data)?;
    async_fs::write(config_path, json).await?;

    println!("Instances saved successfully");
    Ok(())
}

/// Load instances from disk.
async fn load_instances() -> crate::utils::Result<()> {
    let config_path = get_instances_config_path();

    if !config_path.exists() {
        println!("No instances config found, creating default instance");
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

    println!("Loaded {} instances from config", INSTANCES.read().len());
    Ok(())
}

/// Open instance folder in system file explorer.
pub async fn open_instance_folder(instance_id: u32) -> crate::utils::Result<()> {
    use std::process::Command;

    let instance_dir = get_instance_directory(instance_id);

    // Ensure directory exists
    if !instance_dir.exists() {
        create_instance_directories(instance_id)?;
    }

    // Open in Finder on macOS
    let output = Command::new("open").arg(&instance_dir).output()?;

    if !output.status.success() {
        return Err(simple_error!("Failed to open instance folder"));
    }

    println!("Opened instance {instance_id} folder: {instance_dir:?}");
    Ok(())
}
