//! Core Minecraft launcher implementation.

use super::{
    downloader::{HttpDownloader, models::DownloadTask},
    java::JavaManager,
    models::{AssetManifest, AssetObject, VersionDetails, VersionInfo, VersionManifest},
};
use crate::backend::utils::launcher::paths::{
    ensure_directories, get_asset_indexes_dir, get_asset_path, get_assets_dir, get_cache_dir,
    get_game_dir, get_library_path, get_natives_dir, get_version_jar_path, get_version_json_path,
};
use crate::backend::utils::launcher::starter::CommandBuilder;
use crate::backend::utils::system::files::{
    ensure_directory, ensure_parent_directory, verify_file,
};
use crate::backend::utils::system::os::{
    get_all_native_classifiers, get_minecraft_arch, get_minecraft_os_name, get_os_features,
};
use crate::utils::Result;
use crate::{log_debug, log_error, log_info, log_warn, simple_error};
use std::{path::PathBuf, process::Stdio, sync::Arc};

/// Main Minecraft launcher that handles downloading and launching game instances.
pub struct MinecraftLauncher {
    downloader: Arc<HttpDownloader>,
    java_manager: JavaManager,
    game_dir: PathBuf,
    cache_dir: PathBuf,
    manifest: Option<Arc<VersionManifest>>,
    #[allow(dead_code)]
    instance_id: Option<u32>,
}

impl MinecraftLauncher {
    /// Gets the game directory path.
    pub const fn get_game_dir(&self) -> &PathBuf {
        &self.game_dir
    }

    /// Creates a new `MinecraftLauncher` instance.
    ///
    /// Initializes the launcher with proper directories and loads
    /// the version manifest from cache or downloads it fresh.
    pub async fn new(custom_game_dir: Option<PathBuf>, instance_id: Option<u32>) -> Result<Self> {
        let game_dir = get_game_dir(custom_game_dir, instance_id)?;
        let cache_dir = get_cache_dir()?;

        // Ensure all directories exist
        ensure_directories(instance_id).await?;

        let mut launcher = Self {
            downloader: Arc::new(HttpDownloader::new()?),
            java_manager: JavaManager::new().await?,
            game_dir,
            cache_dir,
            manifest: None,
            instance_id,
        };

        // Load cached manifest or fetched a new one
        if let Err(e) = launcher.load_cached_manifest().await {
            log_warn!("Failed to load cached manifest: {e}");
            launcher.update_manifest().await?;
        } else if launcher.manifest.is_none() {
            log_info!("No cached manifest found, fetching from Mojang...");
            launcher.update_manifest().await?;
        }

        Ok(launcher)
    }

    pub(crate) fn get_available_versions(&self) -> Result<&[VersionInfo]> {
        let manifest = self
            .manifest
            .as_ref()
            .ok_or_else(|| simple_error!("Version manifest not loaded"))?;

        Ok(&manifest.versions)
    }

    pub async fn update_manifest(&mut self) -> Result<()> {
        log_info!("Fetching version manifest from Mojang...");

        let manifest: VersionManifest = self
            .downloader
            .get_json(VersionManifest::MANIFEST_URL)
            .await?;

        // Cache the manifest
        let manifest_path = self.cache_dir.join("version_manifest_v2.json");
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        tokio::fs::write(&manifest_path, manifest_json).await?;

        self.manifest = Some(Arc::new(manifest));
        log_info!("Version manifest updated successfully");

        Ok(())
    }

    async fn load_cached_manifest(&mut self) -> Result<()> {
        let manifest_path = self.cache_dir.join("version_manifest_v2.json");

        if manifest_path.exists() {
            let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
            let manifest: VersionManifest = serde_json::from_str(&manifest_content)?;
            self.manifest = Some(Arc::new(manifest));
            log_debug!("Loaded cached version manifest");
        } else {
            return Err(simple_error!("No cached manifest found"));
        }

        Ok(())
    }

    pub(crate) fn is_java_available(&self, version: &str) -> bool {
        self.java_manager.is_java_available(version)
    }

    pub async fn install_java(&mut self, version: &str) -> Result<()> {
        let required_java =
            crate::backend::launcher::java::runtime::JavaRuntime::get_required_java_version(
                version,
            );

        // Try to install native Java first
        match self.java_manager.install_java_runtime(required_java).await {
            Ok(()) => Ok(()),
            Err(e) => {
                // For modern versions requiring Java 21+, try x86_64 as a fallback
                if required_java >= 21 {
                    log_warn!("Native Java {required_java} installation failed: {e}");
                    log_warn!("Attempting to install x86_64 Java {required_java} as fallback...");

                    match self
                        .java_manager
                        .install_x86_64_java_runtime(required_java)
                        .await
                    {
                        Ok(()) => {
                            log_info!(
                                "Successfully installed x86_64 Java {required_java} as fallback"
                            );
                            Ok(())
                        }
                        Err(x86_err) => {
                            log_error!("Both native and x86_64 Java installation failed");
                            log_error!("Native error: {e}");
                            log_error!("x86_64 error: {x86_err}");
                            Err(simple_error!(
                                "Failed to install Java {required_java}: native installation failed ({e}), x86_64 fallback also failed ({x86_err})"
                            ))
                        }
                    }
                } else {
                    Err(e)
                }
            }
        }
    }

