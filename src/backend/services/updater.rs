//! Service that checks for updates and downloads them automatically.

use self_update::cargo_crate_version;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize, Debug)]
struct Release {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

fn get_platform_asset_name() -> Option<&'static str> {
    match std::env::consts::OS {
        "windows" => Some("DreamLauncher-Windows.exe"),
        "macos" => Some("DreamLauncher-macOS.dmg"),
        "linux" => Some("DreamLauncher-Linux"),
        _ => None,
    }
}

async fn download_file(url: &str) -> Result<Vec<u8>, String> {
    use crate::frontend::services::states::set_update_state;
    use futures_util::StreamExt;
    
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "DreamLauncher-Updater")
        .send()
        .await
        .map_err(|e| format!("Failed to download file: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();
    let mut data = Vec::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Failed to read chunk: {e}"))?;
        data.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;
        
        if total_size > 0 {
            let progress = (downloaded as f32 / total_size as f32) * 100.0;
            let status = format!("Downloading update... {progress:.1}%");
            set_update_state(true, progress, status);
        } else {
            let status = format!("Downloading update... {downloaded} bytes");
            set_update_state(true, 0.0, status);
        }
    }

    Ok(data)
}

fn replace_executable(new_content: &[u8]) -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {e}"))?;

    match std::env::consts::OS {
        "windows" => replace_executable_windows(&current_exe, new_content),
        "linux" | "macos" => replace_executable_unix(&current_exe, new_content),
        _ => Err("Unsupported operating system".to_string()),
    }
}

fn replace_executable_windows(current_exe: &PathBuf, new_content: &[u8]) -> Result<(), String> {
    let temp_path = current_exe.with_extension("exe.new");
    let backup_path = current_exe.with_extension("exe.bak");

    // Write new executable to temp file
    std::fs::write(&temp_path, new_content)
        .map_err(|e| format!("Failed to write new executable: {e}"))?;

    // Create backup of current executable
    std::fs::rename(current_exe, &backup_path)
        .map_err(|e| format!("Failed to backup current executable: {e}"))?;

    // Move new executable to current location
    if let Err(e) = std::fs::rename(&temp_path, current_exe) {
        // Restore backup if replacement fails
        let _ = std::fs::rename(&backup_path, current_exe);
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("Failed to replace executable: {e}"));
    }

    // Clean up backup and temp files with retry mechanism
    for attempt in 1..=3 {
        match std::fs::remove_file(&backup_path) {
            Ok(_) => break,
            Err(e) => {
                if attempt == 3 {
                    log::warn!("Failed to clean up backup file {} after 3 attempts: {}", backup_path.display(), e);
                } else {
                    log::debug!("Attempt {} to remove backup file failed: {}", attempt, e);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }
    for attempt in 1..=3 {
        match std::fs::remove_file(&temp_path) {
            Ok(_) => break,
            Err(e) => {
                if attempt == 3 {
                    log::warn!("Failed to clean up temp file {} after 3 attempts: {}", temp_path.display(), e);
                } else {
                    log::debug!("Attempt {} to remove temp file failed: {}", attempt, e);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }

    Ok(())
}

fn replace_executable_unix(current_exe: &PathBuf, new_content: &[u8]) -> Result<(), String> {
    let temp_path = current_exe.with_extension("new");

    // Write new executable to temp file
    std::fs::write(&temp_path, new_content)
        .map_err(|e| format!("Failed to write new executable: {e}"))?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&temp_path)
            .map_err(|e| format!("Failed to get file metadata: {e}"))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&temp_path, perms)
            .map_err(|e| format!("Failed to set permissions: {e}"))?;
    }

    // Replace the current executable
    std::fs::rename(&temp_path, current_exe)
        .map_err(|e| format!("Failed to replace executable: {e}"))?;

    // On macOS, bypass security restrictions
    if std::env::consts::OS == "macos" {
        if let Err(e) = bypass_macos_security(current_exe) {
            log::info!("Warning: Could not bypass macOS security restrictions: {e}");
        }
    }

    Ok(())
}

fn bypass_macos_security(executable_path: &PathBuf) -> Result<(), String> {
    use std::process::Command;

    log::info!("Removing macOS security restrictions for executable...");

    // Remove quarantine attributes
    let xattr_output = Command::new("xattr")
        .args(["-r", "-d", "com.apple.quarantine"])
        .arg(executable_path)
        .output();

    match xattr_output {
        Ok(output) => {
            if output.status.success() {
                log::info!("Successfully removed quarantine attributes");
            } else {
                log::info!(
                    "Note: Could not remove quarantine attributes (normal if not quarantined)"
                );
            }
        }
        Err(e) => {
            log::info!("Warning: Failed to run xattr command: {}", e);
        }
    }

    // Make sure executable has proper permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(mut perms) = std::fs::metadata(executable_path).map(|m| m.permissions()) {
            perms.set_mode(0o755);
            if let Err(e) = std::fs::set_permissions(executable_path, perms) {
                log::info!("Warning: Could not set executable permissions: {e}");
            } else {
                log::info!("Successfully set executable permissions");
            }
        }
    }

    Ok(())
}

