//! Launcher services.

use crate::backend::communicator::communicator::Communicator;
use crate::backend::services::VersionManifest;
use anyhow::Result;
use log::{error, info};
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock as AsyncRwLock;

static VERSION_MANIFEST: OnceLock<Arc<AsyncRwLock<VersionManifest>>> = OnceLock::new();

pub async fn init_launcher() -> Result<(), Box<dyn std::error::Error>> {
    if VERSION_MANIFEST.get().is_some() {
        return Ok(());
    }

    let archon = crate::get_archon().ok_or_else(|| anyhow::anyhow!("Archon not available"))?;
    let communicator = Communicator::new(archon).await?;
    match communicator.get_version_manifest().await {
        Ok(manifest) => {
            let _ = VERSION_MANIFEST.set(Arc::new(AsyncRwLock::new(manifest)));
            info!("Version manifest loaded successfully from backend");
        }
        Err(e) => {
            error!("Failed to load version manifest from backend: {e}");
            // Fallback to minimal manifest
            let manifest = VersionManifest {
                latest: crate::backend::services::launcher::LatestVersions {
                    release: "1.21.4".to_string(),
                    snapshot: "24w51a".to_string(),
                },
                versions: vec![],
            };
            let _ = VERSION_MANIFEST.set(Arc::new(AsyncRwLock::new(manifest)));
        }
    }

    Ok(())
}

pub async fn get_version_manifest() -> Result<VersionManifest> {
    let manifest_lock = VERSION_MANIFEST
        .get()
        .ok_or_else(|| anyhow::anyhow!("Version manifest not initialized"))?;

    let manifest = manifest_lock.read().await;
    Ok(manifest.clone())
}

pub async fn refresh_version_manifest() -> Result<()> {
    let archon = crate::get_archon().ok_or_else(|| anyhow::anyhow!("Archon not available"))?;
    let communicator = Communicator::new(archon).await?;
    match communicator.get_version_manifest().await {
        Ok(new_manifest) => {
            // Update the global manifest using AsyncRwLock
            let manifest_lock = VERSION_MANIFEST
                .get()
                .ok_or_else(|| anyhow::anyhow!("Version manifest not initialized"))?;

            let mut manifest = manifest_lock.write().await;
            *manifest = new_manifest.clone();

            info!(
                "Version manifest refreshed successfully. Latest release: {}, Latest snapshot: {}",
                new_manifest.latest.release, new_manifest.latest.snapshot
            );

            Ok(())
        }
        Err(e) => {
            error!("Failed to refresh version manifest: {e}");
            Err(anyhow::anyhow!("Failed to refresh version manifest: {e}"))
        }
    }
}