    pub async fn prepare_version(&self, version_id: &str) -> Result<()> {
        log_info!("Preparing Minecraft version: {version_id}");

        // Check if a version already exists offline
        if self.is_version_ready_offline(version_id)? {
            log_info!("Version {version_id} is already prepared offline");
            return Ok(());
        }

        let version_info = self.get_version_info(version_id)?;
        let version_details = match self.get_version_details(version_info).await {
            Ok(details) => details,
            Err(e) => {
                log_warn!("Failed to download version details: {e}. Checking for offline version");
                return self.try_offline_mode(version_id);
            }
        };

        // Download client jar
        log_info!("Downloading client jar...");
        if let Err(e) = self.download_client_jar(&version_details).await {
            log_warn!("Failed to download client jar: {e}");
        }

        // Download libraries
        log_info!("Downloading libraries...");
        if let Err(e) = self.download_libraries(&version_details).await {
            log_warn!("Failed to download libraries: {e}");
        }

        // Download and extracting native libraries
        log_info!("Downloading and extracting native libraries...");
        if let Err(e) = self.download_natives(&version_details).await {
            log_warn!("Failed to download natives: {e}");
        }

        // Download assets
        log_info!("Downloading assets...");
        if let Err(e) = self.download_assets(&version_details).await {
            log_warn!("Failed to download assets: {e}");
        }

        // Try to verify installation, but don't fail if some files are missing
        match self.verify_installation(&version_details).await {
            Ok(()) => log_info!("Version {version_id} prepared successfully"),
            Err(e) => {
                log_warn!("Installation verification failed: {e}. Proceeding anyway");
                log_info!("Version {version_id} prepared with warnings");
            }
        }

        Ok(())
    }

    pub async fn launch(&mut self, version_id: &str) -> Result<()> {
        log_info!("Launching Minecraft version: {version_id}");

        // System diagnostics
        self.log_system_info();

        let version_info = self.get_version_info(version_id)?;
        let version_type = version_info.version_type.clone();
        let version_details = self.get_version_details(version_info).await?;

        log_info!(
            "Version details loaded: main_class = {}",
            version_details.main_class
        );
        log_info!("Assets index: {}", version_details.assets);

        // Get Java runtime
        let (java_path, use_rosetta) = self.java_manager.get_java_for_version(version_id).await?;
        log_info!("Using Java: {java_path:?} (Rosetta: {use_rosetta})");

        // Verify Java executable exists
        if !java_path.exists() {
            return Err(simple_error!("Java executable not found at: {java_path:?}"));
        }

        // Test the Java version and get a major version
        Self::test_java_version(&java_path)?;
        let java_major_version = Self::get_java_major_version(&java_path)?;

        // Build library paths
        let libraries = self.get_library_paths(&version_details);
        log_info!("Loaded {} libraries", libraries.len());

        // Verify critical files exist
        self.verify_game_files(version_id, &libraries)?;

        // Build command
        let minecraft_cmd = CommandBuilder::new()
            .java_path(java_path.clone())
            .game_dir(self.game_dir.clone())
            .version_details(version_details.clone())
            .username("Player".to_string())
            .uuid("00000000-0000-0000-0000-000000000000".to_string())
            .access_token("null".to_string())
            .user_type("mojang".to_string())
            .version_type(version_type)
            .assets_dir(get_assets_dir()?)
            .libraries(libraries)
            .main_jar(get_version_jar_path(&self.game_dir, version_id))
            .java_major_version(java_major_version)
            .use_rosetta(use_rosetta)
            .build()?;

        let mut cmd = minecraft_cmd.build()?;

        println!("Starting Minecraft...");
        log_info!("Full command: {cmd:?}");

        // Add macOS window debugging
        if cfg!(target_os = "macos") && use_rosetta {
            log_info!("System: macOS ARM64 with Rosetta 2");
            log_info!("Java path: {java_path:?}");
            log_info!("Use Rosetta: {use_rosetta}");
            log_info!("Version ID: {version_id}");

            // Check if we can get window manager info
            let _ = std::process::Command::new("defaults")
                .args(["read", "com.apple.dock", "orientation"])
                .output()
                .map(|output| {
                    log_info!(
                        "Dock orientation: {:?}",
                        String::from_utf8_lossy(&output.stdout)
                    );
                });

            // Check current display info
            let _ = std::process::Command::new("system_profiler")
                .args(["SPDisplaysDataType", "-json"])
                .output()
                .map(|output| {
                    if output.status.success() {
                        log_info!("Display info available");
                    }
                });
        }

        // Launch the game with proper logging
        let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

        log_info!("Minecraft process started with PID: {}", child.id());

        // Add window debugging for macOS
        if cfg!(target_os = "macos") {
            // Wait a moment for the process to initialize
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // Check if the Java process is running and what windows it has
            let pid = child.id();
            let _ = std::process::Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "pid,ppid,state,comm"])
                .output()
                .map(|output| {
                    log_info!("Process info: {}", String::from_utf8_lossy(&output.stdout));
                });

