use std::sync::{Arc, OnceLock};
use crate::backend::launcher::models::VersionManifest;
use crate::utils::Result;
use crate::{log_info, log_error};

static VERSION_MANIFEST: OnceLock<Arc<VersionManifest>> = OnceLock::new();

pub async fn init_launcher() {
    if VERSION_MANIFEST.get().is_some() {
        return;
    }

    log_info!("Initializing Minecraft Launcher (manifest)...");
    let downloader = Arc::new(crate::backend::launcher::downloader::HttpDownloader::new().unwrap());
    let cache_dir = crate::backend::utils::launcher::paths::get_cache_dir().unwrap();
    let mut version_manager = crate::backend::launcher::versions::VersionManager::new(
        downloader.clone(),
        cache_dir.clone(),
        None,
    );

    match version_manager.load_or_update_manifest().await {
        Ok(_) => {
            if let Some(manifest) = version_manager.get_manifest() {
                let _ = VERSION_MANIFEST.set(manifest.clone());
                log_info!("Minecraft Version Manifest initialized successfully");
            } else {
                log_error!("Failed to get manifest after loading/updating");
            }
        }
        Err(e) => {
            log_error!("Failed to initialize Minecraft Version Manifest: {e}");
        }
    }
}

pub fn get_version_manifest() -> Result<Arc<VersionManifest>> {
    VERSION_MANIFEST
        .get()
        .cloned()
        .ok_or_else(|| crate::simple_error!("Version Manifest not initialized"))
}