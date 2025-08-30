use std::sync::{Arc, OnceLock};
use crate::backend::launcher::models::{VersionManifest};
use crate::backend::bridge::PythonMinecraftBridge;
use crate::utils::Result;
use crate::{log_info, log_error};

static VERSION_MANIFEST: OnceLock<Arc<VersionManifest>> = OnceLock::new();
static PYTHON_BRIDGE: OnceLock<PythonMinecraftBridge> = OnceLock::new();

pub async fn init_launcher() {
    if VERSION_MANIFEST.get().is_some() {
        return;
    }

    log_info!("Initializing Python Minecraft Bridge...");
    
    match PythonMinecraftBridge::new() {
        Ok(bridge) => {
            log_info!("Python Minecraft Bridge initialized successfully!");
            
            // Load real manifest from Mojang
            match bridge.get_version_manifest().await {
                Ok(manifest_json) => {
                    // Parse the manifest JSON into our VersionManifest struct
                    if let (Some(latest_obj), Some(versions_array)) = (
                        manifest_json.get("latest").and_then(|v| v.as_object()),
                        manifest_json.get("versions").and_then(|v| v.as_array())
                    ) {
                        let latest = crate::backend::launcher::models::LatestVersions {
                            release: latest_obj.get("release")
                                .and_then(|v| v.as_str())
                                .unwrap_or("1.21.4")
                                .to_string(),
                            snapshot: latest_obj.get("snapshot")
                                .and_then(|v| v.as_str())
                                .unwrap_or("24w51a")
                                .to_string(),
                        };
                        
                        let versions = versions_array.iter()
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
                        let _ = VERSION_MANIFEST.set(Arc::new(manifest));
                        log_info!("Version manifest loaded successfully with {} versions", versions_array.len());
                    } else {
                        log_error!("Invalid manifest format received from Python bridge");
                        // Fallback to minimal manifest
                        let manifest = VersionManifest {
                            latest: crate::backend::launcher::models::LatestVersions {
                                release: "1.21.4".to_string(),
                                snapshot: "24w51a".to_string(),
                            },
                            versions: vec![],
                        };
                        let _ = VERSION_MANIFEST.set(Arc::new(manifest));
                    }
                }
                Err(e) => {
                    log_error!("Failed to load version manifest: {}", e);
                    // Fallback to minimal manifest
                    let manifest = VersionManifest {
                        latest: crate::backend::launcher::models::LatestVersions {
                            release: "1.21.4".to_string(),
                            snapshot: "24w51a".to_string(),
                        },
                        versions: vec![],
                    };
                    let _ = VERSION_MANIFEST.set(Arc::new(manifest));
                }
            }
            
            let _ = PYTHON_BRIDGE.set(bridge);
        }
        Err(e) => {
            log_error!("Failed to initialize Python Bridge: {}", e);
        }
    }
}

pub fn get_version_manifest() -> Result<Arc<VersionManifest>> {
    VERSION_MANIFEST.get()
        .cloned()
        .ok_or_else(|| crate::simple_error!("Version manifest not initialized"))
}

pub fn get_python_bridge() -> Result<&'static PythonMinecraftBridge> {
    PYTHON_BRIDGE.get()
        .ok_or_else(|| crate::simple_error!("Python bridge not initialized"))
}