            // Check for Java windows using system tools
            let _ = std::process::Command::new("osascript")
                .args(["-e", "tell application \"System Events\" to get name of every process whose name contains \"java\""])
                .output()
                .map(|output| {
                    log_info!("Java processes: {}", String::from_utf8_lossy(&output.stdout));
                });

            // Try to find the Minecraft window
            let _ = std::process::Command::new("osascript")
                .args(["-e", "tell application \"System Events\" to get name of every window of every process"])
                .output()
                .map(|output| {
                    let windows = String::from_utf8_lossy(&output.stdout);
                    if windows.contains("Minecraft") {
                        log_info!("Found Minecraft window in system");
                    } else {
                        log_info!("No Minecraft window found in system windows");
                    }
                });
        }

        // Monitor the process
        if let Some(stdout) = child.stdout.take() {
            let reader = std::io::BufReader::new(stdout);
            tokio::spawn(async move {
                use std::io::BufRead;
                for line in reader.lines().map_while(std::result::Result::ok) {
                    if line.contains("ERROR") || line.contains("FATAL") {
                        log_error!("MC: {line}");
                    } else if line.contains("WARN") {
                        log_warn!("MC: {line}");
                    } else {
                        log_debug!("MC: {line}");
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let reader = std::io::BufReader::new(stderr);
            tokio::spawn(async move {
                use std::io::BufRead;
                for line in reader.lines().map_while(std::result::Result::ok) {
                    // Look for specific macOS / window-related errors
                    if line.contains("NSWindow")
                        || line.contains("display")
                        || line.contains("OpenGL")
                        || line.contains("LWJGL")
                    {
                        log_error!("MC Window Error: {line}");
                    } else {
                        log_error!("MC Error: {line}");
                    }
                }
            });
        }

        // Wait for the process to complete with timeout using spawn_blocking
        let pid = child.id();
        let status = tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            tokio::task::spawn_blocking(move || child.wait()),
        )
        .await;

        let status = match status {
            Ok(result) => result.map_err(|e| simple_error!("Join error: {}", e))??,
            Err(_) => {
                log_info!(
                    "Minecraft process still running after 30 seconds, considering it successful"
                );

                if cfg!(target_os = "macos") {
                    let _ = std::process::Command::new("osascript")
                        .args([
                            "-e",
                            &format!(
                                "tell application \"System Events\" to get windows of process {pid}"
                            ),
                        ])
                        .output()
                        .map(|output| {
                            log_info!(
                                "Windows of PID {}: {}",
                                pid,
                                String::from_utf8_lossy(&output.stdout)
                            );
                        });

                    // Try to bring any Java windows to the front
                    let _ = std::process::Command::new("osascript")
                        .args(["-e", "tell application \"System Events\" to set frontmost of first process whose name contains \"java\" to true"])
                        .output()
                        .map(|output| {
                            log_info!("Attempted to bring Java to front: {}", String::from_utf8_lossy(&output.stdout));
                        });
                }

                return Ok(());
            }
        };

        if status.success() {
            println!("Minecraft exited successfully");
        } else {
            log_error!("Minecraft exited with code: {:?}", status.code());
            return Err(simple_error!(
                "Minecraft process failed with code: {:?}",
                status.code()
            ));
        }

        Ok(())
    }

    fn get_version_info(&self, version_id: &str) -> Result<&VersionInfo> {
        let manifest = self
            .manifest
            .as_ref()
            .ok_or_else(|| simple_error!("Version manifest not loaded"))?;

        manifest
            .get_version(version_id)
            .ok_or_else(|| simple_error!("Version {version_id} not found"))
    }

    async fn get_version_details(&self, version_info: &VersionInfo) -> Result<VersionDetails> {
        let version_json_path = get_version_json_path(&self.game_dir, &version_info.id);

        // Check if we have cached version details
        if version_json_path.exists() {
            let content = tokio::fs::read_to_string(&version_json_path).await?;
            if let Ok(details) = serde_json::from_str::<VersionDetails>(&content) {
                return Ok(details);
            }
        }

        // Download version details
        log_info!("Downloading version details for {}", version_info.id);
        let details: VersionDetails = self.downloader.get_json(&version_info.url).await?;

        // Cache the details
        ensure_parent_directory(&version_json_path).await?;
        let details_json = serde_json::to_string_pretty(&details)?;
        tokio::fs::write(&version_json_path, details_json).await?;

        Ok(details)
    }

    async fn download_client_jar(&self, version_details: &VersionDetails) -> Result<()> {
        if let Some(client) = &version_details.downloads.client {
            let jar_path = get_version_jar_path(&self.game_dir, &version_details.id);

            if !verify_file(&jar_path, Some(client.size), Some(&client.sha1)).await? {
                log_info!("Downloading client jar for {}", version_details.id);

                self.downloader
                    .download_file(&client.url, &jar_path, Some(&client.sha1))
                    .await?;
            }
        }

        Ok(())
    }

    async fn download_libraries(&self, version_details: &VersionDetails) -> Result<()> {
        let os_name = get_minecraft_os_name();
        let os_arch = get_minecraft_arch();
        let os_features = get_os_features();

        // Download libraries
        let mut download_tasks = Vec::new();

        for library in &version_details.libraries {
            // Check if this library should be used on this platform
            if !library.should_use(os_name, os_arch, &os_features) {
                continue;
            }

            // Download the main artifact
            if let Some(artifact) = &library.downloads.artifact
                && let Some(path) = &artifact.path
            {
                let lib_path = get_library_path(&self.game_dir, path);

                if !verify_file(&lib_path, Some(artifact.size), Some(&artifact.sha1)).await? {
                    download_tasks.push(
                        DownloadTask::new(artifact.url.clone(), lib_path)
                            .with_sha1(artifact.sha1.clone()),
                    );
                }
            }
        }

        if !download_tasks.is_empty() {
            println!("Downloading {} libraries...", download_tasks.len());
            self.downloader.download_multiple(download_tasks, 8).await?;
        }

        Ok(())
    }

    async fn download_natives(&self, version_details: &VersionDetails) -> Result<()> {
        let os_name = get_minecraft_os_name();
        let os_arch = get_minecraft_arch();
        let os_features = get_os_features();
        let native_classifiers = get_all_native_classifiers();

        log_info!(
            "Starting native libraries download for version {}",
            version_details.id
        );
        log_info!("OS: {os_name}, Arch: {os_arch}, Features: {os_features:?}");
        log_info!("Native classifiers for this platform: {native_classifiers:?}");

        let natives_dir = get_natives_dir(&self.game_dir, &version_details.id);
        ensure_directory(&natives_dir).await?;
        log_info!("Created natives directory: {natives_dir:?}");

        // Check if natives directory already has files
        let existing_files = std::fs::read_dir(&natives_dir)?.count();
        log_info!("Existing files in natives directory: {existing_files}");

        let mut download_tasks = Vec::new();
        let mut native_libraries = Vec::new();

        for library in &version_details.libraries {
            let should_use = library.should_use(os_name, os_arch, &os_features);
            log_debug!(
                "Library {}: should_use={}, has_rules={}, has_classifiers={}",
                library.name,
                should_use,
                library.rules.is_some(),
                library.downloads.classifiers.is_some()
            );

            if let Some(rules) = &library.rules {
                for rule in rules {
                    let matches = rule.matches(os_name, os_arch, &os_features);
                    log_debug!(
                        "  Rule action={}, matches={}, os={:?}, arch={:?}",
                        rule.action,
                        matches,
                        rule.os,
                        os_arch
                    );
                }
            }

            if !should_use {
                log_debug!("Skipping library {} (not for this platform)", library.name);
                continue;
            }

            if let Some(classifiers) = &library.downloads.classifiers {
                // Try all possible native classifiers (with fallback)
                let mut found_native = false;
                for classifier in &native_classifiers {
                    if let Some(native_artifact) = classifiers.get(classifier)
                        && let Some(path) = &native_artifact.path
                    {
                        let native_path = get_library_path(&self.game_dir, path);
                        native_libraries.push((native_path.clone(), library.clone()));

                        if !verify_file(
                            &native_path,
                            Some(native_artifact.size),
                            Some(&native_artifact.sha1),
                        )
                        .await?
                        {
                            log_info!(
                                "Need to download native ({}): {} -> {:?}",
                                classifier,
                                native_artifact.url,
                                native_path
                            );
                            download_tasks.push(
                                DownloadTask::new(native_artifact.url.clone(), native_path)
                                    .with_sha1(native_artifact.sha1.clone()),
                            );
                        } else {
                            log_info!("Native already exists ({classifier}): {native_path:?}");
                        }
                        found_native = true;
                        break; // Use the first matching classifier
                    }
                }

                if !found_native {
                    if let Some(classifiers) = &library.downloads.classifiers {
                        let available_classifiers: Vec<String> =
                            classifiers.keys().cloned().collect();
                        log_debug!(
                            "No native artifacts found for library {} with classifiers {:?}. Available classifiers: {:?}",
                            library.name,
                            native_classifiers,
                            available_classifiers
                        );
                    } else {
                        log_debug!(
                            "No native artifacts found for library {} with classifiers {:?}",
                            library.name,
                            native_classifiers
                        );
                    }
                }
            }
        }

        log_info!(
            "Found {} native libraries for platform {:?}",
            native_libraries.len(),
            native_classifiers
        );

        if !download_tasks.is_empty() {
            println!("Downloading {} native libraries...", download_tasks.len());
            self.downloader.download_multiple(download_tasks, 8).await?;
        }

        // Always try to extract natives (even if already downloaded)
        println!("Extracting native libraries...");
        self.extract_natives(version_details).await?;

        // Verify extraction
        let extracted_count = std::fs::read_dir(&natives_dir)?.count();
        log_info!("Extracted {extracted_count} files to natives directory");

        if extracted_count == 0 {
            log_error!("CRITICAL: No files were extracted to natives directory!");
            log_error!("This will cause 'no lwjgl in java.library.path' error");

            log_info!("Available native libraries to extract:");
            for (native_path, library) in &native_libraries {
                log_info!("  - Library: {}", library.name);
                log_info!("    Path: {native_path:?}");
                log_info!("    Exists: {}", native_path.exists());
                if native_path.exists() {
                    let size = std::fs::metadata(native_path)?.len();
                    log_info!("    Size: {size} bytes");
                }
            }

            // Force re-extraction
            for (native_path, library) in &native_libraries {
                if native_path.exists() {
                    log_warn!("Force extracting: {native_path:?}");
                    match self
                        .extract_native_library(native_path, &natives_dir, &library.extract)
                        .await
                    {
                        Ok(()) => log_info!("Successfully extracted {native_path:?}"),
                        Err(e) => log_error!("Failed to extract {native_path:?}: {e}"),
                    }
                }
            }

            // Re-check after force extraction
            let final_count = std::fs::read_dir(&natives_dir)?.count();
            if final_count == 0 {
                return Err(simple_error!(
                    "Failed to extract any native libraries to {natives_dir:?}. This will cause LWJGL errors."
                ));
            }
            log_info!("After force extraction: {final_count} files in natives directory");
        } else {
            // List extracted files for verification
            log_info!("Extracted native files:");
            for entry in std::fs::read_dir(&natives_dir)?.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    let size = std::fs::metadata(&path)?.len();
                    log_info!("  - {name} ({size} bytes)");
                }
            }
        }

        Ok(())
    }

    fn create_dylib_symlinks(natives_dir: &PathBuf) -> Result<()> {
        // On macOS, old LWJGL versions (2.x) use .jnilib extension, but modern Java expects .dylib
        // Create symlinks from .dylib to .jnilib files for compatibility
        if cfg!(target_os = "macos") {
            log_info!(
                "Creating .dylib symlinks for .jnilib files in {:?}",
                natives_dir
            );

            let entries = std::fs::read_dir(natives_dir)?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if let Some(file_name) = path.file_name() {
                    let file_name_str = file_name.to_string_lossy();

                    // If it's a .jnilib file, create a .dylib symlink
                    if file_name_str.ends_with(".jnilib") {
                        let dylib_name = file_name_str.replace(".jnilib", ".dylib");
                        let dylib_path = natives_dir.join(&dylib_name);

                        // Only create symlink if .dylib doesn't already exist
                        if !dylib_path.exists() {
                            log_info!("Creating symlink: {dylib_name} -> {file_name_str}");

                            #[cfg(unix)]
                            {
                                std::os::unix::fs::symlink(file_name_str.as_ref(), &dylib_path)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn extract_natives(&self, version_details: &VersionDetails) -> Result<()> {
        let os_name = get_minecraft_os_name();
        let os_arch = get_minecraft_arch();
        let os_features = get_os_features();
        let natives_dir = get_natives_dir(&self.game_dir, &version_details.id);
        let native_classifiers = get_all_native_classifiers();

        log_info!("Extracting natives to: {:?}", natives_dir);
        log_info!(
            "Total libraries to check: {}",
            version_details.libraries.len()
        );

        let mut processed_count = 0;
        let mut extracted_count = 0;

        for library in &version_details.libraries {
            processed_count += 1;

            if !library.should_use(os_name, os_arch, &os_features) {
                log_debug!("Skipping library {} (not for this platform)", library.name);
                continue;
            }

            if let Some(classifiers) = &library.downloads.classifiers {
                // Try all possible native classifiers (with fallback)
                let mut found_and_extracted = false;
                for classifier in &native_classifiers {
                    if let Some(native_artifact) = classifiers.get(classifier)
                        && let Some(path) = &native_artifact.path
                    {
                        let native_path = get_library_path(&self.game_dir, path);

                        if native_path.exists() {
                            log_info!(
                                "Extracting native library ({}) for {}: {:?}",
                                classifier,
                                library.name,
                                native_path
                            );
                            match self
                                .extract_native_library(
                                    &native_path,
                                    &natives_dir,
                                    &library.extract,
                                )
                                .await
                            {
                                Ok(()) => {
                                    extracted_count += 1;
                                    found_and_extracted = true;
                                    log_info!(
                                        "Successfully extracted native library for {}",
                                        library.name
                                    );
                                }
                                Err(e) => {
                                    log_error!(
                                        "Failed to extract native library for {}: {}",
                                        library.name,
                                        e
                                    );
                                }
                            }
                            break; // Only extract the first matching classifier
                        }
                        log_debug!(
                            "Native library not found ({}) for {}: {:?}",
                            classifier,
                            library.name,
                            native_path
                        );
                    }
                }

                if !found_and_extracted {
                    log_debug!("No native library extracted for {}", library.name);
                }
            } else {
                log_debug!("Library {} has no classifiers", library.name);
            }
        }

        log_info!(
            "Native extraction summary: processed {processed_count} libraries, extracted {extracted_count} native libraries"
        );

        // Create .dylib symlinks for .jnilib files on macOS
        Self::create_dylib_symlinks(&natives_dir)?;

        Ok(())
    }

    async fn extract_native_library(
        &self,
        archive_path: &PathBuf,
        extract_dir: &PathBuf,
        extract_rules: &Option<crate::backend::launcher::models::ExtractRules>,
    ) -> Result<()> {
        log_info!(
            "Extracting native archive: {:?} to {:?}",
            archive_path,
            extract_dir
        );

        if !archive_path.exists() {
            return Err(simple_error!("Archive does not exist: {:?}", archive_path));
        }

        let archive_size = std::fs::metadata(archive_path)?.len();
        log_info!("Archive size: {archive_size} bytes");

        // Create an extract directory
        tokio::fs::create_dir_all(extract_dir).await?;

        // Use our custom archive extraction
        crate::utils::extract_zip(archive_path, extract_dir)
            .await
            .map_err(|e| {
                simple_error!(
                    "Failed to extract native library archive {:?}: {}",
                    archive_path,
                    e
                )
            })?;

        // If we have excluded rules, we need to clean up excluded files after extraction
        if let Some(rules) = extract_rules {
            if let Some(exclude) = &rules.exclude {
                log_debug!("Processing exclude rules for native library extraction");

                // Walk through extracted files and remove those matching exclude patterns
                let mut entries = tokio::fs::read_dir(extract_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    let relative_path = path.strip_prefix(extract_dir).unwrap_or(&path);
                    let path_str = relative_path.to_string_lossy();

                    for pattern in exclude {
                        if path_str.contains(pattern.as_str()) {
                            log_debug!("Removing excluded file: {:?} (pattern: {})", path, pattern);
                            if path.is_file() {
                                let _ = tokio::fs::remove_file(&path).await;
                            } else if path.is_dir() {
                                let _ = tokio::fs::remove_dir_all(&path).await;
                            }
                            break;
                        }
                    }
                }
            }
        }

        log_info!("Native library extraction completed for {:?}", archive_path);
        Ok(())
    }

    async fn download_assets(&self, version_details: &VersionDetails) -> Result<()> {
        let asset_index_path =
            get_asset_indexes_dir()?.join(format!("{}.json", version_details.asset_index.id));

        // Download asset index
        if !verify_file(
            &asset_index_path,
            Some(version_details.asset_index.size),
            Some(&version_details.asset_index.sha1),
        )
        .await?
        {
            println!(
                "Downloading asset index: {}",
                version_details.asset_index.id
            );

            self.downloader
                .download_file(
                    &version_details.asset_index.url,
                    &asset_index_path,
                    Some(&version_details.asset_index.sha1),
                )
                .await?;
        }

        // Parse asset index
        let asset_content = tokio::fs::read_to_string(&asset_index_path).await?;
        let asset_manifest: AssetManifest = serde_json::from_str(&asset_content)?;

        // Download assets
        let mut download_tasks = Vec::new();
        let mut assets_for_virtual = Vec::new();

        for (name, asset) in asset_manifest.objects {
            let asset_path = get_asset_path(&asset.hash)?;

            if !verify_file(&asset_path, Some(asset.size), Some(&asset.hash)).await? {
                let url = format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    &asset.hash[..2],
                    asset.hash
                );

                download_tasks
                    .push(DownloadTask::new(url, asset_path).with_sha1(asset.hash.clone()));
            }

            // Store asset info for virtual assets creation (use references to avoid cloning)
            assets_for_virtual.push((name.clone(), asset.clone()));
        }

        if !download_tasks.is_empty() {
            log_info!("Downloading {} assets...", download_tasks.len());

            // Show progress for assets
            let total_assets = download_tasks.len();
            let mut completed = 0;

            // Download in smaller batches to show progress
            for chunk in download_tasks.chunks(100) {
                self.downloader
                    .download_multiple(chunk.to_vec(), 16)
                    .await?;
                completed += chunk.len();

                let progress = (completed as f64 / total_assets as f64 * 100.0).round() as u8;
                log_info!("Assets: {progress}% ({completed}/{total_assets})");
            }
        }

        // Create virtual assets for versions that need them
        self.create_virtual_assets(version_details, &assets_for_virtual)
            .await?;

        Ok(())
    }

    async fn create_virtual_assets(
        &self,
        version_details: &VersionDetails,
        assets: &[(String, AssetObject)],
    ) -> Result<()> {
        // Check if we need to create virtual assets
        let needs_virtual = match version_details.assets.as_str() {
            "legacy" => true,  // Legacy versions need virtual assets in legacy format
            "pre-1.6" => true, // Pre-1.6 versions need virtual assets
            _ => {
                // Parse version to determine if virtual assets are needed
                // Most versions from 1.7+ need virtual assets, but we'll be more generous
                let version_parts: Vec<&str> = version_details.id.split('.').collect();
                if version_parts.len() >= 2 {
                    if let (Ok(major), Ok(minor)) = (
                        version_parts[0].parse::<i32>(),
                        version_parts[1].parse::<i32>(),
                    ) {
                        major > 1 || (major == 1 && minor >= 6)
                    } else {
                        true // If we can't parse, assume we need virtual assets
                    }
                } else {
                    true // If a version format is unexpected, assume we need virtual assets
                }
            }
        };

        if !needs_virtual {
            log_info!("Version {} doesn't need virtual assets", version_details.id);
            return Ok(());
        }

        log_info!(
            "Version {} needs virtual assets for index '{}'",
            version_details.id,
            version_details.assets
        );

        let virtual_dir = get_assets_dir()?
            .join("virtual")
            .join(&version_details.assets);

        // Create virtual directory
        ensure_directory(&virtual_dir).await?;

        log_info!(
            "Creating virtual assets for {} at {:?}",
            version_details.assets,
            virtual_dir
        );
        log_info!("Processing {} assets", assets.len());

        let mut created_count = 0;
        for (name, asset) in assets {
            // For legacy and pre-1.6 versions, use the original asset structure
            let virtual_path =
                if version_details.assets == "legacy" || version_details.assets == "pre-1.6" {
                    // Legacy versions expect assets in resources/ structure
                    let legacy_name = if name.starts_with("minecraft/") {
                        name.strip_prefix("minecraft/").unwrap_or(name)
                    } else {
                        name
                    };
                    virtual_dir.join("resources").join(legacy_name)
                } else {
                    virtual_dir.join(name)
                };

            let asset_path = get_asset_path(&asset.hash)?;

            // Create a parent directory if needed
            ensure_parent_directory(&virtual_path).await?;

            // Copy or link the asset to a virtual location if it doesn't exist
            if !virtual_path.exists() && asset_path.exists() {
                match tokio::fs::copy(&asset_path, &virtual_path).await {
                    Ok(_) => {
                        created_count += 1;
                        if created_count <= 5 {
                            log_debug!("Created virtual asset: {name}");
                        }
                    }
                    Err(e) => {
                        log_warn!("Failed to create virtual asset {name}: {e}");
                    }
                }
            } else if !asset_path.exists() {
                log_warn!("Source asset file missing for {name}: {asset_path:?}");
            }
        }

        if created_count > 0 {
            log_info!("Created {created_count} virtual assets");
        }

        Ok(())
    }

    async fn verify_installation(&self, version_details: &VersionDetails) -> Result<()> {
        log_info!("Verifying installation for version {}", version_details.id);

        // Check the main jar
        let main_jar = get_version_jar_path(&self.game_dir, &version_details.id);
        if !main_jar.exists() {
            log_warn!("Main jar missing: {main_jar:?}");
            return Err(simple_error!("Main jar missing: {main_jar:?}"));
        }

        // Check natives directory - but allow empty for some versions
        let natives_dir = get_natives_dir(&self.game_dir, &version_details.id);
        if !natives_dir.exists() {
            log_warn!("Natives directory missing: {natives_dir:?}");
            // Try to create an empty natives directory
            ensure_directory(&natives_dir).await?;
            log_info!("Created empty natives directory");
        }

        let native_count = std::fs::read_dir(&natives_dir)?.count();
        if native_count == 0 {
            log_warn!("Natives directory is empty: {natives_dir:?}");
            log_info!("Some versions may work without native libraries");
        } else {
            log_info!("Installation verified, {native_count} native files found");
        }

        Ok(())
    }

    fn is_version_ready_offline(&self, version_id: &str) -> Result<bool> {
        let version_dir = self.game_dir.join("versions").join(version_id);
        let jar_file = version_dir.join(format!("{version_id}.jar"));
        let json_file = version_dir.join(format!("{version_id}.json"));

        Ok(version_dir.exists() && jar_file.exists() && json_file.exists())
    }

    fn try_offline_mode(&self, version_id: &str) -> Result<()> {
        log_info!("Attempting to use offline mode for version {version_id}");

        if self.is_version_ready_offline(version_id)? {
            log_info!("Version {version_id} found offline, skipping downloads");
            return Ok(());
        }

        // Try to copy from a similar version if available
        let versions_dir = self.game_dir.join("versions");
        if let Ok(entries) = std::fs::read_dir(&versions_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str()
                    && name != version_id
                    && entry.path().is_dir()
                {
                    log_info!("Found existing version: {name}, you can try launching that instead",);
                }
            }
        }

        Err(simple_error!(
            "Version {version_id} not available offline. Available versions in {versions_dir:?}. Try running the official Minecraft launcher first to download the version."
        ))
    }

    fn log_system_info(&self) {
        use std::process::Command;

        log_info!("=== System Diagnostics ===");

        // Memory info
        if let Ok(output) = Command::new("free").arg("-h").output()
            && let Ok(memory_info) = String::from_utf8(output.stdout)
        {
            log_info!("Memory info:\n{memory_info}");
        }

        // Java processes
        if let Ok(output) = Command::new("ps").args(["aux"]).output()
            && let Ok(ps_output) = String::from_utf8(output.stdout)
        {
            let java_processes: Vec<&str> = ps_output
                .lines()
                .filter(|line| line.contains("java") || line.contains("minecraft"))
                .collect();
            if !java_processes.is_empty() {
                log_warn!("Existing Java/Minecraft processes found:");
                for process in java_processes {
                    log_warn!("  {process}");
                }
            }
        }

        log_info!("Game directory: {:?}", self.game_dir);
        log_info!("Cache directory: {:?}", self.cache_dir);
    }

    fn test_java_version(java_path: &PathBuf) -> Result<()> {
        use std::process::Command;

        log_info!("Testing Java installation...");

        let output = Command::new(java_path)
            .args(["-version"])
            .output()
            .map_err(|e| simple_error!("Failed to run Java: {}", e))?;

        let version_info = String::from_utf8_lossy(&output.stderr);
        log_info!("Java version info: {version_info}");

        // Extract a major version for better compatibility checking
        let major_version = Self::get_java_major_version(java_path).unwrap_or(8);

        // Provide version-specific guidance
        if major_version >= 24 {
            log_warn!(
                "You're using Java {major_version} which is very new. For optimal compatibility with Minecraft, consider using Java 21"
            );
        } else if major_version >= 22 {
            log_info!("Using Java {major_version}");
        } else if major_version == 21 {
            log_info!("Using Java 21");
        }

        Ok(())
    }

    fn get_java_major_version(java_path: &PathBuf) -> Result<u8> {
        use std::process::Command;

        let output = Command::new(java_path)
            .args(["-version"])
            .output()
            .map_err(|e| simple_error!("Failed to run Java: {}", e))?;

        let version_info = String::from_utf8_lossy(&output.stderr);

        if version_info.contains("\"1.8.") {
            Ok(8)
        } else if version_info.contains("\"17.") {
            Ok(17)
        } else if version_info.contains("\"21.") {
            Ok(21)
        } else if version_info.contains("\"11.") {
            Ok(11)
        } else if version_info.contains("\"16.") {
            Ok(16)
        } else {
            for line in version_info.lines() {
                if line.contains("version")
                    && let Some(start) = line.find('"')
                    && let Some(end) = line[start + 1..].find('"')
                {
                    let version_str = &line[start + 1..start + 1 + end];
                    let parts: Vec<&str> = version_str.split('.').collect();
                    if let Some(first_part) = parts.first()
                        && let Ok(major) = first_part.parse::<u8>()
                    {
                        return Ok(major);
                    }
                }
            }
            log_warn!("Could not determine Java major version, defaulting to 8");
            Ok(8)
        }
    }

    fn verify_game_files(&self, version_id: &str, libraries: &[PathBuf]) -> Result<()> {
        log_info!("Verifying game files...");

        // Check main .jar
        let main_jar = get_version_jar_path(&self.game_dir, version_id);
        if !main_jar.exists() {
            return Err(simple_error!("Main jar not found: {main_jar:?}"));
        }
        log_info!("Main jar exists: {main_jar:?}");

        // Check libraries
        let mut missing_libs = Vec::new();
        for lib in libraries {
            if !lib.exists() {
                missing_libs.push(lib.clone());
            }
        }

        if !missing_libs.is_empty() {
            log_warn!("Missing {} libraries:", missing_libs.len());
            for lib in &missing_libs {
                log_warn!("  Missing: {lib:?}");
            }
            return Err(simple_error!("Missing required libraries"));
        }

        log_info!("All {} libraries verified", libraries.len());

        // Check natives directory
        let natives_dir = get_natives_dir(&self.game_dir, version_id);
        if !natives_dir.exists() {
            log_warn!("Natives directory doesn't exist: {natives_dir:?}");
        } else {
            let natives_count = std::fs::read_dir(&natives_dir)?.count();
            log_info!("Natives directory contains {natives_count} files");
        }

        Ok(())
    }

    fn get_library_paths(&self, version_details: &VersionDetails) -> Vec<PathBuf> {
        let os_name = get_minecraft_os_name();
        let os_arch = get_minecraft_arch();
        let os_features = get_os_features();

        let mut library_paths = Vec::new();

        for library in &version_details.libraries {
            if !library.should_use(os_name, os_arch, &os_features) {
                continue;
            }

            if let Some(artifact) = &library.downloads.artifact
                && let Some(path) = &artifact.path
            {
                let lib_path = get_library_path(&self.game_dir, path);
                library_paths.push(lib_path);
            }
        }

        library_paths
    }
}
