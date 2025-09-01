//! Windows updater helper that replaces the main executable after it exits.
//! This is necessary because Windows locks running executables and prevents replacement.

use anyhow::{Context, Result};
use log::{error, info, warn};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        error!("Usage: {} <temp_file_path> <target_exe_path>", args[0]);
        std::process::exit(1);
    }

    let temp_file = &args[1];
    let target_exe = &args[2];

    info!("Windows updater helper");
    info!("Temp file: {temp_file}");
    info!("Target executable: {target_exe}");

    // Wait for the main process to exit
    info!("Waiting for main process to exit...");
    thread::sleep(Duration::from_secs(2));

    // Try to replace the executable multiple times if needed
    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 10;

    while attempts < MAX_ATTEMPTS {
        match replace_executable(temp_file, target_exe) {
            Ok(_) => {
                info!("Successfully replaced executable!");
                break;
            }
            Err(e) => {
                attempts += 1;
                error!("Attempt {attempts}/{MAX_ATTEMPTS} failed: {e}");

                if attempts < MAX_ATTEMPTS {
                    info!("Retrying in 1 second...");
                    thread::sleep(Duration::from_secs(1));
                } else {
                    error!("Failed to replace executable after {MAX_ATTEMPTS} attempts");
                    std::process::exit(1);
                }
            }
        }
    }

    // Clean up temp file
    if let Err(e) = fs::remove_file(temp_file) {
        warn!("Failed to remove temp file {temp_file}: {e}");
    }

    // Start the updated application
    info!("Starting updated application...");
    match Command::new(target_exe).spawn() {
        Ok(_) => {
            info!("Successfully started updated application");
        }
        Err(e) => {
            error!("Failed to start updated application: {e}");
            std::process::exit(1);
        }
    }
}

fn replace_executable(temp_file: &str, target_exe: &str) -> Result<()> {
    // Check if temp file exists
    if !Path::new(temp_file).exists() {
        return Err(anyhow::anyhow!("Temp file does not exist: {temp_file}"));
    }

    // Check if target executable exists
    if !Path::new(target_exe).exists() {
        return Err(anyhow::anyhow!(
            "Target executable does not exist: {target_exe}"
        ));
    }

    // Create backup of current executable
    let backup_path = format!("{target_exe}.backup");
    fs::copy(target_exe, &backup_path).with_context(|| "Failed to create backup")?;

    // Try to replace the executable
    match fs::copy(temp_file, target_exe) {
        Ok(_) => {
            // Remove backup on success
            let _ = fs::remove_file(&backup_path);
            Ok(())
        }
        Err(e) => {
            // Restore backup on failure
            if let Err(restore_err) = fs::copy(&backup_path, target_exe) {
                error!("Critical error: Failed to restore backup: {restore_err}");
            }
            let _ = fs::remove_file(&backup_path);
            Err(anyhow::anyhow!("Failed to replace executable: {e}"))
        }
    }
}
