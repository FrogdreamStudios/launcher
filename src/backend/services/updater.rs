//! Service that checks for updates and downloads them automatically.

use self_update::cargo_crate_version;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    label: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Release {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
    #[serde(default)]
    body: Option<String>,
}

fn find_platform_asset(assets: &[ReleaseAsset]) -> Option<&ReleaseAsset> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    
    // Define platform-specific patterns
    let patterns = match os {
        "windows" => vec![
            format!("Dream Launcher-{}.exe", arch),
            format!("Dream Launcher-windows-{}.exe", arch),
            "Dream Launcher.exe".to_string(),
            format!("launcher-{}.exe", arch),
            format!("launcher-windows-{}.exe", arch),
        ],
        "macos" => vec![
            format!("Dream Launcher-{}.dmg", arch),
            format!("Dream Launcher-macos-{}.dmg", arch),
            "Dream Launcher.dmg".to_string(),
            format!("launcher-{}.dmg", arch),
            format!("launcher-macos-{}.dmg", arch),
        ],
        "linux" => vec![
            format!("Dream Launcher-{}.AppImage", arch),
            format!("Dream Launcher-linux-{}.AppImage", arch),
            format!("Dream Launcher-{}.tar.gz", arch),
            format!("Dream Launcher-linux-{}.tar.gz", arch),
            "Dream Launcher".to_string(),
            format!("launcher-{}", arch),
            format!("launcher-linux-{}", arch),
        ],
        _ => return None,
    };
    
    // Try to find exact matches first
    for pattern in &patterns {
        if let Some(asset) = assets.iter().find(|a| a.name == *pattern) {
            return Some(asset);
        }
    }
    
    // Fallback: try partial matches
    for pattern in &patterns {
        if let Some(asset) = assets.iter().find(|a| {
            a.name.to_lowercase().contains(&pattern.to_lowercase()) ||
            pattern.to_lowercase().contains(&a.name.to_lowercase())
        }) {
            return Some(asset);
        }
    }
    
    // Last resort: match by file extension
    let extensions = match os {
        "windows" => vec![".exe", ".msi"],
        "macos" => vec![".dmg", ".pkg"],
        "linux" => vec![".AppImage", ".tar.gz", ".deb", ".rpm"],
        _ => return None,
    };
    
    for ext in extensions {
        if let Some(asset) = assets.iter().find(|a| a.name.to_lowercase().ends_with(ext)) {
            return Some(asset);
        }
    }
    
    None
}

async fn download_file_to_disk(url: &str, target_path: &std::path::Path, expected_sha256: Option<&str>) -> Result<(), String> {
    use crate::frontend::services::states::set_update_state;
    use futures_util::StreamExt;
    use sha2::{Sha256, Digest};
    use tokio::io::AsyncWriteExt;

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
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = target_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create directory: {e}"))?;
    }
    
    let mut file = tokio::fs::File::create(target_path)
        .await
        .map_err(|e| format!("Failed to create file: {e}"))?;
    
    let mut hasher = if expected_sha256.is_some() {
        Some(Sha256::new())
    } else {
        None
    };

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Failed to read chunk: {e}"))?;
        
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write to file: {e}"))?;
        
        if let Some(ref mut h) = hasher {
            h.update(&chunk);
        }
        
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
    
    file.flush().await.map_err(|e| format!("Failed to flush file: {e}"))?;
    drop(file);
    
    // Verify SHA256 if expected hash is provided
    if let (Some(expected), Some(hasher)) = (expected_sha256, hasher) {
        let computed_hash = hex::encode(hasher.finalize());
        if computed_hash != expected {
            // Remove the corrupted file
            let _ = tokio::fs::remove_file(target_path).await;
            return Err(format!(
                "SHA256 verification failed. Expected: {}, Got: {}",
                expected, computed_hash
            ));
        }
        log::info!("SHA256 verification passed: {}", computed_hash);
    }

    Ok(())
}