async fn install_dmg(dmg_content: &[u8], version: &str) -> Result<(), String> {
    use std::process::Command;
    use tokio::fs;

    // Create temp directory for DMG
    let temp_dir = std::env::temp_dir().join(format!("dreamlauncher_update_{version}"));
    fs::create_dir_all(&temp_dir)
        .await
        .map_err(|e| format!("Failed to create temp directory: {e}"))?;

    let dmg_path = temp_dir.join("DreamLauncher.dmg");

    // Write DMG to temp file
    fs::write(&dmg_path, dmg_content)
        .await
        .map_err(|e| format!("Failed to write DMG file: {e}"))?;

    // Mount the DMG
    let mount_output = Command::new("hdiutil")
        .args(["attach", "-nobrowse"])
        .arg(&dmg_path)
        .output()
        .map_err(|e| format!("Failed to execute hdiutil attach: {e}"))?;

    if !mount_output.status.success() {
        let stderr_output = String::from_utf8_lossy(&mount_output.stderr);
        let stdout_output = String::from_utf8_lossy(&mount_output.stdout);
        let _ = fs::remove_dir_all(&temp_dir).await;
        return Err(format!(
            "Failed to mount DMG - stdout: {stdout_output}, stderr: {stderr_output}"
        ));
    }

    let stdout_output = String::from_utf8_lossy(&mount_output.stdout);
    let stderr_output = String::from_utf8_lossy(&mount_output.stderr);
    log::info!("hdiutil attach stdout: {}", stdout_output);
    log::info!("hdiutil attach stderr: {}", stderr_output);

    // Try to parse mount point from either stdout or stderr, or use fallback
    let mount_point = parse_mount_point(&stdout_output)
        .or_else(|| parse_mount_point(&stderr_output))
        .map(|s| s.to_string())
        .or_else(|| find_mount_point_fallback())
        .ok_or_else(|| {
            format!(
                "Failed to parse mount point - stdout: '{stdout_output}', stderr: '{stderr_output}'"
            )
        })?;

    log::info!("Parsed mount point: {mount_point}");

    // Find the app bundle in the mounted DMG
    let mount_path = std::path::Path::new(&mount_point);
    log::info!("Looking for .app bundle in: {}", mount_path.display());

    let app_entries = std::fs::read_dir(mount_path).map_err(|e| {
        format!(
            "Failed to read mounted DMG contents at {}: {}",
            mount_path.display(),
            e
        )
    })?;

    let entries: Vec<_> = app_entries.flatten().collect();
    log::info!("Found {} entries in DMG", entries.len());

    for entry in &entries {
        log::info!("DMG entry: {}", entry.path().display());
    }

    let app_bundle = entries
        .into_iter()
        .find(|entry| {
            let is_app = entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "app")
                .unwrap_or(false);
            log::info!("Checking {}: is_app = {}", entry.path().display(), is_app);
            is_app
        })
        .ok_or("No .app bundle found in DMG")?;

    let app_name = app_bundle.file_name();
    let app_source = app_bundle.path();
    let app_destination = std::path::Path::new("/Applications").join(&app_name);

    log::info!(
        "Installing app from {} to {}",
        app_source.display(),
        app_destination.display()
    );

    // Remove existing app if it exists
    if app_destination.exists() {
        log::info!("Removing existing app at {}", app_destination.display());
        let remove_output = Command::new("rm")
            .args(["-rf"])
            .arg(&app_destination)
            .output()
            .map_err(|e| format!("Failed to remove existing app: {e}"))?;

        if !remove_output.status.success() {
            let error_msg = String::from_utf8_lossy(&remove_output.stderr);
            log::info!("Warning: Could not remove existing app: {error_msg}");
        } else {
            log::info!("Successfully removed existing app");
        }
    }

    // Copy the app to Applications
    log::info!("Copying app to Applications folder...");
    let cp_output = Command::new("cp")
        .args(["-R"])
        .arg(&app_source)
        .arg("/Applications/")
        .output()
        .map_err(|e| format!("Failed to execute cp command: {e}"))?;

    if !cp_output.status.success() {
        let error_msg = String::from_utf8_lossy(&cp_output.stderr);
        log::error!("cp command failed: {}", error_msg);
        // Unmount before returning error
        let _ = Command::new("hdiutil")
            .args(["detach", "-quiet"])
            .arg(mount_point)
            .output();
        let _ = fs::remove_dir_all(&temp_dir).await;
        return Err(format!("Failed to copy app to Applications: {error_msg}"));
    }

    log::info!("Successfully copied app to Applications");

    // Remove macOS security restrictions
    log::info!("Removing macOS security restrictions...");

    // Remove quarantine attributes
    let xattr_output = Command::new("xattr")
        .args(["-r", "-d", "com.apple.quarantine"])
        .arg(&app_destination)
        .output();

    match xattr_output {
        Ok(output) => {
            if output.status.success() {
                log::info!("Successfully removed quarantine attributes");
            } else {
                log::info!(
                    "Warning: Could not remove quarantine attributes (this is normal if app wasn't quarantined)"
                );
            }
        }
        Err(e) => {
            log::info!("Warning: Failed to run xattr command: {}", e);
        }
    }

    // Try to bypass Gatekeeper for this specific app
    let spctl_output = Command::new("spctl")
        .args(["--add", "--label", "DreamLauncher-AutoUpdate"])
        .arg(&app_destination)
        .output();

    match spctl_output {
        Ok(output) => {
            if output.status.success() {
                log::info!("Successfully added app to Gatekeeper exceptions");
            } else {
                log::info!(
                    "Warning: Could not add to Gatekeeper exceptions (may require admin privileges)"
                );
            }
        }
        Err(e) => {
            log::info!("Warning: Failed to run spctl command: {}", e);
        }
    }

    // Alternative approach - try to enable the app directly
    let spctl_enable_output = Command::new("spctl")
        .args(["--enable", "--label", "DreamLauncher-AutoUpdate"])
        .output();

    match spctl_enable_output {
        Ok(output) => {
            if output.status.success() {
                log::info!("Successfully enabled app in Gatekeeper");
            }
        }
        Err(_) => {
            // Silently ignore this error as it's a fallback
        }
    }

    // Unmount the DMG
    let detach_output = Command::new("hdiutil")
        .args(["detach", "-quiet"])
        .arg(mount_point)
        .output()
        .map_err(|e| format!("Failed to unmount DMG: {}", e))?;

    if !detach_output.status.success() {
        log::info!("Warning: Failed to unmount DMG, but installation completed");
    }

    // Clean up temp directory
    let _ = fs::remove_dir_all(&temp_dir).await;

    Ok(())
}

