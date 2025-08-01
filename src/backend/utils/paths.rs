use anyhow::Result;
use dirs;
use sha1::{Digest, Sha1};
use std::path::PathBuf;

/// Get the base launcher directory.
pub fn get_launcher_dir() -> Result<PathBuf> {
    let base_dir = dirs::data_local_dir()
        .or_else(|| dirs::data_dir())
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;

    Ok(base_dir.join("DreamLauncher"))
}

/// Get the Minecraft game directory.
pub fn get_game_dir(custom_path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = custom_path {
        return Ok(path);
    }

    let default_dir = match std::env::consts::OS {
        "windows" => dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine AppData directory"))?
            .join(".minecraft"),
        "macos" => dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
            .join("Library")
            .join("Application Support")
            .join("minecraft"),
        _ => dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
            .join(".minecraft"),
    };

    Ok(default_dir)
}

/// Get the versions directory.
pub fn get_versions_dir(game_dir: &PathBuf) -> PathBuf {
    game_dir.join("versions")
}

/// Get the libraries directory.
pub fn get_libraries_dir(game_dir: &PathBuf) -> PathBuf {
    game_dir.join("libraries")
}

/// Get the assets directory.
pub fn get_assets_dir(game_dir: &PathBuf) -> PathBuf {
    game_dir.join("assets")
}

/// Get the Java runtimes directory.
pub fn get_java_dir() -> Result<PathBuf> {
    Ok(get_launcher_dir()?.join("java"))
}

/// Get the launcher cache directory.
pub fn get_cache_dir() -> Result<PathBuf> {
    Ok(get_launcher_dir()?.join("cache"))
}

/// Get the logs directory.
pub fn get_logs_dir(game_dir: &PathBuf) -> PathBuf {
    game_dir.join("logs")
}

/// Get the natives directory for a specific version.
pub fn get_natives_dir(game_dir: &PathBuf, version: &str) -> PathBuf {
    get_versions_dir(game_dir).join(version).join("natives")
}

/// Get the version jar file path.
pub fn get_version_jar_path(game_dir: &PathBuf, version: &str) -> PathBuf {
    get_versions_dir(game_dir)
        .join(version)
        .join(format!("{version}.jar"))
}

/// Get the version JSON file path.
pub fn get_version_json_path(game_dir: &PathBuf, version: &str) -> PathBuf {
    get_versions_dir(game_dir)
        .join(version)
        .join(format!("{version}.json"))
}

/// Get the asset objects directory.
pub fn get_asset_objects_dir(game_dir: &PathBuf) -> PathBuf {
    get_assets_dir(game_dir).join("objects")
}

/// Get the asset indexes directory.
pub fn get_asset_indexes_dir(game_dir: &PathBuf) -> PathBuf {
    get_assets_dir(game_dir).join("indexes")
}

/// Convert asset hash to path (first 2 chars as subdirectory).
pub fn get_asset_path(game_dir: &PathBuf, hash: &str) -> PathBuf {
    let subdir = &hash[..2];
    get_asset_objects_dir(game_dir).join(subdir).join(hash)
}

/// Ensure all necessary directories exist.
pub async fn ensure_directories(game_dir: &PathBuf) -> Result<()> {
    let directories = vec![
        game_dir.clone(),
        get_versions_dir(game_dir),
        get_libraries_dir(game_dir),
        get_assets_dir(game_dir),
        get_asset_objects_dir(game_dir),
        get_asset_indexes_dir(game_dir),
        get_logs_dir(game_dir),
        get_launcher_dir()?,
        get_java_dir()?,
        get_cache_dir()?,
    ];

    for dir in directories {
        tokio::fs::create_dir_all(&dir).await?;
    }

    Ok(())
}

/// Get platform-specific classpath separator.
pub fn get_classpath_separator() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

/// Get the full library path in the libraries directory.
pub fn get_library_path(game_dir: &PathBuf, library_path: &str) -> PathBuf {
    get_libraries_dir(game_dir).join(library_path)
}

/// Check if a file exists and has the correct size and hash.
pub async fn verify_file(
    path: &PathBuf,
    expected_size: Option<u64>,
    expected_sha1: Option<&str>,
) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let metadata = tokio::fs::metadata(path).await?;

    // Check size if provided
    if let Some(size) = expected_size {
        if metadata.len() != size {
            return Ok(false);
        }
    }

    // Check SHA1 hash if provided
    if let Some(expected_hash) = expected_sha1 {
        let content = tokio::fs::read(path).await?;
        let mut hasher = Sha1::new();
        hasher.update(&content);
        let computed_hash = hasher.finalize();
        let computed_hex = hex::encode(computed_hash);

        if computed_hex != expected_hash {
            return Ok(false);
        }
    }

    Ok(true)
}