/// Try to find SHA256 hash for an asset from release body or asset label
fn find_asset_sha256(asset: &ReleaseAsset, release_body: Option<&str>) -> Option<String> {
    // First, check if the asset label contains a SHA256 hash
    if let Some(label) = &asset.label {
        if let Some(hash) = extract_sha256_from_text(label) {
            return Some(hash);
        }
    }
    
    // Then check the release body for SHA256 hashes
    if let Some(body) = release_body {
        // Look for patterns like "filename.ext: sha256hash" or "filename.ext sha256hash"
        let asset_name = &asset.name;
        
        // Split body into lines and look for our asset
        for line in body.lines() {
            if line.contains(asset_name) {
                if let Some(hash) = extract_sha256_from_text(line) {
                    return Some(hash);
                }
            }
        }
        
        // Also look for a separate .sha256 file asset
        let _sha256_filename = format!("{}.sha256", asset_name);
        // This would require downloading the .sha256 file, which we'll skip for now
        // but could be implemented later
    }
    
    None
}

/// Extract SHA256 hash from text (64 hex characters)
fn extract_sha256_from_text(text: &str) -> Option<String> {
    use regex::Regex;
    
    // SHA256 is 64 hex characters
    let re = Regex::new(r"\b[a-fA-F0-9]{64}\b").ok()?;
    
    if let Some(captures) = re.find(text) {
        return Some(captures.as_str().to_lowercase());
    }
    
    None
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
    use std::process::Command;
    
    // Find the updater.exe helper
    let current_dir = current_exe.parent().ok_or("Cannot get parent directory")?;
    let updater_path = current_dir.join("updater.exe");
    
    if !updater_path.exists() {
        return Err(format!("Updater helper not found at: {}", updater_path.display()));
    }
    
    // Write the new executable to a temporary file
    let temp_path = current_exe.with_extension("exe.new");
    std::fs::write(&temp_path, new_content)
        .map_err(|e| format!("Failed to write new executable: {e}"))?;
    
    log::info!("Starting Windows updater helper: {}", updater_path.display());
    
    // Start the updater helper with temp file and target executable paths
    let mut cmd = Command::new(&updater_path);
    cmd.arg(temp_path.to_string_lossy().to_string())
       .arg(current_exe.to_string_lossy().to_string());
    
    match cmd.spawn() {
        Ok(_) => {
            log::info!("Updater helper started successfully. Main process will exit now.");
            Ok(())
        }
        Err(e) => {
            // Clean up temp file on failure
            let _ = std::fs::remove_file(&temp_path);
            Err(format!("Failed to start updater helper: {}", e))
        }
    }
}

