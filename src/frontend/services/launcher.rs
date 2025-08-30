use crate::backend::bridge::PythonMinecraftBridge;
use crate::backend::launcher::models::VersionManifest;
use anyhow::Result;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock as AsyncRwLock;

static VERSION_MANIFEST: OnceLock<Arc<AsyncRwLock<VersionManifest>>> = OnceLock::new();
static PYTHON_BRIDGE: OnceLock<PythonMinecraftBridge> = OnceLock::new();

pub async fn init_launcher() {
    if VERSION_MANIFEST.get().is_some() {
        return;
    }

    log::info!("Initializing Python Minecraft Bridge...");

    match PythonMinecraftBridge::new() {
        Ok(bridge) => {
            log::info!("Python Minecraft Bridge initialized successfully!");

            // Load real manifest from Mojang
            match bridge.get_version_manifest().await {
                Ok(manifest_json) => {
                    // Parse the manifest JSON into our VersionManifest struct
                    if let (Some(latest_obj), Some(versions_array)) = (
                        manifest_json.get("latest").and_then(|v| v.as_object()),
                        manifest_json.get("versions").and_then(|v| v.as_array()),
                    ) {
                        let latest = crate::backend::launcher::models::LatestVersions {
                            release: latest_obj
                                .get("release")
                                .and_then(|v| v.as_str())
                                .unwrap_or("1.21.4")
                                .to_string(),
                            snapshot: latest_obj
                                .get("snapshot")
                                .and_then(|v| v.as_str())
                                .unwrap_or("24w51a")
                                .to_string(),
                        };

                        let versions = versions_array
                            .iter()
                            .filter_map(|v| {
                                let obj = v.as_object()?;
                                let id = obj.get("id")?.as_str()?.to_string();
                                let version_type = obj.get("type")?.as_str()?.to_string();
                                let url = obj.get("url")?.as_str()?.to_string();
                                let time = obj.get("time")?.as_str()?.to_string();
                                let release_time = obj.get("releaseTime")?.as_str()?.to_string();

                                Some(crate::backend::launcher::models::VersionInfo {
                                    id,
                                    version_type,
                                    url,
                                    time,
                                    release_time,
                                })
                            })
                            .collect();

                        let manifest = VersionManifest { latest, versions };
                        let _ = VERSION_MANIFEST.set(Arc::new(AsyncRwLock::new(manifest)));
                        log::info!(
                            "Version manifest loaded successfully with {} versions",
                            versions_array.len()
                        );
                    } else {
                        log::error!("Invalid manifest format received from Python bridge");
                        // Fallback to minimal manifest
                        let manifest = VersionManifest {
                            latest: crate::backend::launcher::models::LatestVersions {
                                release: "1.21.4".to_string(),
                                snapshot: "24w51a".to_string(),
                            },
                            versions: vec![],
                        };
                        let _ = VERSION_MANIFEST.set(Arc::new(AsyncRwLock::new(manifest)));
                    }
                }
                Err(e) => {
                    log::error!("Failed to load version manifest: {}", e);
                    // Fallback to minimal manifest
                    let manifest = VersionManifest {
                        latest: crate::backend::launcher::models::LatestVersions {
                            release: "1.21.4".to_string(),
                            snapshot: "24w51a".to_string(),
                        },
                        versions: vec![],
                    };
                    let _ = VERSION_MANIFEST.set(Arc::new(AsyncRwLock::new(manifest)));
                }
            }

            let _ = PYTHON_BRIDGE.set(bridge);
        }
        Err(e) => {
            log::error!("Failed to initialize Python Bridge: {}", e);
        }
    }
}

pub async fn get_version_manifest() -> Result<VersionManifest> {
    let manifest_lock = VERSION_MANIFEST
        .get()
        .ok_or_else(|| anyhow::anyhow!("Version manifest not initialized"))?;
    
    let manifest = manifest_lock.read().await;
    Ok(manifest.clone())
}

pub fn get_python_bridge() -> Result<&'static PythonMinecraftBridge> {
    PYTHON_BRIDGE
        .get()
        .ok_or_else(|| anyhow::anyhow!("Python bridge not initialized"))
}

pub async fn refresh_version_manifest() -> Result<()> {
    log::info!("Refreshing version manifest from Mojang servers...");
    
    let bridge = get_python_bridge()?;
    
    match bridge.get_version_manifest().await {
        Ok(manifest_json) => {
            // Parse the manifest JSON into our VersionManifest struct
            if let (Some(latest_obj), Some(versions_array)) = (
                manifest_json.get("latest").and_then(|v| v.as_object()),
                manifest_json.get("versions").and_then(|v| v.as_array()),
            ) {
                let latest = crate::backend::launcher::models::LatestVersions {
                    release: latest_obj
                        .get("release")
                        .and_then(|v| v.as_str())
                        .unwrap_or("1.21.4")
                        .to_string(),
                    snapshot: latest_obj
                        .get("snapshot")
                        .and_then(|v| v.as_str())
                        .unwrap_or("24w51a")
                        .to_string(),
                };

                let versions = versions_array
                    .iter()
                    .filter_map(|v| {
                        let obj = v.as_object()?;
                        let id = obj.get("id")?.as_str()?.to_string();
                        let version_type = obj.get("type")?.as_str()?.to_string();
                        let url = obj.get("url")?.as_str()?.to_string();
                        let time = obj.get("time")?.as_str()?.to_string();
                        let release_time = obj.get("releaseTime")?.as_str()?.to_string();

                        Some(crate::backend::launcher::models::VersionInfo {
                            id,
                            version_type,
                            url,
                            time,
                            release_time,
                        })
                    })
                    .collect();

                let new_manifest = VersionManifest { latest, versions };
                
                // Update the global manifest using AsyncRwLock
                let manifest_lock = VERSION_MANIFEST
                    .get()
                    .ok_or_else(|| anyhow::anyhow!("Version manifest not initialized"))?;
                
                let mut manifest = manifest_lock.write().await;
                *manifest = new_manifest.clone();
                
                log::info!(
                    "Version manifest refreshed successfully with {} versions. Latest release: {}, Latest snapshot: {}",
                    versions_array.len(),
                    new_manifest.latest.release,
                    new_manifest.latest.snapshot
                );
                
                Ok(())
            } else {
                Err(anyhow::anyhow!("Invalid manifest format received from Python bridge"))
            }
        }
        Err(e) => {
            log::error!("Failed to refresh version manifest: {e}");
            Err(anyhow::anyhow!("Failed to refresh version manifest: {e}"))
        }
    }
}
