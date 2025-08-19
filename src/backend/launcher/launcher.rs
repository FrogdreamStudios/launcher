//! Core Minecraft launcher implementation.

use super::{
    downloader::{HttpDownloader, models::DownloadTask},
    java::JavaManager,
    models::{AssetManifest, AssetObject, VersionDetails, VersionInfo},
};
use crate::backend::utils::launcher::paths::{
    ensure_directories, get_asset_indexes_dir, get_asset_path, get_assets_dir, get_cache_dir,
    get_game_dir, get_library_path, get_natives_dir, get_version_jar_path,
};
use crate::backend::utils::launcher::starter::CommandBuilder;
use crate::backend::utils::system::files::{ensure_directory, ensure_parent_directory};
use crate::utils::Result;
use crate::{log_error, log_info, log_warn, simple_error};
use std::{path::PathBuf, process::Stdio, sync::Arc};

// Import our modular components
use super::common::{DownloadHelper, FileValidator, PlatformInfo, SystemInfo};
use super::versions::VersionManager;

/// Download context for managing file downloads with verification.
struct DownloadContext<'a> {
    downloader: &'a HttpDownloader,
}

impl<'a> DownloadContext<'a> {
    const fn new(downloader: &'a HttpDownloader) -> Self {
        Self { downloader }
    }

