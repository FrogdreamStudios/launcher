use std::path::{Path, PathBuf};
use tokio::fs;

/// Directory structure for Minecraft.
pub struct MinecraftDirectories {
    pub versions: PathBuf,
    pub libraries: PathBuf,
    pub assets: PathBuf,
    pub assets_indexes: PathBuf,
    pub assets_objects: PathBuf,
    pub cache: PathBuf,
}

impl MinecraftDirectories {
    pub fn new(root: PathBuf) -> Self {
        Self {
            versions: root.join("versions"),
            libraries: root.join("libraries"),
            assets: root.join("assets"),
            assets_indexes: root.join("assets/indexes"),
            assets_objects: root.join("assets/objects"),
            cache: root.join(".cache"),
        }
    }

    /// Create all necessary directories.
    pub async fn create_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        let dirs = vec![
            &self.versions,
            &self.libraries,
            &self.assets,
            &self.assets_indexes,
            &self.assets_objects,
            &self.cache,
        ];

        for dir in dirs {
            fs::create_dir_all(dir).await?;
        }

        Ok(())
    }

    /// Get a version-specific directory.
    pub fn get_version_dir(&self, version: &str) -> PathBuf {
        self.versions.join(version)
    }

    /// Get the client JAR path for a version.
    pub fn get_client_jar_path(&self, version: &str) -> PathBuf {
        self.get_version_dir(version)
            .join(format!("{version}.jar"))
    }
}

/// Directory manager utility.
pub struct DirectoryManager;

impl DirectoryManager {
    /// Create directory if it doesn't exist.
    pub async fn ensure_dir(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if !path.exists() {
            fs::create_dir_all(path).await?;
        }
        Ok(())
    }

    /// Create parent directory for a file path.
    pub async fn ensure_parent_dir(file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = file_path.parent() {
            Self::ensure_dir(parent).await?;
        }
        Ok(())
    }
}