fn parse_mount_point(hdiutil_output: &str) -> Option<&str> {
    // Method 1: Look for /Volumes/ and extract everything from there to end of line
    if let Some(mount_point) = hdiutil_output.lines().find_map(|line| {
        if let Some(volumes_pos) = line.find("/Volumes/") {
            return Some(line[volumes_pos..].trim());
        }
        None
    }) {
        return Some(mount_point);
    }

    // Method 2: Parse hdiutil tabular output format
    // Format: /dev/diskXsY        Apple_HFS                      /Volumes/Volume Name
    for line in hdiutil_output.lines() {
        if line.contains("/Volumes/") {
            // Split by tabs first, then by multiple spaces
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let mount_part = parts.last().unwrap().trim();
                if mount_part.starts_with("/Volumes/") {
                    return Some(mount_part);
                }
            }

            // Fallback: split by multiple whitespace and reconstruct volume path
            if let Some(volumes_start) = line.find("/Volumes/") {
                let remaining = &line[volumes_start..];
                return Some(remaining.trim());
            }
        }
    }

    // Method 3: Last resort - find any path starting with /Volumes/
    hdiutil_output.lines().find_map(|line| {
        line.split_whitespace()
            .find(|part| part.starts_with("/Volumes/"))
    })
}

fn find_mount_point_fallback() -> Option<String> {
    use std::process::Command;

    // Try to find the most recently mounted DMG volume
    if let Ok(output) = Command::new("df").args(["-h"]).output() {
        let df_output = String::from_utf8_lossy(&output.stdout);

        // Look for /Volumes/ entries (DMG mounts typically appear here)
        let mut volumes: Vec<&str> = df_output
            .lines()
            .filter_map(|line| {
                if line.contains("/Volumes/") {
                    line.split_whitespace().last()
                } else {
                    None
                }
            })
            .collect();

        // Return the last mounted volume (most recent)
        if let Some(volume) = volumes.pop() {
            return Some(volume.to_string());
        }
    }

    // Fallback: check /Volumes directory for newest entry
    if let Ok(entries) = std::fs::read_dir("/Volumes") {
        let mut volumes: Vec<_> = entries
            .flatten()
            .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .collect();

        // Sort by modification time, newest first
        volumes.sort_by_key(|entry| {
            entry
                .metadata()
                .and_then(|meta| meta.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        volumes.reverse();

        if let Some(newest) = volumes.first() {
            return Some(format!("/Volumes/{}", newest.file_name().to_string_lossy()));
        }
    }

    None
}

pub async fn check_for_updates() {
    use crate::frontend::services::states::set_update_state;
    
    log::info!("Checking for updates...");
    set_update_state(true, 0.0, "Checking for updates...".to_string());

    // Get the platform-specific asset name
    let platform_asset_name = match get_platform_asset_name() {
        Some(name) => name,
        None => {
            log::error!("Unsupported platform for auto-updates");
            set_update_state(false, 0.0, String::new());
            return;
        }
    };

    // Fetch latest release info from GitHub
    set_update_state(true, 10.0, "Fetching release information...".to_string());
    let client = reqwest::Client::new();
    let response = match client
        .get("https://api.github.com/repos/FrogdreamStudios/launcher/releases/latest")
        .header("User-Agent", "DreamLauncher-Updater")
        .send()
        .await
    {
        Ok(res) => res,
        Err(e) => {
            log::error!("Failed to fetch release info from GitHub: {e}");
            set_update_state(false, 0.0, String::new());
            return;
        }
    };

    let release = match response.json::<Release>().await {
        Ok(release) => release,
        Err(e) => {
            log::error!("Failed to parse GitHub release info: {e}");
            set_update_state(false, 0.0, String::new());
            return;
        }
    };

    // Check if we need to update
    set_update_state(true, 20.0, "Checking version...".to_string());
    let current_version = cargo_crate_version!();
    let latest_version = release.tag_name.trim_start_matches('v');

    if latest_version == current_version {
        log::info!("Already running the latest version: {current_version}");
        set_update_state(false, 0.0, String::new());
        return;
    }

    log::info!("New version available: {latest_version} (current: {current_version})");
    set_update_state(true, 25.0, format!("New version {} available!", latest_version));

    // Find the asset for our platform
    let asset = match release
        .assets
        .iter()
        .find(|asset| asset.name == platform_asset_name)
    {
        Some(asset) => asset,
        None => {
            log::error!("No compatible binary found for platform: {platform_asset_name}");
            set_update_state(false, 0.0, String::new());
            return;
        }
    };

    log::info!("Downloading update from: {}", asset.browser_download_url);
    set_update_state(true, 30.0, "Starting download...".to_string());

    // Download the new version
    let new_content = match download_file(&asset.browser_download_url).await {
        Ok(content) => content,
        Err(e) => {
            log::error!("Failed to download update: {e}");
            set_update_state(false, 0.0, String::new());
            return;
        }
    };

    log::info!("Download completed. Installing update...");
    set_update_state(true, 95.0, "Installing update...".to_string());

    // Handle DMG files on macOS (automatic installation)
    if platform_asset_name.ends_with(".dmg") {
        log::info!("DMG file detected. Installing automatically...");

        match install_dmg(&new_content, &release.tag_name).await {
            Ok(_) => {
                log::info!("DMG installation completed successfully!");
                log::info!("The application has been updated to version {latest_version}");
                set_update_state(true, 100.0, format!("Update to {} completed!", latest_version));
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                set_update_state(false, 0.0, String::new());
                
                // Restart the launcher
                log::info!("Restarting launcher...");
                restart_launcher();
                return;
            }
            Err(e) => {
                log::error!("Failed to install DMG automatically: {e}");
                log::info!(
                    "Please download and install manually from: {}",
                    asset.browser_download_url
                );
                set_update_state(false, 0.0, String::new());
                return;
            }
        }
    }

    // Replace the current executable (for non-DMG files)
    match replace_executable(&new_content) {
        Ok(_) => {
            log::info!("Update installed successfully!");
            log::info!("The application will now restart with version {latest_version}");
            set_update_state(true, 100.0, format!("Update to {} completed!", latest_version));
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            std::process::exit(0);
        }
        Err(e) => {
            log::error!("Failed to install update: {e}");
            set_update_state(false, 0.0, String::new());
        }
    }
}

fn restart_launcher() {
    use std::process::Command;
    
    // Get the current executable path
    let current_exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            log::error!("Failed to get current executable path: {e}");
            return;
        }
    };
    
    log::info!("Restarting launcher from: {:?}", current_exe);
    
    #[cfg(target_os = "macos")]
    {
        // On macOS, if we're running from Applications, launch the .app bundle
        if let Some(app_path) = current_exe.to_str() {
            if app_path.contains("/Applications/Dream Launcher.app/") {
                // Launch the .app bundle
                match Command::new("open")
                    .arg("/Applications/Dream Launcher.app")
                    .spawn()
                {
                    Ok(_) => {
                        log::info!("Successfully launched new instance from Applications");
                        std::process::exit(0);
                    }
                    Err(e) => {
                        log::error!("Failed to launch from Applications: {e}");
                    }
                }
            }
        }
    }
    
    // Fallback: launch the current executable directly
    match Command::new(&current_exe).spawn() {
        Ok(_) => {
            log::info!("Successfully launched");
            std::process::exit(0);
        }
        Err(e) => {
            log::error!("Failed to restart launcher: {e}");
        }
    }
}