    /// Download the file if needed (with verification).
    async fn download_if_needed(
        &self,
        url: &str,
        path: &PathBuf,
        expected_size: Option<u64>,
        expected_sha1: Option<&str>,
    ) -> Result<bool> {
        if DownloadHelper::needs_download(path, expected_size, expected_sha1).await? {
            self.downloader
                .download_file(url, path, expected_sha1)
                .await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Execute download tasks in batches with progress reporting.
    async fn execute_downloads(&self, tasks: Vec<DownloadTask>, item_type: &str) -> Result<()> {
        use crate::backend::utils::progress_bridge::update_global_progress;

        if tasks.is_empty() {
            return Ok(());
        }

        let total = tasks.len();
        let batch_size = DownloadHelper::calculate_batch_size(total, 32);

        log_info!("Downloading {total} {item_type}...");

        for (i, chunk) in tasks.chunks(batch_size).enumerate() {
            let completed = i * batch_size;
            let progress_percent = completed as f32 / total as f32;

            // Calculate base progress based on item type
            let base_progress = match item_type {
                "libraries" => 0.4,
                "natives" => 0.5,
                "assets" => 0.55,
                _ => 0.3,
            };
            let stage_range = 0.05; // Each download stage gets 5% of total progress
            let current_progress = base_progress + (progress_percent * stage_range);

            update_global_progress(
                current_progress,
                format!("Downloading {} ({}/{})", item_type, completed + 1, total),
            );

            self.downloader.download_multiple(chunk.to_vec(), 8).await?;
            let completed = (i + 1) * chunk.len().min(total - i * batch_size);
            DownloadHelper::log_progress(completed, total, item_type);
        }

        Ok(())
    }
}

/// Main Minecraft launcher that handles downloading and launching game instances.
pub struct MinecraftLauncher {
    downloader: Arc<HttpDownloader>,
    java_manager: JavaManager,
    game_dir: PathBuf,
    cache_dir: PathBuf,
    version_manager: VersionManager,
    #[allow(dead_code)]
    instance_id: Option<u32>,
}

impl MinecraftLauncher {
    /// Gets the game directory path.
    pub const fn get_game_dir(&self) -> &PathBuf {
        &self.game_dir
    }

    /// Creates a new `MinecraftLauncher` instance.
    pub async fn new(custom_game_dir: Option<PathBuf>, instance_id: Option<u32>) -> Result<Self> {
        let game_dir = get_game_dir(custom_game_dir, instance_id)?;
        let cache_dir = get_cache_dir()?;

        // Ensure all directories exist
        ensure_directories(instance_id).await?;

        let downloader = Arc::new(HttpDownloader::new()?);
        let mut version_manager = VersionManager::new(downloader.clone(), cache_dir.clone());

        // Load a cached manifest or fetch a new one
        if let Err(e) = version_manager.load_cached_manifest().await {
            log_warn!("Failed to load cached manifest: {e}");
            version_manager.update_manifest().await?;
        } else if version_manager.get_manifest().is_none() {
            log_info!("No cached manifest found, fetching from Mojang...");
            version_manager.update_manifest().await?;
        }

        Ok(Self {
            downloader,
            java_manager: JavaManager::new().await?,
            game_dir,
            cache_dir,
            version_manager,
            instance_id,
        })
    }

    /// Gets available versions from the version manager.
    pub fn get_available_versions(&self) -> Result<&[VersionInfo]> {
        self.version_manager.get_available_versions()
    }

    /// Updates the version manifest.
    pub async fn update_manifest(&mut self) -> Result<()> {
        self.version_manager.update_manifest().await
    }

    /// Checks if Java is available for a specific version.
    pub fn is_java_available(&self, version: &str) -> bool {
        self.java_manager.is_java_available(version)
    }

    /// Installs Java for a specific version.
    pub async fn install_java(&mut self, version: &str) -> Result<()> {
        match self.java_manager.get_java_for_version(version).await {
            Ok(_) => {
                log_info!("Java installed successfully for Minecraft version {version}");
                Ok(())
            }
            Err(e) => {
                log_error!("Failed to install Java for version {version}: {e}");
                Err(e)
            }
        }
    }

    /// Prepares a Minecraft version for launch by downloading all necessary files.
    pub async fn prepare_version(&self, version_id: &str) -> Result<()> {
        use crate::backend::utils::progress_bridge::update_global_progress;

        log_info!("Preparing Minecraft version: {version_id}");

        // Check if a version already exists offline
        if self
            .version_manager
            .is_version_ready_offline(&self.game_dir, version_id)?
        {
            log_info!("Version {version_id} is already prepared offline");
            update_global_progress(0.65, format!("Version {} is already prepared", version_id));
            return Ok(());
        }

        update_global_progress(
            0.25,
            format!("Loading version manifest for {}...", version_id),
        );

        let version_info = self.version_manager.get_version_info(version_id)?;
        let version_details = match self.version_manager.get_version_details(version_info).await {
            Ok(details) => details,
            Err(e) => {
                log_warn!("Failed to download version details: {e}. Checking for offline version");
                return self.try_offline_mode(version_id);
            }
        };

        // Download all required components
        update_global_progress(0.3, format!("Downloading {} game files...", version_id));
        log_info!("Downloading client jar...");
        if let Err(e) = self.download_client_jar(&version_details).await {
            log_warn!("Failed to download client jar: {e}");
        }

        update_global_progress(0.4, "Downloading game libraries...".to_string());
        log_info!("Downloading libraries...");
        if let Err(e) = self.download_libraries(&version_details).await {
            log_warn!("Failed to download libraries: {e}");
        }

        update_global_progress(0.5, "Downloading native libraries...".to_string());
        log_info!("Downloading and extracting native libraries...");
        if let Err(e) = self.download_natives(&version_details).await {
            log_warn!("Failed to download natives: {e}");
        }

        update_global_progress(0.55, "Downloading game assets...".to_string());
        log_info!("Downloading assets...");
        if let Err(e) = self.download_assets(&version_details).await {
            log_warn!("Failed to download assets: {e}");
        }

        update_global_progress(0.65, "Verifying installation...".to_string());
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

    /// Launches Minecraft with the specified version.
    pub async fn launch(&mut self, version_id: &str) -> Result<()> {
        log_info!("Launching Minecraft version: {version_id}");

        // System diagnostics
        SystemInfo::log_system_info(&self.game_dir, &self.cache_dir);
        SystemInfo::check_existing_processes();

        let version_info = self.version_manager.get_version_info(version_id)?;
        let version_type = version_info.version_type.clone();
        let version_details = self
            .version_manager
            .get_version_details(version_info)
            .await?;

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

        // Test Java version
        Self::test_java_version(&java_path)?;
        let java_major_version = Self::get_java_major_version(&java_path)?;
        log_info!(
            "Java version verification complete: Java {}",
            java_major_version
        );

        // Log detailed Java info on Windows for debugging
        if cfg!(windows) {
            log_info!("Windows Java debugging info:");
            log_info!("  Java executable: {}", java_path.display());
            log_info!("  Java major version: {}", java_major_version);
            log_info!("  Use Rosetta: {}", use_rosetta);
        }

        // Build library paths
        let libraries = self.get_library_paths(&version_details)?;
        log_info!("Loaded {} libraries", libraries.len());

        // Verify critical files exist
        FileValidator::verify_critical_files(&self.game_dir, version_id, &libraries)?;

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
        log_info!("Full command: {:?}", cmd);

        // Extra debugging for Windows
        if cfg!(windows) {
            log_info!("Windows launch debugging:");
            log_info!("  Working directory: {:?}", cmd.get_current_dir());
            log_info!("  Environment variables:");
            for (key, value) in cmd.get_envs() {
                if let (Some(k), Some(v)) = (key.to_str(), value.and_then(|v| v.to_str())) {
                    if k.contains("JAVA") || k.contains("PATH") || k.contains("LWJGL") {
                        log_info!("    {}: {}", k, v);
                    }
                }
            }
        }

        // Launch the game
        let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
        log_info!("Minecraft process started with PID: {}", child.id());

        // Log success on Windows
        if cfg!(windows) {
            log_info!("Windows process launch successful - PID: {}", child.id());
        }

        // Hide progress bar immediately after successful process start
        if let Some(sender) = crate::backend::utils::progress_bridge::get_progress_sender() {
            // Send completion signal to hide progress bar
            let _ = sender.send(crate::backend::launcher::progress::ProgressInfo {
                progress: 0.0,
                message: "".to_string(),
                stage: crate::backend::launcher::progress::ProgressStage::Completed,
            });
        }

        // Monitor output streams
        if let Some(stdout) = child.stdout.take() {
            let reader = std::io::BufReader::new(stdout);
            tokio::spawn(async move {
                use std::io::BufRead;
                for line in reader.lines().map_while(std::result::Result::ok) {
                    if line.contains("ERROR") || line.contains("FATAL") {
                        log_error!("MC: {line}");
                    } else if line.contains("WARN") {
                        log_warn!("MC: {line}");
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let reader = std::io::BufReader::new(stderr);
            tokio::spawn(async move {
                use std::io::BufRead;
                for line in reader.lines().map_while(std::result::Result::ok) {
                    log_error!("MC Error: {line}");
                }
            });
        }

        // Wait for the process with timeout
        let _pid = child.id();
        let status = tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            tokio::task::spawn_blocking(move || child.wait()),
        )
        .await;

        match status {
            Ok(result) => {
                let status = result.map_err(|_e| simple_error!("Join error"))??;
                if status.success() {
                    println!("Minecraft exited successfully");
                } else {
                    log_error!("Minecraft exited with code: {:?}", status.code());
                    return Err(simple_error!(
                        "Minecraft process failed with code: {:?}",
                        status.code()
                    ));
                }
            }
            Err(_) => {
                log_info!(
                    "Minecraft process still running after 30 seconds, considering it successful"
                );
            }
        }

        Ok(())
    }

    // Private helper methods
    async fn download_client_jar(&self, version_details: &VersionDetails) -> Result<()> {
        if let Some(client) = &version_details.downloads.client {
            let jar_path = get_version_jar_path(&self.game_dir, &version_details.id);
            let download_ctx = DownloadContext::new(&self.downloader);

            if download_ctx
                .download_if_needed(
                    &client.url,
                    &jar_path,
                    Some(client.size),
                    Some(&client.sha1),
                )
                .await?
            {
                log_info!("Downloaded client jar for {}", version_details.id);
            }
        }
        Ok(())
    }

    async fn download_libraries(&self, version_details: &VersionDetails) -> Result<()> {
        let platform_info = PlatformInfo::new();
        let download_ctx = DownloadContext::new(&self.downloader);
        let mut download_tasks = Vec::new();

        for library in &version_details.libraries {
            if !library.should_use(
                platform_info.os_name,
                platform_info.os_arch,
                &platform_info.os_features,
            ) {
                continue;
            }

            if let Some(artifact) = &library.downloads.artifact
                && let Some(path) = &artifact.path
            {
                let lib_path = get_library_path(&self.game_dir, path);
                if DownloadHelper::needs_download(
                    &lib_path,
                    Some(artifact.size),
                    Some(&artifact.sha1),
                )
                .await?
                {
                    download_tasks.push(
                        DownloadTask::new(artifact.url.clone(), lib_path)
                            .with_sha1(artifact.sha1.clone()),
                    );
                }
            }
        }

        download_ctx
            .execute_downloads(download_tasks, "libraries")
            .await
    }

    async fn download_natives(&self, version_details: &VersionDetails) -> Result<()> {
        let platform_info = PlatformInfo::new();
        let download_ctx = DownloadContext::new(&self.downloader);
        let natives_dir = get_natives_dir(&self.game_dir, &version_details.id);
        ensure_directory(&natives_dir).await?;

        let mut download_tasks = Vec::new();

        for library in &version_details.libraries {
            if !library.should_use(
                platform_info.os_name,
                platform_info.os_arch,
                &platform_info.os_features,
            ) {
                continue;
            }

            if let Some(classifiers) = &library.downloads.classifiers {
                for classifier in &platform_info.native_classifiers {
                    if let Some(native_artifact) = classifiers.get(classifier)
                        && let Some(path) = &native_artifact.path
                    {
                        let native_path = get_library_path(&self.game_dir, path);
                        if DownloadHelper::needs_download(
                            &native_path,
                            Some(native_artifact.size),
                            Some(&native_artifact.sha1),
                        )
                        .await?
                        {
                            download_tasks.push(
                                DownloadTask::new(native_artifact.url.clone(), native_path)
                                    .with_sha1(native_artifact.sha1.clone()),
                            );
                        }
                        break;
                    }
                }
            }
        }

        download_ctx
            .execute_downloads(download_tasks, "natives")
            .await?;
        self.extract_natives(version_details).await
    }

    async fn download_assets(&self, version_details: &VersionDetails) -> Result<()> {
        let download_ctx = DownloadContext::new(&self.downloader);

        // Download asset index
        let asset_index_path =
            get_asset_indexes_dir()?.join(format!("{}.json", version_details.asset_index.id));
        if download_ctx
            .download_if_needed(
                &version_details.asset_index.url,
                &asset_index_path,
                Some(version_details.asset_index.size),
                Some(&version_details.asset_index.sha1),
            )
            .await?
        {
            log_info!("Downloaded asset index: {}", version_details.asset_index.id);
        }

        // Parse asset index and collect download tasks
        let asset_content = tokio::fs::read_to_string(&asset_index_path).await?;
        let asset_manifest: AssetManifest = serde_json::from_str(&asset_content)?;

        let mut download_tasks = Vec::new();
        let mut assets_for_virtual = Vec::new();

        for (name, asset) in asset_manifest.objects {
            let asset_path = get_asset_path(&asset.hash)?;
            if DownloadHelper::needs_download(&asset_path, Some(asset.size), Some(&asset.hash))
                .await?
            {
                let url = format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    &asset.hash[..2],
                    asset.hash
                );
                download_tasks
                    .push(DownloadTask::new(url, asset_path).with_sha1(asset.hash.clone()));
            }
            assets_for_virtual.push((name, asset));
        }

        download_ctx
            .execute_downloads(download_tasks, "assets")
            .await?;
        self.create_virtual_assets(version_details, &assets_for_virtual)
            .await
    }

    async fn extract_natives(&self, version_details: &VersionDetails) -> Result<()> {
        let platform_info = PlatformInfo::new();
        let natives_dir = get_natives_dir(&self.game_dir, &version_details.id);

        log_info!("Extracting natives to: {}", natives_dir.display());
        log_info!(
            "Platform info: OS={}, Arch={}, Classifiers={:?}",
            platform_info.os_name,
            platform_info.os_arch,
            platform_info.native_classifiers
        );

        let mut extracted_count = 0;
        let mut total_natives = 0;

        for library in &version_details.libraries {
            if !library.should_use(
                platform_info.os_name,
                platform_info.os_arch,
                &platform_info.os_features,
            ) {
                continue;
            }

            if let Some(classifiers) = &library.downloads.classifiers {
                for classifier in &platform_info.native_classifiers {
                    if let Some(native_artifact) = classifiers.get(classifier)
                        && let Some(path) = &native_artifact.path
                    {
                        total_natives += 1;
                        let native_path = get_library_path(&self.game_dir, path);
                        log_info!(
                            "Processing native: {} -> {}",
                            native_path.display(),
                            natives_dir.display()
                        );

                        if native_path.exists() {
                            match crate::utils::extract_zip(&native_path, &natives_dir).await {
                                Ok(()) => {
                                    extracted_count += 1;
                                    log_info!(
                                        "✓ Successfully extracted native: {}",
                                        native_path
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                    );
                                }
                                Err(e) => {
                                    log_warn!(
                                        "✗ Failed to extract {}: {}",
                                        native_path.display(),
                                        e
                                    );
                                }
                            }
                        } else {
                            log_warn!("✗ Native library not found: {}", native_path.display());
                        }
                        break;
                    }
                }
            }
        }

        log_info!(
            "Native extraction complete: {}/{} libraries extracted",
            extracted_count,
            total_natives
        );

        // Verify natives directory contents on Windows
        if cfg!(windows) {
            if let Ok(entries) = std::fs::read_dir(&natives_dir) {
                let files: Vec<_> = entries
                    .filter_map(|entry| entry.ok().and_then(|e| e.file_name().into_string().ok()))
                    .collect();
                log_info!("Natives directory contents: {:?}", files);

                // Look for essential LWJGL libraries
                let essential_libs = ["lwjgl.dll", "lwjgl_opengl.dll", "lwjgl_glfw.dll"];
                for lib in &essential_libs {
                    if files.iter().any(|f| f.contains(lib)) {
                        log_info!("✓ Found essential library: {}", lib);
                    } else {
                        log_warn!("✗ Missing essential library: {}", lib);
                    }
                }
            } else {
                log_warn!(
                    "Failed to read natives directory: {}",
                    natives_dir.display()
                );
            }
        }

        Ok(())
    }

    async fn create_virtual_assets(
        &self,
        version_details: &VersionDetails,
        assets: &[(String, AssetObject)],
    ) -> Result<()> {
        // Check if we need virtual assets
        let needs_virtual = matches!(version_details.assets.as_str(), "legacy" | "pre-1.6")
            || version_details
                .id
                .split('.')
                .next()
                .unwrap_or("1")
                .parse::<i32>()
                .unwrap_or(1)
                >= 1;

        if !needs_virtual {
            return Ok(());
        }

        let virtual_dir = get_assets_dir()?
            .join("virtual")
            .join(&version_details.assets);
        ensure_directory(&virtual_dir).await?;

        for (name, asset) in assets {
            let virtual_path =
                if version_details.assets == "legacy" || version_details.assets == "pre-1.6" {
                    let legacy_name = name.strip_prefix("minecraft/").unwrap_or(name);
                    virtual_dir.join("resources").join(legacy_name)
                } else {
                    virtual_dir.join(name)
                };

            let asset_path = get_asset_path(&asset.hash)?;
            ensure_parent_directory(&virtual_path).await?;

            if !virtual_path.exists() && asset_path.exists() {
                let _ = tokio::fs::copy(&asset_path, &virtual_path).await;
            }
        }

        Ok(())
    }

    async fn verify_installation(&self, version_details: &VersionDetails) -> Result<()> {
        let main_jar = get_version_jar_path(&self.game_dir, &version_details.id);
        if !main_jar.exists() {
            return Err(simple_error!("Main jar missing: {main_jar:?}"));
        }

        let natives_dir = get_natives_dir(&self.game_dir, &version_details.id);
        if !natives_dir.exists() {
            ensure_directory(&natives_dir).await?;
        }

        Ok(())
    }

    fn try_offline_mode(&self, version_id: &str) -> Result<()> {
        if self
            .version_manager
            .is_version_ready_offline(&self.game_dir, version_id)?
        {
            log_info!("Version {version_id} found offline, skipping downloads");
            return Ok(());
        }

        Err(simple_error!("Version {version_id} not available offline"))
    }

    fn test_java_version(java_path: &PathBuf) -> Result<()> {
        let output = std::process::Command::new(java_path)
            .args(["-version"])
            .output()
            .map_err(|e| simple_error!("Failed to run Java: {}", e))?;

        let version_info = String::from_utf8_lossy(&output.stderr);
        log_info!("Java version info: {}", version_info);
        Ok(())
    }

    fn get_java_major_version(java_path: &PathBuf) -> Result<u8> {
        let output = std::process::Command::new(java_path)
            .args(["-version"])
            .output()
            .map_err(|_e| simple_error!("Failed to run Java"))?;

        let version_info = String::from_utf8_lossy(&output.stderr);

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

        Ok(8) // Default fallback
    }

    fn get_library_paths(&self, version_details: &VersionDetails) -> Result<Vec<PathBuf>> {
        let platform_info = PlatformInfo::new();
        let mut library_paths = Vec::new();

        for library in &version_details.libraries {
            if !library.should_use(
                platform_info.os_name,
                platform_info.os_arch,
                &platform_info.os_features,
            ) {
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
