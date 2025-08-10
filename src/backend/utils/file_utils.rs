use anyhow::Result;
use sha1::{Digest, Sha1};
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};

/// Ensures a directory exists, creating it and all parent directories if necessary.
pub async fn ensure_directory<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        fs::create_dir_all(path).await?;
        debug!("Created directory: {path:?}");
    }
    Ok(())
}

/// Ensures the parent directory of a file exists.
pub async fn ensure_parent_directory<P: AsRef<Path>>(file_path: P) -> Result<()> {
    let file_path = file_path.as_ref();
    if let Some(parent) = file_path.parent() {
        ensure_directory(parent).await?;
    }
    Ok(())
}

/// Checks if a file exists and optionally verifies its size and SHA1 hash
pub async fn verify_file<P: AsRef<Path>>(
    path: P,
    expected_size: Option<u64>,
    expected_sha1: Option<&str>,
) -> Result<bool> {
    let path = path.as_ref();

    if !path.exists() {
        return Ok(false);
    }

    // Check file size if provided
    if let Some(expected_size) = expected_size {
        let metadata = fs::metadata(path).await?;
        if metadata.len() != expected_size {
            debug!(
                "File size mismatch for {:?}: expected {}, got {}",
                path,
                expected_size,
                metadata.len()
            );
            return Ok(false);
        }
    }

    // Check SHA1 hash if provided
    if let Some(expected_sha1) = expected_sha1 {
        let content = fs::read(path).await?;
        let mut hasher = Sha1::new();
        hasher.update(&content);
        let computed_hash = hex::encode(hasher.finalize());

        if computed_hash != expected_sha1 {
            debug!("SHA1 mismatch for {path:?}: expected {expected_sha1}, got {computed_hash}");
            return Ok(false);
        }
    }

    Ok(true)
}

/// Removes a file if it exists.
pub async fn remove_file_if_exists<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if path.exists() {
        fs::remove_file(path).await?;
        debug!("Removed file: {path:?}");
    }
    Ok(())
}

/// Removes a directory and all its contents if it exists.
pub async fn remove_dir_if_exists<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if path.exists() {
        fs::remove_dir_all(path).await?;
        info!("Removed directory: {path:?}");
    }
    Ok(())
}

/// Gets the file size in bytes.
pub async fn get_file_size<P: AsRef<Path>>(path: P) -> Result<u64> {
    let metadata = fs::metadata(path).await?;
    Ok(metadata.len())
}

/// Checks if a directory contains the expected files for a Minecraft version.
pub fn is_minecraft_version_complete<P: AsRef<Path>>(version_dir: P, version_name: &str) -> bool {
    let version_dir = version_dir.as_ref();
    let jar_file = version_dir.join(format!("{version_name}.jar"));
    let json_file = version_dir.join(format!("{version_name}.json"));

    jar_file.exists() && json_file.exists()
}
