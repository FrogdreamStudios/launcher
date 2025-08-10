use anyhow::Result;
use std::path::PathBuf;
use std::process::Stdio;
use tracing::{debug, error, info, warn};

use super::downloader::{HttpDownloader, ProgressTracker};
use super::java::JavaManager;
use super::models::{AssetManifest, AssetObject, VersionDetails, VersionInfo, VersionManifest};
use crate::backend::creeper::downloader::models::DownloadTask;
use crate::backend::utils::command::CommandBuilder;
use crate::backend::utils::file_utils::{ensure_directory, ensure_parent_directory, verify_file};
use crate::backend::utils::os::{
    get_all_native_classifiers, get_minecraft_arch, get_minecraft_os_name, get_os_features,
};
use crate::backend::utils::paths::*;

pub struct MinecraftLauncher {
    downloader: HttpDownloader,
    java_manager: JavaManager,
    game_dir: PathBuf,
    cache_dir: PathBuf,
    manifest: Option<VersionManifest>,
}

impl MinecraftLauncher {
    pub fn get_game_dir(&self) -> &PathBuf {
        &self.game_dir
    }

    pub async fn new(custom_game_dir: Option<PathBuf>) -> Result<Self> {
        let game_dir = get_game_dir(custom_game_dir)?;
        let cache_dir = get_cache_dir()?;

        // Ensure all directories exist
        ensure_directories(&game_dir).await?;

        let mut launcher = Self {
            downloader: HttpDownloader::new()?,
            java_manager: JavaManager::new().await?,
            game_dir,
            cache_dir,
            manifest: None,
        };

        // Load cached manifest or fetch new one
        if let Err(e) = launcher.load_cached_manifest().await {
            warn!("Failed to load cached manifest: {e}");
            launcher.update_manifest().await?;
        } else if launcher.manifest.is_none() {
            info!("No cached manifest found, fetching from Mojang...");
            launcher.update_manifest().await?;
        }

        Ok(launcher)
    }

    pub(crate) async fn get_available_versions(&self) -> Result<Vec<VersionInfo>> {
        let manifest = self
            .manifest
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Version manifest not loaded"))?;

        Ok(manifest.versions.clone())
    }

    pub async fn update_manifest(&mut self) -> Result<()> {
        info!("Fetching version manifest from Mojang...");

        let manifest: VersionManifest = self
            .downloader
            .get_json(VersionManifest::MANIFEST_URL)
            .await?;

        // Cache the manifest
        let manifest_path = self.cache_dir.join("version_manifest_v2.json");
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        tokio::fs::write(&manifest_path, manifest_json).await?;

        self.manifest = Some(manifest);
        info!("Version manifest updated successfully");

        Ok(())
    }

    async fn load_cached_manifest(&mut self) -> Result<()> {
        let manifest_path = self.cache_dir.join("version_manifest_v2.json");

        if manifest_path.exists() {
            let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
            let manifest: VersionManifest = serde_json::from_str(&manifest_content)?;
            self.manifest = Some(manifest);
            debug!("Loaded cached version manifest");
        } else {
            return Err(anyhow::anyhow!("No cached manifest found"));
        }

        Ok(())
    }

    pub(crate) async fn is_java_available(&self, version: &str) -> Result<bool> {
        Ok(self.java_manager.is_java_available(version))
    }

