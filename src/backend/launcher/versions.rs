//! Version management module for handling Minecraft version manifest and version information.

use super::{
    downloader::HttpDownloader,
    models::{VersionDetails, VersionInfo, VersionManifest},
};
use crate::utils::Result;
use crate::{log_info, log_warn, simple_error};
use std::path::Path;
use std::{path::PathBuf, sync::Arc};

/// Version manager for handling Minecraft version operations.
pub struct VersionManager {
    downloader: Arc<HttpDownloader>,
    cache_dir: PathBuf,
    manifest: Option<Arc<VersionManifest>>,
}

impl VersionManager {
    /// Creates a new version manager.
    pub fn new(
        downloader: Arc<HttpDownloader>,
        cache_dir: PathBuf,
        manifest: Option<Arc<VersionManifest>>,
    ) -> Self {
        Self {
            downloader,
            cache_dir,
            manifest,
        }
    }

    /// Loads the manifest from the cache or updates it from the network if the cache is stale or missing.
    pub async fn load_or_update_manifest(&mut self) -> Result<()> {
        if self.manifest.is_some() {
            return Ok(());
        }

        // Try to load from a cache
        if self.load_cached_manifest().await.is_ok() {
            return Ok(());
        }

        // If the cache load failed, try to update from network
        log_warn!("Failed to load cached manifest, attempting to update from network");
        self.update_manifest().await?;
        Ok(())
    }

    /// Gets available versions from the manifest.
    pub fn get_available_versions(&self) -> Result<&[VersionInfo]> {
        let manifest = self
            .manifest
            .as_ref()
            .ok_or_else(|| simple_error!("Version manifest not loaded"))?;

        Ok(&manifest.versions)
    }

    /// Updates the version manifest by downloading it from Mojang.
    pub async fn update_manifest(&mut self) -> Result<()> {
        log_info!("Fetching version manifest from Mojang...");

        let manifest: VersionManifest = self
            .downloader
            .get_json(VersionManifest::MANIFEST_URL)
            .await?;

        // Cache the manifest
        let manifest_path = self.cache_dir.join("version_manifest_v2.json");

        // Ensure the parent directory exists before writing the file
        if let Some(parent) = manifest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        log_info!("Attempting to write manifest to: {}", manifest_path.display());
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        tokio::fs::write(&manifest_path, manifest_json).await?;

        self.manifest = Some(Arc::new(manifest));
        log_info!("Version manifest updated successfully");

        Ok(())
    }

    /// Loads cached manifest from disk.
    pub async fn load_cached_manifest(&mut self) -> Result<()> {
        let manifest_path = self.cache_dir.join("version_manifest_v2.json");

        if !manifest_path.exists() {
            return Err(simple_error!("No cached manifest found"));
        }

        let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
        let manifest: VersionManifest = serde_json::from_str(&manifest_content)?;

        self.manifest = Some(Arc::new(manifest));
        log_info!("Loaded cached version manifest");

        Ok(())
    }

    /// Gets version information for a specific version ID.
    pub fn get_version_info(&self, version_id: &str) -> Result<&VersionInfo> {
        let versions = self.get_available_versions()?;

        versions
            .iter()
            .find(|v| v.id == version_id)
            .ok_or_else(|| simple_error!("Version {version_id} not found"))
    }

    /// Downloads and parses version details for a specific version.
    pub async fn get_version_details(&self, version_info: &VersionInfo) -> Result<VersionDetails> {
        let version_details: VersionDetails = self.downloader.get_json(&version_info.url).await?;

        Ok(version_details)
    }

    /// Checks if a version is ready for offline use.
    pub fn is_version_ready_offline(&self, game_dir: &Path, version_id: &str) -> Result<bool> {
        let version_dir = game_dir.join("versions").join(version_id);
        let jar_file = version_dir.join(format!("{version_id}.jar"));
        let json_file = version_dir.join(format!("{version_id}.json"));

        Ok(version_dir.exists() && jar_file.exists() && json_file.exists())
    }

    /// Gets the loaded manifest.
    pub const fn get_manifest(&self) -> Option<&Arc<VersionManifest>> {
        self.manifest.as_ref()
    }
}
