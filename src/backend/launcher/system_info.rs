//! System information utilities.

use crate::log_info;
use std::path::PathBuf;

#[cfg(debug_assertions)]
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

    log_info!("Game directory: {game_dir:?}");
    log_info!("Cache directory: {cache_dir:?}");
}

#[cfg(debug_assertions)]
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
