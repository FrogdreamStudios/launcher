//! Path utilities.

use std::path::PathBuf;

use anyhow::Result;

/// Name of the main launcher directory.
const LAUNCHER_DIR: &str = "DreamLauncher";

/// Subdirectory for instances.
const INSTANCES: &str = "instances";

/// Get the base launcher directory (`DreamLauncher`).
#[inline]
pub fn get_launcher_dir() -> Result<PathBuf> {
    let base_dir = match std::env::consts::OS {
        "windows" => std::env::var("APPDATA")
            .ok()
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("Could not determine AppData directory"))?,
        "macos" => std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join("Library/Application Support"))
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?,
        _ => std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?,
    };
    Ok(base_dir.join(LAUNCHER_DIR))
}

/// Get the Minecraft game directory for a specific instance.
pub fn get_game_dir(custom_path: Option<PathBuf>, instance_id: Option<u32>) -> Result<PathBuf> {
    if let Some(path) = custom_path {
        return Ok(path);
    }
    let launcher_dir = get_launcher_dir()?;
    match instance_id {
        Some(id) => Ok(launcher_dir.join(INSTANCES).join(format!("instance_{id}"))),
        None => Ok(launcher_dir),
    }
}
