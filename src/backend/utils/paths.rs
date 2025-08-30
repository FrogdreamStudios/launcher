//! Path utilities.

use std::path::PathBuf;

use anyhow::Result;

/// Name of the main launcher directory.
const LAUNCHER_DIR: &str = "DreamLauncher";

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
