
//! File system utilities for common operations.

use crate::utils::Result;
use crate::utils::{Digest, Sha1};
use crate::{log_debug, log_info};
use std::path::Path;
use tokio::fs;

/// Ensures a directory exists, creating it and all parent directories if necessary.
pub async fn ensure_directory<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        fs::create_dir_all(path).await?;
        log_debug!("Created directory: {path:?}");
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

/// Batch creates multiple directories for optimal I/O performance.
/// Deduplicates paths and creates them concurrently to minimize system calls.
pub async fn batch_ensure_directories<P: AsRef<Path>>(paths: Vec<P>) -> Result<()> {
    use std::collections::HashSet;
    use tokio::task::JoinSet;

    if paths.is_empty() {
        return Ok(());
    }

    // Deduplicate and collect unique parent directories
    let mut unique_dirs = HashSet::new();
    for path in paths {
        let path = path.as_ref();
        if !path.exists() {
            unique_dirs.insert(path.to_path_buf());
        }
        // Also collect parent directories for file paths
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                unique_dirs.insert(parent.to_path_buf());
            }
        }
    }

    if unique_dirs.is_empty() {
        return Ok(());
    }

    log_debug!("Batch creating {} directories", unique_dirs.len());

    // Create directories concurrently in batches
    let mut join_set = JoinSet::new();
    const BATCH_SIZE: usize = 16; // Limit concurrent operations

    for chunk in unique_dirs
        .into_iter()
        .collect::<Vec<_>>()
        .chunks(BATCH_SIZE)
    {
        for dir in chunk {
            let dir = dir.clone();
            join_set.spawn(async move {
                if !dir.exists() {
                    fs::create_dir_all(&dir).await?;
                    log_debug!("Batch created directory: {dir:?}");
                }
                Ok::<(), std::io::Error>(())
            });
        }

        // Process batch and wait for completion
        while join_set.len() >= BATCH_SIZE {
            if let Some(result) = join_set.join_next().await {
                result??; // Handle both join and IO errors
            }
        }
    }

    // Wait for remaining tasks
    while let Some(result) = join_set.join_next().await {
        result??;
    }

    Ok(())
}

/// Batch ensure parent directories for multiple file paths.
pub async fn batch_ensure_parent_directories<P: AsRef<Path>>(file_paths: Vec<P>) -> Result<()> {
    let parent_dirs: Vec<_> = file_paths
        .into_iter()
        .filter_map(|path| path.as_ref().parent().map(|p| p.to_path_buf()))
        .collect();

    batch_ensure_directories(parent_dirs).await
}

/// Checks if a file exists and optionally verifies its size and SHA1 hash.
pub async fn verify_file<P: AsRef<Path>>(
    path: P,
    expected_size: Option<u64>,
    expected_sha1: Option<&str>,
) -> Result<bool> {
    use tokio::io::AsyncReadExt;

    let path = path.as_ref();

    if !path.exists() {
        return Ok(false);
    }

    // Get metadata once for both size and potential file operations
    let metadata = fs::metadata(path).await?;

    // Check file size if provided (early return for performance)
    if let Some(expected_size) = expected_size {
        if metadata.len() != expected_size {
            log_debug!(
                "File size mismatch for {:?}: expected {}, got {}",
                path,
                expected_size,
                metadata.len()
            );
            return Ok(false);
        }
    }

    // Early return if only a size check was needed
    if expected_sha1.is_none() {
        return Ok(true);
    }

    // Check SHA1 hash with buffered reading for better performance
    if let Some(expected_sha1) = expected_sha1 {
        let mut file = fs::File::open(path).await?;
        let mut hasher = Sha1::new();
        let mut buffer = vec![0u8; 65536]; // 64KB buffer for optimal performance

        loop {
            match file.read(&mut buffer).await? {
                0 => break, // EOF
                n => hasher.update(&buffer[..n]),
            }
        }

        let computed_hash = crate::utils::hex_encode(hasher.finalize());

        if computed_hash != expected_sha1 {
            log_debug!("SHA1 mismatch for {path:?}: expected {expected_sha1}, got {computed_hash}");
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
        log_debug!("Removed file: {path:?}");
    }
    Ok(())
}

/// Removes a directory and all its contents if it exists.
pub async fn remove_dir_if_exists<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if path.exists() {
        fs::remove_dir_all(path).await?;
        log_info!("Removed directory: {path:?}");
    }
    Ok(())
}

/// Gets the file size in bytes.
pub async fn get_file_size<P: AsRef<Path>>(path: P) -> Result<u64> {
    let metadata = fs::metadata(path).await?;
    Ok(metadata.len())
}
