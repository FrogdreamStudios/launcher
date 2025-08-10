//! Path utilities for Minecraft launcher directories.

use anyhow::{Result, anyhow};
use dirs;
use std::path::{Path, PathBuf};

/// Name of the main launcher directory.
const LAUNCHER_DIR: &str = "DreamLauncher";

/// Name of the Minecraft game directory.
const MINECRAFT_DIR: &str = ".minecraft";

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

/// Get the base launcher directory.
#[inline]
pub fn get_launcher_dir() -> Result<PathBuf> {
    let base_dir = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .ok_or_else(|| anyhow!("Could not determine data directory"))?;
    Ok(base_dir.join(LAUNCHER_DIR))
}

/// Get the Minecraft game directory.
pub fn get_game_dir(custom_path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = custom_path {
        return Ok(path);
    }
    let dir = match std::env::consts::OS {
        "windows" => dirs::data_dir()
            .ok_or_else(|| anyhow!("Could not determine AppData directory"))?
            .join(MINECRAFT_DIR),
        "macos" => dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?
            .join("Library/Application Support/minecraft"),
        _ => dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?
            .join(MINECRAFT_DIR),
    };
    Ok(dir)
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

/// Gets the assets directory within the game directory.
#[inline]
pub fn get_assets_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(ASSETS)
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

/// Gets the asset objects directory within the assets directory.
#[inline]
pub fn get_asset_objects_dir(game_dir: &Path) -> PathBuf {
    get_assets_dir(game_dir).join(OBJECTS)
}

/// Gets the asset indexes directory within the assets directory.
#[inline]
pub fn get_asset_indexes_dir(game_dir: &Path) -> PathBuf {
    get_assets_dir(game_dir).join(INDEXES)
}

/// Gets the path to a specific asset file based on its hash.
#[inline]
pub fn get_asset_path(game_dir: &Path, hash: &str) -> PathBuf {
    get_asset_objects_dir(game_dir).join(&hash[..2]).join(hash)
}

/// Gets the classpath separator for the current platform.
#[inline]
pub fn get_classpath_separator() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

/// Gets the full path to a library file.
#[inline]
pub fn get_library_path(game_dir: &Path, library_path: &str) -> PathBuf {
    get_libraries_dir(game_dir).join(library_path)
}

/// Ensure all necessary directories exist.
///
/// Creates all required directories for the launcher and Minecraft game
/// including versions, libraries, assets, logs, and launcher-specific dirs.
pub async fn ensure_directories(game_dir: &Path) -> Result<()> {
    let mut dirs = vec![
        game_dir.to_path_buf(),
        get_versions_dir(game_dir),
        get_libraries_dir(game_dir),
        get_assets_dir(game_dir),
        get_asset_objects_dir(game_dir),
        get_asset_indexes_dir(game_dir),
        get_logs_dir(game_dir),
    ];
    dirs.push(get_launcher_dir()?);
    dirs.push(get_java_dir()?);
    dirs.push(get_cache_dir()?);

    // Create all directories
    for dir in dirs {
        tokio::fs::create_dir_all(&dir).await?;
    }

    Ok(())
}
