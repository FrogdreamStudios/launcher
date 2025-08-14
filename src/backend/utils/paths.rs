//! Path utilities for Minecraft launcher directories.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use dirs;

/// Name of the main launcher directory.
const LAUNCHER_DIR: &str = "DreamLauncher";

/// Subdirectory for Minecraft versions.
const VERSIONS: &str = "versions";

/// Subdirectory for library files.
const LIBRARIES: &str = "libraries";

/// Subdirectory for game assets.
const ASSETS: &str = "assets";

/// Subdirectory for Java installations.
const JAVA: &str = "java";

/// Subdirectory for cached files.
const CACHE: &str = "cache";

/// Subdirectory for log files.
const LOGS: &str = "logs";

/// Subdirectory for asset objects.
const OBJECTS: &str = "objects";

/// Subdirectory for asset indexes.
const INDEXES: &str = "indexes";

/// Subdirectory for native libraries.
const NATIVES: &str = "natives";

/// Subdirectory for instances.
const INSTANCES: &str = "instances";

/// Get the base launcher directory (`DreamLauncher`).
#[inline]
pub fn get_launcher_dir() -> Result<PathBuf> {
    let base_dir = match std::env::consts::OS {
        "windows" => {
            dirs::data_dir().ok_or_else(|| anyhow!("Could not determine AppData directory"))?
        }
        "macos" => dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?
            .join("Library/Application Support"),
        _ => dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?,
    };
    Ok(base_dir.join(LAUNCHER_DIR))
}

/// Get the Minecraft game directory for a specific instance.
/// If `custom_path` is provided, it will be used instead of the instance-specific path.
/// If `instance_id` is None, it returns the shared `DreamLauncher` directory (for assets, etc.).
pub fn get_game_dir(custom_path: Option<PathBuf>, instance_id: Option<u32>) -> Result<PathBuf> {
    if let Some(path) = custom_path {
        return Ok(path);
    }

    let launcher_dir = get_launcher_dir()?;

    match instance_id {
        Some(id) => Ok(launcher_dir.join(INSTANCES).join(format!("instance_{id}"))),
        None => Ok(launcher_dir), // For shared resources like assets
    }
}

/// Gets the versions directory within the game directory.
#[inline]
pub fn get_versions_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(VERSIONS)
}

/// Gets the libraries directory within the game directory.
#[inline]
pub fn get_libraries_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(LIBRARIES)
}

/// Gets the shared assets directory (always in the main `DreamLauncher` directory).
#[inline]
pub fn get_assets_dir() -> Result<PathBuf> {
    Ok(get_launcher_dir()?.join(ASSETS))
}

/// Gets the Java installations directory within the launcher directory.
#[inline]
pub fn get_java_dir() -> Result<PathBuf> {
    Ok(get_launcher_dir()?.join(JAVA))
}

/// Gets the cache directory within the launcher directory.
#[inline]
pub fn get_cache_dir() -> Result<PathBuf> {
    Ok(get_launcher_dir()?.join(CACHE))
}

/// Gets the logs directory within the game directory.
#[inline]
pub fn get_logs_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(LOGS)
}

/// Gets the shared logs directory in the main `DreamLauncher` directory.
#[inline]
pub fn get_shared_logs_dir() -> Result<PathBuf> {
    Ok(get_launcher_dir()?.join(LOGS))
}

/// Gets the natives directory for a specific Minecraft version.
#[inline]
pub fn get_natives_dir(game_dir: &Path, version: &str) -> PathBuf {
    get_versions_dir(game_dir).join(version).join(NATIVES)
}

/// Gets the JAR file path for a specific Minecraft version.
#[inline]
pub fn get_version_jar_path(game_dir: &Path, version: &str) -> PathBuf {
    get_versions_dir(game_dir)
        .join(version)
        .join(format!("{version}.jar"))
}

/// Gets the JSON file path for a specific Minecraft version.
#[inline]
pub fn get_version_json_path(game_dir: &Path, version: &str) -> PathBuf {
    get_versions_dir(game_dir)
        .join(version)
        .join(format!("{version}.json"))
}

/// Gets the asset objects directory within the shared assets directory.
#[inline]
pub fn get_asset_objects_dir() -> Result<PathBuf> {
    Ok(get_assets_dir()?.join(OBJECTS))
}

/// Gets the asset indexes directory within the shared assets directory.
#[inline]
pub fn get_asset_indexes_dir() -> Result<PathBuf> {
    Ok(get_assets_dir()?.join(INDEXES))
}

/// Gets the path to a specific asset file based on its hash.
#[inline]
pub fn get_asset_path(hash: &str) -> Result<PathBuf> {
    Ok(get_asset_objects_dir()?.join(&hash[..2]).join(hash))
}

/// Gets the classpath separator for the current platform.
#[inline]
pub const fn get_classpath_separator() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

/// Gets the full path to a library file.
#[inline]
pub fn get_library_path(game_dir: &Path, library_path: &str) -> PathBuf {
    get_libraries_dir(game_dir).join(library_path)
}

/// Gets the instances directory.
#[inline]
pub fn get_instances_dir() -> Result<PathBuf> {
    Ok(get_launcher_dir()?.join(INSTANCES))
}

/// Gets the directory for a specific instance.
#[inline]
pub fn get_instance_dir(instance_id: u32) -> Result<PathBuf> {
    Ok(get_instances_dir()?.join(format!("instance_{instance_id}")))
}

/// Ensure all necessary directories exist for the launcher.
///
/// Creates all required directories for the launcher, including shared assets,
/// Java, cache, and logs directories.
pub async fn ensure_launcher_directories() -> Result<()> {
    let dirs = vec![
        get_launcher_dir()?,
        get_assets_dir()?,
        get_asset_objects_dir()?,
        get_asset_indexes_dir()?,
        get_java_dir()?,
        get_cache_dir()?,
        get_shared_logs_dir()?,
        get_instances_dir()?,
    ];

    // Create all directories
    for dir in dirs {
        tokio::fs::create_dir_all(&dir).await?;
    }

    Ok(())
}

/// Ensure all necessary directories exist for a specific instance.
///
/// Creates all required directories for a specific instance including
/// versions, libraries, logs, and other game-specific directories.
pub async fn ensure_instance_directories(instance_id: u32) -> Result<()> {
    let instance_dir = get_instance_dir(instance_id)?;

    let dirs = vec![
        instance_dir.clone(),
        get_versions_dir(&instance_dir),
        get_libraries_dir(&instance_dir),
        get_logs_dir(&instance_dir),
        instance_dir.join("mods"),
        instance_dir.join("config"),
        instance_dir.join("saves"),
        instance_dir.join("resourcepacks"),
        instance_dir.join("shaderpacks"),
        instance_dir.join("crash-reports"),
    ];

    // Create all directories
    for dir in dirs {
        tokio::fs::create_dir_all(&dir).await?;
    }

    Ok(())
}

/// Ensure all necessary directories exist for both launcher and a specific instance.
///
/// This is a convenience function that combines `ensure_launcher_directories`
/// and `ensure_instance_directories`.
pub async fn ensure_directories(instance_id: Option<u32>) -> Result<()> {
    // Always ensure launcher directories exist
    ensure_launcher_directories().await?;

    // If instance_id is provided, also ensure instance directories exist
    if let Some(id) = instance_id {
        ensure_instance_directories(id).await?;
    }

    Ok(())
}
