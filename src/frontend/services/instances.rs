//! Instance management service.

use crate::backend::services::Instance;

use crate::backend::communicator::communicator::Communicator;
use dioxus::prelude::*;
use log::{error, info, warn};
use std::collections::HashMap;

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
                let Some(archon) = crate::get_archon() else {
                    error!("Archon not available");
                    return;
                };
                match Communicator::new(archon.clone()).await {
                    Ok(communicator) => {
                        match communicator.get_instances().await {
                            Ok(instances_data) => {
                                let mut instances_map = HashMap::new();
                                let mut max_id = 0;
                                for instance in &instances_data {
                                    if instance.id > max_id {
                                        max_id = instance.id;
                                    }
                                    instances_map.insert(instance.id, instance.clone());
                                }
                                *INSTANCES.write() = instances_map;
                                *NEXT_ID.write() = max_id + 1;
                                info!("Loaded {} instances from backend", instances_data.len());
                            }
                            Err(e) => {
                                error!("Failed to load instances: {e}, creating default instance");
                                // Create default instance if loading fails
                                if let Ok(instance_id_opt) =
                                    communicator.create_instance("1.21.8").await
                                    && let Some(instance_id) = instance_id_opt
                                {
                                    let mut instances = INSTANCES.write();
                                    let new_instance = Instance::new_with_version(
                                        instance_id,
                                        "1.21.8".to_string(),
                                    );
                                    instances.insert(instance_id, new_instance);
                                    *NEXT_ID.write() = instance_id + 1;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to initialize communicator: {e}");
                    }
                }
                *INSTANCES_LOADED.write() = true;
            });
        }
    }

    #[must_use]
    pub fn create_instance_with_version(version: &str) -> Option<u32> {
        info!("create_instance_with_version called with version: {version}");

        // Check if we can create more instances (max 14)
        if !Self::can_create_instance() {
            warn!("Cannot create more instances, limit reached");
            return None;
        }

        let version = version.to_string();
        spawn(async move {
            let Some(archon) = crate::get_archon() else {
                error!("Archon not available");
                return;
            };
            match Communicator::new(archon).await {
                Ok(communicator) => {
                    match communicator.create_instance(&version).await {
                        Ok(instance_id_opt) => {
                            if let Some(instance_id) = instance_id_opt {
                                let new_instance =
                                    Instance::new_with_version(instance_id, version.clone());
                                info!("Created instance {instance_id} with version: {version}");

                                // Update local state
                                let mut instances = INSTANCES.write();
                                instances.insert(instance_id, new_instance);
                                *NEXT_ID.write() = instance_id + 1;
                            } else {
                                error!("Failed to create instance: no ID returned");
                            }
                        }
                        Err(e) => {
                            error!("Failed to create instance: {e}");
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to initialize communicator: {e}");
                }
            }
        });

        // Return a placeholder ID for now - in a real implementation, this should be async
        let current_id = *NEXT_ID.read();
        Some(current_id)
    }

    #[must_use]
    pub fn delete_instance(id: u32) -> bool {
        let mut instances = INSTANCES.write();
        let removed = instances.remove(&id).is_some();

        if removed {
            drop(instances); // Release the write lock
            spawn(async move {
                let archon = if let Some(archon) = crate::get_archon() {
                    archon
                } else {
                    error!("Archon not available");
                    return;
                };
                match Communicator::new(archon).await {
                    Ok(communicator) => {
                        if let Err(e) = communicator.delete_instance(id).await {
                            error!("Failed to delete instance {id} from backend: {e}");
                        } else {
                            info!("Deleted instance {id} from backend");
                        }
                    }
                    Err(e) => {
                        error!("Failed to initialize communicator: {e}");
                    }
                }
            });
        }

        removed
    }

    #[must_use]
    pub fn rename_instance(id: u32, new_name: &str) -> bool {
        let mut instances = INSTANCES.write();
        let renamed = if let Some(instance) = instances.get_mut(&id) {
            instance.name = new_name.chars().take(8).collect();
            true
        } else {
            false
        };

        if renamed {
            let new_name = new_name.to_string();
            drop(instances); // Release the write lock
            spawn(async move {
                let Some(archon) = crate::get_archon() else {
                    error!("Archon not available");
                    return;
                };
                match Communicator::new(archon).await {
                    Ok(communicator) => {
                        if let Err(e) = communicator.rename_instance(id, &new_name).await {
                            error!("Failed to rename instance {id} in backend: {e}");
                        } else {
                            info!("Renamed instance {id} in backend");
                        }
                    }
                    Err(e) => {
                        error!("Failed to initialize communicator: {e}");
                    }
                }
            });
        }

        renamed
    }

    pub fn toggle_debug_mode() {
        let current = *DEBUG_MODE.read();
        *DEBUG_MODE.write() = !current;
    }

    #[must_use]
    pub fn is_debug_mode() -> bool {
        *DEBUG_MODE.read()
    }

    #[must_use]
    pub fn can_create_instance() -> bool {
        INSTANCES.read().len() < 14
    }

    #[must_use]
    pub fn get_instances_sorted() -> Vec<Instance> {
        let instances = INSTANCES.read();
        let mut sorted: Vec<Instance> = instances.values().cloned().collect();
        sorted.sort_by_key(|i| i.id);
        sorted
    }
}

/// Open instance folder in system file explorer.
pub fn open_instance_folder(instance_id: u32) {
    spawn(async move {
        let Some(archon) = crate::get_archon() else {
            error!("Archon not available");
            return;
        };
        match Communicator::new(archon).await {
            Ok(communicator) => {
                if let Err(e) = communicator.open_instance_folder(instance_id).await {
                    error!("Failed to open instance {instance_id} folder: {e}");
                } else {
                    info!("Opened instance {instance_id} folder");
                }
            }
            Err(e) => {
                error!("Failed to initialize communicator: {e}");
            }
        }
    });
}