    pub async fn install_java(&mut self, version: &str) -> Result<()> {
        let required_java =
            crate::backend::creeper::java::runtime::JavaRuntime::get_required_java_version(version);

        // Try to install native Java first
        match self.java_manager.install_java_runtime(required_java).await {
            Ok(()) => Ok(()),
            Err(e) => {
                // For modern versions requiring Java 21+, try x86_64 as fallback
                if required_java >= 21 {
                    warn!("Native Java {required_java} installation failed: {e}");
                    warn!("Attempting to install x86_64 Java {required_java} as fallback...");

                    match self
                        .java_manager
                        .install_x86_64_java_runtime(required_java)
                        .await
                    {
                        Ok(()) => {
                            info!("Successfully installed x86_64 Java {required_java} as fallback");
                            Ok(())
                        }
                        Err(x86_err) => {
                            error!("Both native and x86_64 Java installation failed");
                            error!("Native error: {e}");
                            error!("x86_64 error: {x86_err}");
                            Err(anyhow::anyhow!(
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
        info!("Preparing Minecraft version: {version_id}");

        // Check if version already exists offline
        if self.is_version_ready_offline(version_id)? {
            info!("Version {version_id} is already prepared offline");
            return Ok(());
        }

        let version_info = self.get_version_info(version_id)?;
        let version_details = match self.get_version_details(version_info).await {
            Ok(details) => details,
            Err(e) => {
                warn!("Failed to download version details: {e}. Checking for offline version");
                return self.try_offline_mode(version_id).await;
            }
        };

        // Download client jar
        info!("Downloading client jar...");
        if let Err(e) = self.download_client_jar(&version_details).await {
            warn!("Failed to download client jar: {e}");
        }

        // Download libraries
        info!("Downloading libraries...");
        if let Err(e) = self.download_libraries(&version_details).await {
            warn!("Failed to download libraries: {e}");
        }

        // Download and extracting native libraries
        info!("Downloading and extracting native libraries...");
        if let Err(e) = self.download_natives(&version_details).await {
            warn!("Failed to download natives: {e}");
        }

        // Download assets
        info!("Downloading assets...");
        if let Err(e) = self.download_assets(&version_details).await {
            warn!("Failed to download assets: {e}");
        }

        // Try to verify installation, but don't fail if some files are missing
        match self.verify_installation(&version_details).await {
            Ok(_) => info!("Version {version_id} prepared successfully"),
            Err(e) => {
                warn!("Installation verification failed: {e}. Proceeding anyway");
                info!("Version {version_id} prepared with warnings");
            }
        }

        Ok(())
    }

    pub async fn launch(&mut self, version_id: &str) -> Result<()> {
        info!("Launching Minecraft version: {version_id}");

        // System diagnostics
        self.log_system_info()?;

        let version_info = self.get_version_info(version_id)?;
        let version_type = version_info.version_type.clone();
        let version_details = self.get_version_details(version_info).await?;

        info!(
            "Version details loaded: main_class = {}",
            version_details.main_class
        );
        info!("Assets index: {}", version_details.assets);

        // Get Java runtime
        let (java_path, use_rosetta) = self.java_manager.get_java_for_version(version_id).await?;
        info!("Using Java: {java_path:?} (Rosetta: {use_rosetta})");

        // Verify Java executable exists
        if !java_path.exists() {
            return Err(anyhow::anyhow!(
                "Java executable not found at: {java_path:?}"
            ));
        }

        // Test Java version and get major version
        self.test_java_version(&java_path)?;
        let java_major_version = self.get_java_major_version(&java_path)?;

        // Build library paths
        let libraries = self.get_library_paths(&version_details)?;
        info!("Loaded {} libraries", libraries.len());

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
            .assets_dir(get_assets_dir(&self.game_dir))
            .libraries(libraries)
            .main_jar(get_version_jar_path(&self.game_dir, version_id))
            .java_major_version(java_major_version)
            .use_rosetta(use_rosetta)
            .build()?;

        let mut cmd = minecraft_cmd.build()?;

        println!("Starting Minecraft...");
        info!("Full command: {cmd:?}");

        // Add macOS window debugging
        if cfg!(target_os = "macos") && use_rosetta {
            info!("System: macOS ARM64 with Rosetta 2");
            info!("Java path: {java_path:?}");
            info!("Use Rosetta: {use_rosetta}");
            info!("Version ID: {version_id}");

            // Check if we can get window manager info
            let _ = std::process::Command::new("defaults")
                .args(["read", "com.apple.dock", "orientation"])
                .output()
                .map(|output| {
                    info!(
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
                        info!("Display info available");
                    }
                });
        }

        // Launch the game with proper logging
        let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

        info!("Minecraft process started with PID: {}", child.id());

        // Add window debugging for macOS
        if cfg!(target_os = "macos") {
            // Wait a moment for process to initialize
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // Check if Java process is running and what windows it has
            let pid = child.id();
            let _ = std::process::Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "pid,ppid,state,comm"])
                .output()
                .map(|output| {
                    info!("Process info: {}", String::from_utf8_lossy(&output.stdout));
                });

            // Check for Java windows using system tools
            let _ = std::process::Command::new("osascript")
                .args(["-e", "tell application \"System Events\" to get name of every process whose name contains \"java\""])
                .output()
                .map(|output| {
                    info!("Java processes: {}", String::from_utf8_lossy(&output.stdout));
                });

            // Try to find Minecraft window
            let _ = std::process::Command::new("osascript")
                .args(["-e", "tell application \"System Events\" to get name of every window of every process"])
                .output()
                .map(|output| {
                    let windows = String::from_utf8_lossy(&output.stdout);
                    if windows.contains("Minecraft") {
                        info!("Found Minecraft window in system");
                    } else {
                        info!("No Minecraft window found in system windows");
                    }
                });
        }

        // Monitor the process
        if let Some(stdout) = child.stdout.take() {
            let reader = std::io::BufReader::new(stdout);
            tokio::spawn(async move {
                use std::io::BufRead;
                for line in reader.lines().map_while(Result::ok) {
                    if line.contains("ERROR") || line.contains("FATAL") {
                        error!("MC: {line}");
                    } else if line.contains("WARN") {
                        warn!("MC: {line}");
                    } else {
                        debug!("MC: {line}");
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let reader = std::io::BufReader::new(stderr);
            tokio::spawn(async move {
                use std::io::BufRead;
                for line in reader.lines().map_while(Result::ok) {
                    // Look for specific macOS/window related errors
                    if line.contains("NSWindow")
                        || line.contains("display")
                        || line.contains("OpenGL")
                        || line.contains("LWJGL")
                    {
                        error!("MC Window Error: {line}");
                    } else {
                        error!("MC Error: {line}");
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
            Ok(result) => result.map_err(|e| anyhow::anyhow!("Join error: {e}"))??,
            Err(_) => {
                info!(
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
                            info!(
                                "Windows of PID {}: {}",
                                pid,
                                String::from_utf8_lossy(&output.stdout)
                            );
                        });

                    // Try to bring any Java windows to front
                    let _ = std::process::Command::new("osascript")
                        .args(["-e", "tell application \"System Events\" to set frontmost of first process whose name contains \"java\" to true"])
                        .output()
                        .map(|output| {
                            info!("Attempted to bring Java to front: {}", String::from_utf8_lossy(&output.stdout));
                        });
                }

                return Ok(());
            }
        };

        if status.success() {
            println!("Minecraft exited successfully");
        } else {
            error!("Minecraft exited with code: {:?}", status.code());
            return Err(anyhow::anyhow!(
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
            .ok_or_else(|| anyhow::anyhow!("Version manifest not loaded"))?;

        manifest
            .get_version(version_id)
            .ok_or_else(|| anyhow::anyhow!("Version {version_id} not found"))
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
        info!("Downloading version details for {}", version_info.id);
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
                info!("Downloading client jar for {}", version_details.id);
                let mut progress = ProgressTracker::new(format!("{} client", version_details.id));

                self.downloader
                    .download_file(
                        &client.url,
                        &jar_path,
                        Some(&client.sha1),
                        Some(&mut progress),
                    )
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

            // Download main artifact
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

        info!(
            "Starting native libraries download for version {}",
            version_details.id
        );
        info!("OS: {os_name}, Arch: {os_arch}, Features: {os_features:?}");
        info!("Native classifiers for this platform: {native_classifiers:?}");

        let natives_dir = get_natives_dir(&self.game_dir, &version_details.id);
        ensure_directory(&natives_dir).await?;
        info!("Created natives directory: {natives_dir:?}");

        // Check if natives directory already has files
        let existing_files = std::fs::read_dir(&natives_dir)?.count();
        info!("Existing files in natives directory: {existing_files}");

        let mut download_tasks = Vec::new();
        let mut native_libraries = Vec::new();

        for library in &version_details.libraries {
            let should_use = library.should_use(os_name, os_arch, &os_features);
            debug!(
                "Library {}: should_use={}, has_rules={}, has_classifiers={}",
                library.name,
                should_use,
                library.rules.is_some(),
                library.downloads.classifiers.is_some()
            );

            if let Some(rules) = &library.rules {
                for rule in rules {
                    let matches = rule.matches(os_name, os_arch, &os_features);
                    debug!(
                        "  Rule action={}, matches={}, os={:?}, arch={:?}",
                        rule.action, matches, rule.os, os_arch
                    );
                }
            }

            if !should_use {
                debug!("Skipping library {} (not for this platform)", library.name);
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
                            info!(
                                "Need to download native ({}): {} -> {:?}",
                                classifier, native_artifact.url, native_path
                            );
                            download_tasks.push(
                                DownloadTask::new(native_artifact.url.clone(), native_path)
                                    .with_sha1(native_artifact.sha1.clone()),
                            );
                        } else {
                            info!("Native already exists ({classifier}): {native_path:?}");
                        }
                        found_native = true;
                        break; // Use first matching classifier
                    }
                }

                if !found_native {
                    if let Some(classifiers) = &library.downloads.classifiers {
                        let available_classifiers: Vec<String> =
                            classifiers.keys().cloned().collect();
                        debug!(
                            "No native artifacts found for library {} with classifiers {:?}. Available classifiers: {:?}",
                            library.name, native_classifiers, available_classifiers
                        );
                    } else {
                        debug!(
                            "No native artifacts found for library {} with classifiers {:?}",
                            library.name, native_classifiers
                        );
                    }
                }
            }
        }

        info!(
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
        info!("Extracted {extracted_count} files to natives directory");

        if extracted_count == 0 {
            error!("CRITICAL: No files were extracted to natives directory!");
            error!("This will cause 'no lwjgl in java.library.path' error");

            info!("Available native libraries to extract:");
            for (native_path, library) in &native_libraries {
                info!("  - Library: {}", library.name);
                info!("    Path: {native_path:?}");
                info!("    Exists: {}", native_path.exists());
                if native_path.exists() {
                    let size = std::fs::metadata(native_path)?.len();
                    info!("    Size: {size} bytes");
                }
            }

            // Force re-extraction
            for (native_path, library) in &native_libraries {
                if native_path.exists() {
                    warn!("Force extracting: {native_path:?}");
                    match self.extract_native_library(native_path, &natives_dir, &library.extract) {
                        Ok(()) => info!("Successfully extracted {native_path:?}"),
                        Err(e) => error!("Failed to extract {native_path:?}: {e}"),
                    }
                }
            }

            // Re-check after force extraction
            let final_count = std::fs::read_dir(&natives_dir)?.count();
            if final_count == 0 {
                return Err(anyhow::anyhow!(
                    "Failed to extract any native libraries to {natives_dir:?}. This will cause LWJGL errors."
                ));
            } else {
                info!("After force extraction: {final_count} files in natives directory");
            }
        } else {
            // List extracted files for verification
            info!("Extracted native files:");
            for entry in std::fs::read_dir(&natives_dir)?.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    let size = std::fs::metadata(&path)?.len();
                    info!("  - {name} ({size} bytes)");
                }
            }
        }

        Ok(())
    }

    fn create_dylib_symlinks(&self, natives_dir: &PathBuf) -> Result<()> {
        // On macOS, old LWJGL versions (2.x) use .jnilib extension, but modern Java expects .dylib
        // Create symlinks from .dylib to .jnilib files for compatibility
        if cfg!(target_os = "macos") {
            info!(
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
                            info!("Creating symlink: {dylib_name} -> {file_name_str}");

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

        info!("Extracting natives to: {:?}", natives_dir);
        info!(
            "Total libraries to check: {}",
            version_details.libraries.len()
        );

        let mut processed_count = 0;
        let mut extracted_count = 0;

        for library in &version_details.libraries {
            processed_count += 1;

            if !library.should_use(os_name, os_arch, &os_features) {
                debug!("Skipping library {} (not for this platform)", library.name);
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
                            info!(
                                "Extracting native library ({}) for {}: {:?}",
                                classifier, library.name, native_path
                            );
                            match self.extract_native_library(
                                &native_path,
                                &natives_dir,
                                &library.extract,
                            ) {
                                Ok(()) => {
                                    extracted_count += 1;
                                    found_and_extracted = true;
                                    info!(
                                        "Successfully extracted native library for {}",
                                        library.name
                                    );
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to extract native library for {}: {}",
                                        library.name, e
                                    );
                                }
                            }
                            break; // Only extract first matching classifier
                        } else {
                            debug!(
                                "Native library not found ({}) for {}: {:?}",
                                classifier, library.name, native_path
                            );
                        }
                    }
                }

                if !found_and_extracted {
                    debug!("No native library extracted for {}", library.name);
                }
            } else {
                debug!("Library {} has no classifiers", library.name);
            }
        }

        info!(
            "Native extraction summary: processed {processed_count} libraries, extracted {extracted_count} native libraries"
        );

        // Create .dylib symlinks for .jnilib files on macOS
        self.create_dylib_symlinks(&natives_dir)?;

        Ok(())
    }

    fn extract_native_library(
        &self,
        archive_path: &PathBuf,
        extract_dir: &PathBuf,
        extract_rules: &Option<crate::backend::creeper::models::ExtractRules>,
    ) -> Result<()> {
        use std::io::Read;

        info!(
            "Extracting native archive: {:?} to {:?}",
            archive_path, extract_dir
        );

        if !archive_path.exists() {
            return Err(anyhow::anyhow!(
                "Archive does not exist: {:?}",
                archive_path
            ));
        }

        let archive_size = std::fs::metadata(archive_path)?.len();
        info!("Archive size: {archive_size} bytes");

        let file = std::fs::File::open(archive_path)
            .map_err(|e| anyhow::anyhow!("Failed to open archive {archive_path:?}: {e}"))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| anyhow::anyhow!("Failed to read ZIP archive {archive_path:?}: {e}"))?;

        let mut extracted_files = 0;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| {
                anyhow::anyhow!("Failed to read entry {i} from {archive_path:?}: {e}")
            })?;
            let file_path = file.mangled_name();

            // Skip directories
            if file.is_dir() {
                continue;
            }

            // Check if this file should be excluded
            let mut should_exclude = false;
            if let Some(rules) = extract_rules
                && let Some(exclude) = &rules.exclude
            {
                for pattern in exclude {
                    if file_path.to_string_lossy().contains(pattern) {
                        debug!(
                            "Excluding file {} due to pattern {}",
                            file_path.display(),
                            pattern
                        );
                        should_exclude = true;
                        break;
                    }
                }
            }

            if should_exclude {
                continue;
            }

            // Extract file
            let extract_path = extract_dir.join(&file_path);

            // Create parent directories
            if let Some(parent) = extract_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| anyhow::anyhow!("Failed to create directory {parent:?}: {e}"))?;
            }

            let mut output_file = std::fs::File::create(&extract_path)
                .map_err(|e| anyhow::anyhow!("Failed to create file {extract_path:?}: {e}"))?;

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to read file {} from archive: {}",
                    file_path.display(),
                    e
                )
            })?;

            use std::io::Write;
            output_file
                .write_all(&buffer)
                .map_err(|e| anyhow::anyhow!("Failed to write to file {extract_path:?}: {e}"))?;

            extracted_files += 1;
            debug!("Extracted: {extract_path:?}");
        }

        info!("Extraction complete: {extracted_files} files from {archive_path:?}");

        if extracted_files == 0 {
            error!("CRITICAL: No files were extracted from {archive_path:?}");
            error!("This archive should contain native libraries (.so, .dylib, .dll files)");

            // List contents of the archive for debugging
            let file = std::fs::File::open(archive_path)?;
            let mut archive = zip::ZipArchive::new(file)?;
            error!("Archive contents:");
            for i in 0..std::cmp::min(archive.len(), 10) {
                let file = archive.by_index(i)?;
                error!(
                    "  - {} (size: {}, dir: {})",
                    file.name(),
                    file.size(),
                    file.is_dir()
                );
            }
            if archive.len() > 10 {
                error!("  ... and {} more files", archive.len() - 10);
            }
        } else {
            info!("Successfully extracted {extracted_files} native files");
        }

        Ok(())
    }

