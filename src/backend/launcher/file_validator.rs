//! File validation.

use crate::backend::utils::launcher::paths::get_version_jar_path;
use crate::utils::Result;
use crate::{log_debug, log_error, simple_error};
use std::path::{Path, PathBuf};

pub struct FileValidator;

impl FileValidator {
    /// Verify that critical game files exist.
    pub fn verify_critical_files(
        game_dir: &Path,
        version_id: &str,
        library_paths: &[PathBuf],
    ) -> Result<()> {
        log_debug!("Verifying game files for version {version_id}");

        // Check main jar
        let main_jar = get_version_jar_path(game_dir, version_id);
        if !main_jar.exists() {
            return Err(simple_error!("Main jar file missing: {main_jar:?}"));
        }

        // Check critical libraries
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
                log_error!("  - {lib:?}");
            }
            return Err(simple_error!(
                "Missing {} critical libraries. Please re-download the version",
                missing_libs.len()
            ));
        }

        log_debug!("Game files verification passed");
        Ok(())
    }
}
