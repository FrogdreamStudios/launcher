//! Helper for managing downloads and file caching.

use crate::backend::utils::system::files::verify_file;
use crate::log_info;
use crate::utils::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::backend::launcher::downloader::models::DownloadTask;

/// Cached file metadata for fast lookups.
#[derive(Debug, Clone)]
struct CachedFileInfo {
    exists: bool,
    size: Option<u64>,
    hash_verified: bool,
}

/// Common download utilities with file metadata caching.
pub struct DownloadHelper {
    file_cache: Arc<RwLock<HashMap<PathBuf, CachedFileInfo>>>,
}

impl Default for DownloadHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloadHelper {
    pub fn new() -> Self {
        Self {
            file_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a file needs to be downloaded with caching for better performance.
    pub async fn needs_download(
        path: &PathBuf,
        expected_size: Option<u64>,
        expected_hash: Option<&str>,
    ) -> Result<bool> {
        let helper = get_helper();

        // Check cache first
        {
            let cache = helper.file_cache.read().await;
            if let Some(cached) = cache.get(path) {
                if !cached.exists {
                    return Ok(true); // File doesn't exist, needs download
                }
                if let (Some(expected), Some(actual)) = (expected_size, cached.size) {
                    if expected != actual {
                        return Ok(true); // Size mismatch, needs download
                    }
                }
                if expected_hash.is_none() || cached.hash_verified {
                    return Ok(false); // File is valid based on cache
                }
            }
        }

        // Not in cache or needs hash verification, do full check
        let needs_download = !verify_file(path, expected_size, expected_hash).await?;

        // Update cache
        {
            let mut cache = helper.file_cache.write().await;
            cache.insert(
                path.clone(),
                CachedFileInfo {
                    exists: !needs_download,
                    size: if !needs_download { expected_size } else { None },
                    hash_verified: !needs_download && expected_hash.is_some(),
                },
            );
        }

        Ok(needs_download)
    }

    /// Log download progress information.
    pub fn log_progress(completed: usize, total: usize, item_type: &str) {
        let progress = (completed as f64 / total as f64 * 100.0).round() as u8;
        log_info!("{}: {}% ({}/{})", item_type, progress, completed, total);
    }

    /// Calculate the optimal batch size for downloads based on total count.
    pub fn calculate_batch_size(total_items: usize, max_batch: usize) -> usize {
        if total_items < 10 {
            total_items
        } else if total_items < 50 {
            32
        } else if total_items < 200 {
            48
        } else {
            max_batch
        }
    }

    /// Batch validate files and create download tasks for files that need downloading.
    /// This is much faster than checking files one by one due to reduced I/O operations.
    pub async fn batch_validate_and_create_tasks(
        potential_downloads: Vec<(String, PathBuf, u64, String)>, // (url, path, size, hash)
    ) -> Result<Vec<DownloadTask>> {
        use tokio::task::JoinSet;

        if potential_downloads.is_empty() {
            return Ok(Vec::new());
        }

        let mut join_set = JoinSet::new();
        let total = potential_downloads.len();

        // Process validations in parallel batches for maximum speed
        for (url, path, size, hash) in potential_downloads {
            join_set.spawn(async move {
                let needs_download = Self::needs_download(&path, Some(size), Some(&hash)).await?;
                if needs_download {
                    Ok(Some(DownloadTask::new(url, path).with_sha1(hash)))
                } else {
                    Ok::<Option<DownloadTask>, crate::utils::Error>(None)
                }
            });
        }

        let mut download_tasks = Vec::new();
        let mut validated = 0;

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(Some(task))) => download_tasks.push(task),
                Ok(Ok(None)) => (), // File already exists and valid
                Ok(Err(e)) => {
                    log_info!("Validation main (will re-download): {}", e);
                    // On validation main, we'll assume download is needed
                }
                Err(e) => {
                    log_info!("Join main during validation: {}", e);
                }
            }

            validated += 1;
            if validated % 100 == 0 {
                log_info!("Validated {}/{} files for download", validated, total);
            }
        }

        log_info!(
            "Batch validation complete: {}/{} files need downloading",
            download_tasks.len(),
            total
        );
        Ok(download_tasks)
    }
}

static DOWNLOAD_HELPER: std::sync::OnceLock<DownloadHelper> = std::sync::OnceLock::new();

fn get_helper() -> &'static DownloadHelper {
    DOWNLOAD_HELPER.get_or_init(DownloadHelper::new)
}