    async fn download_assets(&self, version_details: &VersionDetails) -> Result<()> {
        let asset_index_path = get_asset_indexes_dir(&self.game_dir)
            .join(format!("{}.json", version_details.asset_index.id));

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
            let mut progress = ProgressTracker::new("Asset index".to_string());

            self.downloader
                .download_file(
                    &version_details.asset_index.url,
                    &asset_index_path,
                    Some(&version_details.asset_index.sha1),
                    Some(&mut progress),
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
            let asset_path = get_asset_path(&self.game_dir, &asset.hash);

            if !verify_file(&asset_path, Some(asset.size), Some(&asset.hash)).await? {
                let url = format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    &asset.hash[..2],
                    asset.hash
                );

                download_tasks
                    .push(DownloadTask::new(url, asset_path).with_sha1(asset.hash.clone()));
            }

            // Store asset info for virtual assets creation
            assets_for_virtual.push((name, asset));
        }

        if !download_tasks.is_empty() {
            info!("Downloading {} assets...", download_tasks.len());

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
                info!("Assets: {progress}% ({completed}/{total_assets})");
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
                    true // If version format is unexpected, assume we need virtual assets
                }
            }
        };

        if !needs_virtual {
            info!("Version {} doesn't need virtual assets", version_details.id);
            return Ok(());
        }

        info!(
            "Version {} needs virtual assets for index '{}'",
            version_details.id, version_details.assets
        );

        let virtual_dir = get_assets_dir(&self.game_dir)
            .join("virtual")
            .join(&version_details.assets);

        // Create virtual directory
        ensure_directory(&virtual_dir).await?;

        info!(
            "Creating virtual assets for {} at {:?}",
            version_details.assets, virtual_dir
        );
        info!("Processing {} assets", assets.len());

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

            let asset_path = get_asset_path(&self.game_dir, &asset.hash);

            // Create parent directory if needed
            ensure_parent_directory(&virtual_path).await?;

            // Copy or link the asset to virtual location if it doesn't exist
            if !virtual_path.exists() && asset_path.exists() {
                match tokio::fs::copy(&asset_path, &virtual_path).await {
                    Ok(_) => {
                        created_count += 1;
                        if created_count <= 5 {
                            debug!("Created virtual asset: {name}");
                        }
                    }
                    Err(e) => {
                        warn!("Failed to create virtual asset {name}: {e}");
                    }
                }
            } else if !asset_path.exists() {
                warn!("Source asset file missing for {name}: {asset_path:?}");
            }
        }

        if created_count > 0 {
            info!("Created {created_count} virtual assets");
        }

        Ok(())
    }

    async fn verify_installation(&self, version_details: &VersionDetails) -> Result<()> {
        info!("Verifying installation for version {}", version_details.id);

        // Check main jar
        let main_jar = get_version_jar_path(&self.game_dir, &version_details.id);
        if !main_jar.exists() {
            warn!("Main jar missing: {main_jar:?}");
            return Err(anyhow::anyhow!("Main jar missing: {main_jar:?}"));
        }

        // Check natives directory - but allow empty for some versions
        let natives_dir = get_natives_dir(&self.game_dir, &version_details.id);
        if !natives_dir.exists() {
            warn!("Natives directory missing: {natives_dir:?}");
            // Try to create empty natives directory
            ensure_directory(&natives_dir).await?;
            info!("Created empty natives directory");
        }

        let native_count = std::fs::read_dir(&natives_dir)?.count();
        if native_count == 0 {
            warn!("Natives directory is empty: {natives_dir:?}");
            info!("Some versions may work without native libraries");
        } else {
            info!("Installation verified, {native_count} native files found");
        }

        Ok(())
    }

    fn is_version_ready_offline(&self, version_id: &str) -> Result<bool> {
        let version_dir = self.game_dir.join("versions").join(version_id);
        let jar_file = version_dir.join(format!("{version_id}.jar"));
        let json_file = version_dir.join(format!("{version_id}.json"));

        Ok(version_dir.exists() && jar_file.exists() && json_file.exists())
    }

    async fn try_offline_mode(&self, version_id: &str) -> Result<()> {
        info!("Attempting to use offline mode for version {version_id}");

        if self.is_version_ready_offline(version_id)? {
            info!("Version {version_id} found offline, skipping downloads");
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
                    info!("Found existing version: {name}, you can try launching that instead",);
                }
            }
        }

        Err(anyhow::anyhow!(
            "Version {version_id} not available offline. Available versions in {versions_dir:?}. Try running the official Minecraft launcher first to download the version."
        ))
    }

    fn log_system_info(&self) -> Result<()> {
        use std::process::Command;

        info!("=== System Diagnostics ===");

        // Memory info
        if let Ok(output) = Command::new("free").arg("-h").output()
            && let Ok(memory_info) = String::from_utf8(output.stdout)
        {
            info!("Memory info:\n{memory_info}");
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
                warn!("Existing Java/Minecraft processes found:");
                for process in java_processes {
                    warn!("  {process}");
                }
            }
        }

        info!("Game directory: {:?}", self.game_dir);
        info!("Cache directory: {:?}", self.cache_dir);

        Ok(())
    }

    fn test_java_version(&self, java_path: &PathBuf) -> Result<()> {
        use std::process::Command;

        info!("Testing Java installation...");

        let output = Command::new(java_path)
            .args(["-version"])
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run Java: {e}"))?;

        let version_info = String::from_utf8_lossy(&output.stderr);
        info!("Java version info: {version_info}");

        // Extract major version for better compatibility checking
        let major_version = self.get_java_major_version(java_path).unwrap_or(8);

        // Provide version-specific guidance
        if major_version >= 24 {
            warn!(
                "You're using Java {major_version} which is very new. For optimal compatibility with Minecraft, consider using Java 21"
            );
        } else if major_version >= 22 {
            info!("Using Java {major_version}");
        } else if major_version == 21 {
            info!("Using Java 21");
        }

        Ok(())
    }

    fn get_java_major_version(&self, java_path: &PathBuf) -> Result<u8> {
        use std::process::Command;

        let output = Command::new(java_path)
            .args(["-version"])
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run Java: {e}"))?;

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
            warn!("Could not determine Java major version, defaulting to 8");
            Ok(8)
        }
    }

    fn verify_game_files(&self, version_id: &str, libraries: &[PathBuf]) -> Result<()> {
        info!("Verifying game files...");

        // Check main .jar
        let main_jar = get_version_jar_path(&self.game_dir, version_id);
        if !main_jar.exists() {
            return Err(anyhow::anyhow!("Main jar not found: {main_jar:?}"));
        }
        info!("Main jar exists: {main_jar:?}");

        // Check libraries
        let mut missing_libs = Vec::new();
        for lib in libraries {
            if !lib.exists() {
                missing_libs.push(lib.clone());
            }
        }

        if !missing_libs.is_empty() {
            warn!("Missing {} libraries:", missing_libs.len());
            for lib in &missing_libs {
                warn!("  Missing: {lib:?}");
            }
            return Err(anyhow::anyhow!("Missing required libraries"));
        }

        info!("All {} libraries verified", libraries.len());

        // Check natives directory
        let natives_dir = get_natives_dir(&self.game_dir, version_id);
        if !natives_dir.exists() {
            warn!("Natives directory doesn't exist: {natives_dir:?}");
        } else {
            let natives_count = std::fs::read_dir(&natives_dir)?.count();
            info!("Natives directory contains {natives_count} files");
        }

        Ok(())
    }

    fn get_library_paths(&self, version_details: &VersionDetails) -> Result<Vec<PathBuf>> {
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

        Ok(library_paths)
    }
}
