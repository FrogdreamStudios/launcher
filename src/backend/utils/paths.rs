use anyhow::{Result, anyhow};
use dirs;
use std::path::{Path, PathBuf};

const LAUNCHER_DIR: &str = "DreamLauncher";
const MINECRAFT_DIR: &str = ".minecraft";
const VERSIONS: &str = "versions";
const LIBRARIES: &str = "libraries";
const ASSETS: &str = "assets";
const JAVA: &str = "java";
const CACHE: &str = "cache";
const LOGS: &str = "logs";
const OBJECTS: &str = "objects";
const INDEXES: &str = "indexes";
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

#[inline]
pub fn get_versions_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(VERSIONS)
}
#[inline]
pub fn get_libraries_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(LIBRARIES)
}
#[inline]
pub fn get_assets_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(ASSETS)
}
#[inline]
pub fn get_java_dir() -> Result<PathBuf> {
    Ok(get_launcher_dir()?.join(JAVA))
}
#[inline]
pub fn get_cache_dir() -> Result<PathBuf> {
    Ok(get_launcher_dir()?.join(CACHE))
}
#[inline]
pub fn get_logs_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(LOGS)
}
#[inline]
pub fn get_natives_dir(game_dir: &Path, version: &str) -> PathBuf {
    get_versions_dir(game_dir).join(version).join(NATIVES)
}
#[inline]
pub fn get_version_jar_path(game_dir: &Path, version: &str) -> PathBuf {
    get_versions_dir(game_dir)
        .join(version)
        .join(format!("{version}.jar"))
}
#[inline]
pub fn get_version_json_path(game_dir: &Path, version: &str) -> PathBuf {
    get_versions_dir(game_dir)
        .join(version)
        .join(format!("{version}.json"))
}
#[inline]
pub fn get_asset_objects_dir(game_dir: &Path) -> PathBuf {
    get_assets_dir(game_dir).join(OBJECTS)
}
#[inline]
pub fn get_asset_indexes_dir(game_dir: &Path) -> PathBuf {
    get_assets_dir(game_dir).join(INDEXES)
}
#[inline]
pub fn get_asset_path(game_dir: &Path, hash: &str) -> PathBuf {
    get_asset_objects_dir(game_dir).join(&hash[..2]).join(hash)
}
#[inline]
pub fn get_classpath_separator() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}
#[inline]
pub fn get_library_path(game_dir: &Path, library_path: &str) -> PathBuf {
    get_libraries_dir(game_dir).join(library_path)
}

/// Ensure all necessary directories exist.
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

    for dir in dirs {
        tokio::fs::create_dir_all(&dir).await?;
    }

    Ok(())
}
