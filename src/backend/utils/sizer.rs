//! Directory size calculation.

use anyhow::Result;

/// Calculates the total size of a directory or file in bytes.
///
/// For files, returns the file size directly. For directories,
/// recursively calculates the total size of all contained files.
pub fn calculate_directory_size(path: &std::path::Path) -> Result<u64> {
    if path.is_file() {
        // For files, just return the file size
        return Ok(path.metadata()?.len());
    }
    if path.is_dir() {
        // For directories, recursively sum all file sizes
        return std::fs::read_dir(path)?
            .map(|entry| calculate_directory_size(&entry?.path()))
            .try_fold(0, |acc, size| Ok(acc + size?));
    }

    // Path doesn't exist or isn't a file/directory
    Ok(0)
}
