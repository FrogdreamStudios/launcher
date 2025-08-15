//! Common utilities and platform information for the launcher.

use crate::backend::utils::system::files::verify_file;
use crate::backend::utils::system::os::{
    get_all_native_classifiers, get_minecraft_arch, get_minecraft_os_name, get_os_features,
};
use crate::log_info;
use crate::utils::Result;
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

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

/// Common download utilities.
pub struct DownloadHelper;

impl DownloadHelper {
    /// Check if a file needs to be downloaded based on size and hash verification.
    pub async fn needs_download(
        path: &PathBuf,
        expected_size: Option<u64>,
        expected_hash: Option<&str>,
    ) -> Result<bool> {
        Ok(!verify_file(path, expected_size, expected_hash).await?)
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
        } else if total_items < 100 {
            16
        } else {
            max_batch.min(32)
        }
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
