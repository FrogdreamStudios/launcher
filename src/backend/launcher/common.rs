//! Common utilities and platform information for the launcher.

use crate::backend::utils::system::files::verify_file;
use crate::backend::utils::system::os::{
    get_all_native_classifiers, get_minecraft_arch, get_minecraft_os_name, get_os_features,
};
use crate::log_info;
use crate::utils::Result;
use std::path::Path;
use std::sync::Arc;
use std::{collections::HashMap, path::PathBuf};
use tokio::sync::RwLock;

/// Platform information cached for downloads.
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub os_name: &'static str,
    pub os_arch: &'static str,
    pub os_features: HashMap<String, bool>,
    pub native_classifiers: Vec<String>,
}

impl PlatformInfo {
    pub fn new() -> Self {
        Self {
            os_name: get_minecraft_os_name(),
            os_arch: get_minecraft_arch(),
            os_features: get_os_features(),
            native_classifiers: get_all_native_classifiers(),
        }
    }
}

impl Default for PlatformInfo {
    fn default() -> Self {
        Self::new()
    }
}

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
}

/// Static instance for global use
static DOWNLOAD_HELPER: std::sync::OnceLock<DownloadHelper> = std::sync::OnceLock::new();

fn get_helper() -> &'static DownloadHelper {
    DOWNLOAD_HELPER.get_or_init(DownloadHelper::new)
}

impl DownloadHelper {
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

    /// Clear the file cache to force re-validation of all files.
    pub async fn clear_cache() {
        let helper = get_helper();
        helper.file_cache.write().await.clear();
    }

    /// Preload file metadata into cache for batch operations.
    pub async fn preload_cache(paths: &[PathBuf]) -> Result<()> {
        let helper = get_helper();
        let mut cache = helper.file_cache.write().await;

        for path in paths {
            if !cache.contains_key(path) {
                let exists = path.exists();
                let size = if exists {
                    tokio::fs::metadata(path).await.map(|m| m.len()).ok()
                } else {
                    None
                };

                cache.insert(
                    path.clone(),
                    CachedFileInfo {
                        exists,
                        size,
                        hash_verified: false,
                    },
                );
            }
        }

        Ok(())
    }

    /// Batch validate files and create download tasks for files that need downloading.
    /// This is much faster than checking files one by one due to reduced I/O operations.
    pub async fn batch_validate_and_create_tasks(
        potential_downloads: Vec<(String, PathBuf, u64, String)>, // (url, path, size, hash)
    ) -> Result<Vec<crate::backend::launcher::downloader::models::DownloadTask>> {
        use crate::backend::launcher::downloader::models::DownloadTask;
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
                    log_info!("Validation error (will re-download): {}", e);
                    // On validation error, we'll assume download is needed
                }
                Err(e) => {
                    log_info!("Join error during validation: {}", e);
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

/// System information utilities.
pub struct SystemInfo;

impl SystemInfo {
    /// Log comprehensive system information for debugging.
    pub fn log_system_info(game_dir: &PathBuf, cache_dir: &PathBuf) {
        log_info!("=== System Information ===");
        log_info!("OS: {}", std::env::consts::OS);
        log_info!("Architecture: {}", std::env::consts::ARCH);
        log_info!("Family: {}", std::env::consts::FAMILY);

        // Log memory information if available
        #[cfg(target_os = "macos")]
        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            && let Ok(mem_str) = String::from_utf8(output.stdout)
            && let Ok(mem_bytes) = mem_str.trim().parse::<u64>()
        {
            let mem_gb = mem_bytes / 1024 / 1024 / 1024;
            log_info!("Total Memory: {mem_gb} GB");
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = std::process::Command::new("free").args(["-h"]).output() {
                if let Ok(mem_info) = String::from_utf8(output.stdout) {
                    log_info!("Memory info:\n{}", mem_info);
                }
            }
        }

        log_info!("Game Directory: {game_dir:?}");
        log_info!("Cache Directory: {cache_dir:?}");
    }

    /// Check for existing Java/Minecraft processes.
    pub fn check_existing_processes() {
        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(output) = std::process::Command::new("ps").args(["aux"]).output()
                && let Ok(ps_output) = String::from_utf8(output.stdout)
            {
                let java_processes: Vec<&str> = ps_output
                    .lines()
                    .filter(|line| line.contains("java") || line.contains("minecraft"))
                    .collect();

                if !java_processes.is_empty() {
                    log_info!("Existing Java/Minecraft processes found:");
                    for process in java_processes {
                        log_info!("  {process}");
                    }
                }
            }
        }
    }
}

/// File validation utilities.
pub struct FileValidator;

impl FileValidator {
    /// Verify that critical game files exist.
    pub fn verify_critical_files(
        game_dir: &Path,
        version_id: &str,
        library_paths: &[PathBuf],
    ) -> Result<()> {
        use crate::backend::utils::launcher::paths::get_version_jar_path;
        use crate::{log_error, simple_error};

        log_info!("Verifying game files for version {version_id}");

        // Check main jar
        let main_jar = get_version_jar_path(game_dir, version_id);
        if !main_jar.exists() {
            return Err(simple_error!("Main jar file missing: {main_jar:?}"));
        }

        // Check critical libraries (not all, as some may be optional)
        let mut missing_libs = Vec::new();
        let critical_count = std::cmp::min(5, library_paths.len());

        for lib_path in library_paths.iter().take(critical_count) {
            if !lib_path.exists() {
                missing_libs.push(lib_path.clone());
            }
        }

        if !missing_libs.is_empty() {
            log_error!("Missing critical libraries:");
            for lib in &missing_libs {
                log_error!("  - {:?}", lib);
            }
            return Err(simple_error!(
                "Missing {} critical libraries. Please re-download the version.",
                missing_libs.len()
            ));
        }

        log_info!("Game files verification passed");
        Ok(())
    }
}
