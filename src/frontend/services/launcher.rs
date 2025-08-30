//! Launcher services.

use crate::backend::launcher::bridge::PythonMinecraftBridge;
use crate::backend::launcher::models::VersionManifest;
use crate::backend::utils::paths::get_launcher_dir;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::fs;
use tokio::sync::RwLock as AsyncRwLock;

/// Get the path to the cached version manifest file.
fn get_manifest_cache_path() -> PathBuf {
    get_launcher_dir()
        .unwrap_or_else(|_| PathBuf::from("DreamLauncher"))
        .join("version_manifest.json")
}

/// Fetch versions manifest directly from Mojang API.
async fn fetch_version_manifest_from_mojang() -> Result<serde_json::Value> {
    let url = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
    let response = reqwest::get(url).await?;
    let manifest = response.json::<serde_json::Value>().await?;
    Ok(manifest)
}

/// Save version manifest to the cache file.
async fn save_manifest_to_cache(manifest: &serde_json::Value) -> Result<()> {
    let cache_path = get_manifest_cache_path();

    // Create a parent directory if it doesn't exist
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let manifest_json = serde_json::to_string_pretty(manifest)?;
    fs::write(&cache_path, manifest_json).await?;
    Ok(())
}

/// Load version manifest from the cache file.
async fn load_manifest_from_cache() -> Result<serde_json::Value> {
    let cache_path = get_manifest_cache_path();

    if !cache_path.exists() {
        return Err(anyhow::anyhow!("Cache file does not exist"));
    }

    let manifest_json = fs::read_to_string(&cache_path).await?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_json)?;
    log::info!("Version manifest loaded from cache: {cache_path:?}");
    Ok(manifest)
}

static VERSION_MANIFEST: OnceLock<Arc<AsyncRwLock<VersionManifest>>> = OnceLock::new();
static PYTHON_BRIDGE: OnceLock<PythonMinecraftBridge> = OnceLock::new();

pub async fn init_launcher() {
    if VERSION_MANIFEST.get().is_some() {
        return;
    }

    match PythonMinecraftBridge::new() {
        Ok(bridge) => {
            log::info!("Python Minecraft bridge initialized successfully");

            // Try to load manifest from the internet first, fallback to cache
            let manifest_json = match fetch_version_manifest_from_mojang().await {
                Ok(manifest) => {
                    log::info!("Version manifest loaded from Mojang servers");
                    // Save to cache for offline use
                    if let Err(e) = save_manifest_to_cache(&manifest).await {
                        log::warn!("Failed to cache version manifest: {e}");
                    }
                    Some(manifest)
                }
                Err(e) => {
                    log::warn!("Failed to load version manifest from internet: {e}");
                    // Try to load from a cache
                    match load_manifest_from_cache().await {
                        Ok(cached_manifest) => {
                            log::info!("Using cached version manifest");
                            Some(cached_manifest)
                        }
                        Err(cache_err) => {
                            log::error!("Failed to load cached manifest: {cache_err}");
                            None
                        }
                    }
                }
            };

            if let Some(manifest_json) = manifest_json {
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
                    log::error!("Invalid manifest format");
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
            } else {
                log::error!("No version manifest available (neither online nor cached)");
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

            let _ = PYTHON_BRIDGE.set(bridge);
        }
        Err(e) => {
            log::error!("Failed to initialize Python bridge: {e}");
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
    match fetch_version_manifest_from_mojang().await {
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
                Err(anyhow::anyhow!(
                    "Invalid manifest format received from Python bridge"
                ))
            }
        }
        Err(e) => {
            log::error!("Failed to refresh version manifest: {e}");
            Err(anyhow::anyhow!("Failed to refresh version manifest: {e}"))
        }
    }
}