fn replace_executable_unix(current_exe: &PathBuf, new_content: &[u8]) -> Result<(), String> {
    let temp_path = current_exe.with_extension("new");

    // Write a new executable to the temp file
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
            log::info!("Warning: Failed to run xattr command: {e}");
        }
    }

    // Make sure the executable has proper permissions
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

    // Create a temp directory for DMG and mount point
    let temp_dir = std::env::temp_dir().join(format!("dreamlauncher_update_{version}"));
    fs::create_dir_all(&temp_dir)
        .await
        .map_err(|e| format!("Failed to create temp directory: {e}"))?;

    let dmg_path = temp_dir.join("Dream Launcher.dmg");
    let mount_point = temp_dir.join("mount");
    
    fs::create_dir_all(&mount_point)
        .await
        .map_err(|e| format!("Failed to create mount point directory: {e}"))?;

    // Write DMG to a temp file
    fs::write(&dmg_path, dmg_content)
        .await
        .map_err(|e| format!("Failed to write DMG file: {e}"))?;

    // Mount the DMG with explicit mount point
    let mount_output = Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
        .arg(&mount_point)
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

    log::info!("DMG mounted successfully at: {}", mount_point.display());

    // Find the app bundle in the mounted DMG
    log::info!("Looking for .app bundle in: {}", mount_point.display());

    let app_entries = std::fs::read_dir(&mount_point).map_err(|e| {
        format!(
            "Failed to read mounted DMG contents at {}: {}",
            mount_point.display(),
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
    
    // Try to install to ~/Applications first, fallback to /Applications with admin privileges
    let home_dir = std::env::var("HOME").map_err(|_| "Could not get HOME directory")?;
    let user_apps_dir = std::path::Path::new(&home_dir).join("Applications");
    let system_apps_dir = std::path::Path::new("/Applications");
    
    // Ensure ~/Applications exists
    if let Err(e) = std::fs::create_dir_all(&user_apps_dir) {
        log::warn!("Could not create ~/Applications directory: {}", e);
    }
    
    let (app_destination, needs_admin) = if user_apps_dir.exists() {
        (user_apps_dir.join(&app_name), false)
    } else {
        (system_apps_dir.join(&app_name), true)
    };

    log::info!(
        "Installing app from {} to {} (admin required: {})",
        app_source.display(),
        app_destination.display(),
        needs_admin
    );

    // Remove an existing app if it exists
    if app_destination.exists() {
        log::info!("Removing existing app at {}", app_destination.display());
        let remove_result = if needs_admin {
            // Try with admin privileges using osascript
            Command::new("osascript")
                .args(["-e", &format!(
                    "do shell script \"rm -rf '{}'\" with administrator privileges",
                    app_destination.display()
                )])
                .output()
        } else {
            Command::new("rm")
                .args(["-rf"])
                .arg(&app_destination)
                .output()
        };

        match remove_result {
            Ok(output) if output.status.success() => {
                log::info!("Successfully removed existing app");
            }
            Ok(output) => {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                log::warn!("Could not remove existing app: {}", error_msg);
            }
            Err(e) => {
                log::warn!("Failed to remove existing app: {}", e);
            }
        }
    }

    // Copy the app to Applications
    let target_dir = app_destination.parent().unwrap();
    log::info!("Copying app to {} folder...", target_dir.display());
    
    let cp_result = if needs_admin {
        // Try with admin privileges using osascript
        Command::new("osascript")
            .args(["-e", &format!(
                "do shell script \"cp -R '{}' '{}'\" with administrator privileges",
                app_source.display(),
                target_dir.display()
            )])
            .output()
    } else {
        Command::new("cp")
            .args(["-R"])
            .arg(&app_source)
            .arg(target_dir)
            .output()
    };

    match cp_result {
        Ok(output) if output.status.success() => {
            log::info!("Successfully copied app to {}", target_dir.display());
        }
        Ok(output) => {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            log::error!("cp command failed: {}", error_msg);
            // Unmount before returning error
            let _ = Command::new("hdiutil")
                .args(["detach", "-quiet"])
                .arg(&mount_point)
                .output();
            let _ = fs::remove_dir_all(&temp_dir).await;
            
            if needs_admin {
                return Err(format!("Failed to copy app to {} (admin privileges required): {}. Please try installing manually.", target_dir.display(), error_msg));
            } else {
                // If user installation failed, try system installation
                log::info!("User installation failed, trying system installation with admin privileges...");
                // Fallback to system installation with admin privileges
                 let _system_app_destination = std::path::Path::new("/Applications").join(&app_name);
                 let system_cp_result = Command::new("osascript")
                     .args(["-e", &format!(
                         "do shell script \"cp -R '{}' '/Applications/'\" with administrator privileges",
                         app_source.display()
                     )])
                     .output();
                 
                 match system_cp_result {
                     Ok(output) if output.status.success() => {
                         log::info!("Successfully copied app to /Applications with admin privileges");
                         // Continue with the rest of the installation process
                     }
                     Ok(output) => {
                         let error_msg = String::from_utf8_lossy(&output.stderr);
                         let _ = Command::new("hdiutil")
                             .args(["detach", "-quiet"])
                             .arg(&mount_point)
                             .output();
                         let _ = fs::remove_dir_all(&temp_dir).await;
                         return Err(format!("Failed to copy app to /Applications (admin privileges required): {}. Please try installing manually.", error_msg));
                     }
                     Err(e) => {
                         let _ = Command::new("hdiutil")
                             .args(["detach", "-quiet"])
                             .arg(&mount_point)
                             .output();
                         let _ = fs::remove_dir_all(&temp_dir).await;
                         return Err(format!("Failed to execute system copy command: {}", e));
                     }
                 }
            }
        }
        Err(e) => {
            let _ = Command::new("hdiutil")
                .args(["detach", "-quiet"])
                .arg(&mount_point)
                .output();
            let _ = fs::remove_dir_all(&temp_dir).await;
            return Err(format!("Failed to execute copy command: {}", e));
        }
    }

    // Remove macOS security restrictions
    log::info!("Removing macOS security restrictions...");

    // Remove quarantine attributes
    let xattr_result = if needs_admin {
        Command::new("osascript")
            .args(["-e", &format!(
                "do shell script \"xattr -r -d com.apple.quarantine '{}'\" with administrator privileges",
                app_destination.display()
            )])
            .output()
    } else {
        Command::new("xattr")
            .args(["-r", "-d", "com.apple.quarantine"])
            .arg(&app_destination)
            .output()
    };

    match xattr_result {
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
    let spctl_result = if needs_admin {
        Command::new("osascript")
            .args(["-e", &format!(
                "do shell script \"spctl --add --label 'DreamLauncher-AutoUpdate' '{}'\" with administrator privileges",
                app_destination.display()
            )])
            .output()
    } else {
        Command::new("spctl")
            .args(["--add", "--label", "DreamLauncher-AutoUpdate"])
            .arg(&app_destination)
            .output()
    };

    match spctl_result {
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

    // Unmount the DMG
    let detach_output = Command::new("hdiutil")
        .args(["detach", "-quiet"])
        .arg(&mount_point)
        .output()
        .map_err(|e| format!("Failed to unmount DMG: {}", e))?;

    if !detach_output.status.success() {
        log::info!("Warning: Failed to unmount DMG, but installation completed");
    }

    // Clean up temp directory
    let _ = fs::remove_dir_all(&temp_dir).await;

    Ok(())
}



pub async fn check_for_updates() {
    use crate::frontend::services::states::set_update_state;

    log::info!("Checking for updates...");
    set_update_state(true, 0.0, "Checking for updates...".to_string());

    // We'll find the platform asset later after getting the release info

    // Fetch the latest release info from GitHub
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
            set_update_state(false, 0.0, "Failed to check for updates. Please check your internet connection.".to_string());
            return;
        }
    };

    let release = match response.json::<Release>().await {
        Ok(release) => release,
        Err(e) => {
            log::error!("Failed to parse GitHub release info: {e}");
            set_update_state(false, 0.0, "Failed to parse update information from server.".to_string());
            return;
        }
    };

    // Check if we need to update
    set_update_state(true, 20.0, "Checking version...".to_string());
    let current_version = cargo_crate_version!();
    let latest_version = release.tag_name.trim_start_matches('v');

    if !is_version_newer(latest_version, current_version) {
        log::info!("No newer version available: latest {latest_version}, current {current_version}");
        set_update_state(false, 0.0, "You are running the latest version.".to_string());
        return;
    }

    log::info!("New version available: {latest_version} (current: {current_version})");
    set_update_state(
        true,
        25.0,
        format!("New version {latest_version} available!"),
    );

    // Find the asset for our platform
    let asset = if let Some(asset) = find_platform_asset(&release.assets) {
        asset
    } else {
        log::error!("No compatible binary found for platform: {} {}", std::env::consts::OS, std::env::consts::ARCH);
        set_update_state(false, 0.0, format!("No update available for your platform ({} {})", std::env::consts::OS, std::env::consts::ARCH));
        return;
    };
    
    log::info!("Found compatible asset: {}", asset.name);

    log::info!("Downloading update from: {}", asset.browser_download_url);
    set_update_state(true, 30.0, "Starting download...".to_string());

    // Create temp file path
    let temp_dir = std::env::temp_dir().join("dreamlauncher_update");
    let temp_file = temp_dir.join(&asset.name);

    // Try to find SHA256 hash for integrity verification
    let expected_sha256 = find_asset_sha256(asset, release.body.as_deref());
    if let Some(ref hash) = expected_sha256 {
        log::info!("Found SHA256 hash for verification: {}", hash);
    } else {
        log::warn!("No SHA256 hash found for asset {}. File integrity will not be verified.", asset.name);
    }

    // Download the new version to disk
    match download_file_to_disk(&asset.browser_download_url, &temp_file, expected_sha256.as_deref()).await {
        Ok(_) => {
            log::info!("Download completed successfully");
        }
        Err(e) => {
            log::error!("Failed to download update: {e}");
            set_update_state(false, 0.0, format!("Download failed: {e}"));
            return;
        }
    }

    log::info!("Download completed. Installing update...");
    set_update_state(true, 95.0, "Installing update...".to_string());

    // Handle DMG files on macOS (automatic installation)
    if asset.name.to_lowercase().ends_with(".dmg") {
        log::info!("DMG file detected. Installing automatically...");

        // Read DMG content for install_dmg function
        let dmg_content = match tokio::fs::read(&temp_file).await {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to read DMG file: {e}");
                set_update_state(false, 0.0, format!("Failed to read DMG: {e}"));
                let _ = tokio::fs::remove_file(&temp_file).await;
                return;
            }
        };

        match install_dmg(&dmg_content, &release.tag_name).await {
            Ok(_) => {
                log::info!("DMG installation completed successfully!");
                log::info!("The application has been updated to version {latest_version}");
                set_update_state(
                    true,
                    100.0,
                    format!("Update to {latest_version} completed!"),
                );
                let _ = tokio::fs::remove_file(&temp_file).await;
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                set_update_state(false, 0.0, "".to_string()); // Clear status before restart

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
                set_update_state(false, 0.0, format!("Installation failed: {e}. Please install manually."));
                let _ = tokio::fs::remove_file(&temp_file).await;
                return;
            }
        }
    }

    // Replace the current executable (for non-DMG files)
    let file_content = match tokio::fs::read(&temp_file).await {
        Ok(content) => content,
        Err(e) => {
            log::error!("Failed to read downloaded file: {e}");
            set_update_state(false, 0.0, format!("Failed to read file: {e}"));
            let _ = tokio::fs::remove_file(&temp_file).await;
            return;
        }
    };

    match replace_executable(&file_content) {
        Ok(_) => {
            log::info!("Update installed successfully!");
            set_update_state(true, 100.0, format!("Update to {latest_version} completed"));
            let _ = tokio::fs::remove_file(&temp_file).await;
            
            // On Windows, the helper will restart the app, so we just exit
            // On other platforms, we restart manually
            if std::env::consts::OS == "windows" {
                log::info!("Windows updater helper will restart the application");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                std::process::exit(0);
            } else {
                log::info!("The application will now restart with version {latest_version}");
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                restart_launcher();
            }
        }
        Err(e) => {
            log::error!("Failed to install update: {e}");
            set_update_state(false, 0.0, format!("Installation failed: {e}"));
            let _ = tokio::fs::remove_file(&temp_file).await;
        }
    }
}

fn is_version_newer(new: &str, current: &str) -> bool {
    use semver::Version;
    
    // Parse versions, handling 'v' prefix
    let new_clean = new.trim_start_matches('v');
    let current_clean = current.trim_start_matches('v');
    
    match (Version::parse(new_clean), Version::parse(current_clean)) {
        (Ok(new_ver), Ok(current_ver)) => new_ver > current_ver,
        _ => {
            // Fallback to simple string comparison if semver parsing fails
            log::warn!("Failed to parse versions as semver: '{}' vs '{}', using fallback", new, current);
            let parse_version = |v: &str| -> Vec<u32> {
                v.split('.').filter_map(|s| s.parse().ok()).collect()
            };
            let new_parts = parse_version(new_clean);
            let current_parts = parse_version(current_clean);
            new_parts > current_parts
        }
    }
}

fn restart_launcher() {
    use std::process::Command;
    use std::time::Duration;

    // Get the current executable path
    let current_exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            log::error!("Failed to get current executable path: {e}");
            return;
        }
    };

    log::info!("Restarting launcher from: {current_exe:?}");

    #[cfg(target_os = "macos")]
    {
        // On macOS, if we're running from Applications, launch the .app bundle
        if let Some(app_path) = current_exe.to_str()
            && app_path.contains("/Applications/Dream Launcher.app/")
        {
            // Launch the .app bundle and wait briefly before exiting
            match Command::new("open")
                .arg("/Applications/Dream Launcher.app")
                .spawn()
            {
                Ok(mut child) => {
                    log::info!("Successfully launched new instance from Applications");
                    
                    // Give the new process time to start before we exit
                    std::thread::sleep(Duration::from_millis(500));
                    
                    // Check if the new process is still running
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            log::warn!("New process exited immediately with status: {status}");
                        }
                        Ok(None) => {
                            log::info!("New process is running, exiting old instance");
                            std::process::exit(0);
                        }
                        Err(e) => {
                            log::warn!("Could not check new process status: {e}");
                            std::process::exit(0);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to launch from Applications: {e}");
                }
            }
        }
    }

    // Fallback: launch the current executable directly
    match Command::new(&current_exe).spawn() {
        Ok(mut child) => {
            log::info!("Successfully launched new instance");
            
            // Give the new process time to start before we exit
            std::thread::sleep(Duration::from_millis(500));
            
            // Check if the new process is still running
            match child.try_wait() {
                Ok(Some(status)) => {
                    log::error!("New process exited immediately with status: {status}");
                    log::error!("Failed to restart launcher - new process did not start properly");
                }
                Ok(None) => {
                    log::info!("New process is running, exiting old instance");
                    std::process::exit(0);
                }
                Err(e) => {
                    log::warn!("Could not check new process status: {e}, assuming success");
                    std::process::exit(0);
                }
            }
        }
        Err(e) => {
            log::error!("Failed to restart launcher: {e}");
        }
    }
}
