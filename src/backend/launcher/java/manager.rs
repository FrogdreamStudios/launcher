//! Java runtime management and utilities.

use crate::utils::Result;
use crate::{log_debug, log_error, log_info, log_warn, simple_error};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use tokio::fs as async_fs;

use super::runtime::{AzulJavaManifest, AzulPackage, JavaRuntime};
use crate::backend::launcher::downloader::HttpDownloader;
use crate::backend::utils::archiever::main::extract_archive;
use crate::backend::utils::launcher::paths::get_java_dir;
use crate::backend::utils::system::files::{
    ensure_directory, get_file_size, remove_dir_if_exists, remove_file_if_exists,
};

pub struct JavaManager {
    downloader: HttpDownloader,
    java_dir: PathBuf,
    installed_runtimes: HashMap<u8, JavaRuntime>,
    // For Rosetta compatibility
    x86_64_runtimes: HashMap<u8, JavaRuntime>,
}

impl JavaManager {
    pub async fn new() -> Result<Self> {
        let java_dir = get_java_dir()?;
        ensure_directory(&java_dir).await?;

        let mut manager = Self {
            downloader: HttpDownloader::new()?,
            java_dir,
            installed_runtimes: HashMap::new(),
            x86_64_runtimes: HashMap::new(),
        };

        manager.scan_installed_runtimes().await?;
        Ok(manager)
    }

    pub async fn get_java_for_version(
        &mut self,
        minecraft_version: &str,
    ) -> Result<(PathBuf, bool)> {
        let required_java = JavaRuntime::get_required_java_version(minecraft_version);
        let needs_x86_64 = Self::needs_x86_64_java(minecraft_version);

        log_info!(
            "Minecraft version {minecraft_version} requires Java {required_java} (x86_64: {needs_x86_64})"
        );

        // Check if we already have a compatible runtime
        let invalid_runtime_version = if needs_x86_64 {
            if let Some(runtime) = self.get_compatible_x86_64_runtime(required_java) {
                log_info!(
                    "Using existing x86_64 Java {} runtime",
                    runtime.major_version
                );
                let exe_path = runtime.get_executable_path();
                if exe_path.exists() {
                    return Ok((exe_path, true));
                }
                log_info!(
                    "Installed x86_64 Java runtime not found at {exe_path:?}, removing from cache"
                );
                Some(runtime.major_version)
            } else {
                None
            }
        } else if let Some(runtime) = self.get_compatible_runtime(required_java) {
            log_info!("Using existing Java {} runtime", runtime.major_version);
            let exe_path = runtime.get_executable_path();
            if exe_path.exists() {
                return Ok((exe_path, false));
            }
            log_info!("Installed Java runtime not found at {exe_path:?}, removing from cache");
            Some(runtime.major_version)
        } else {
            None
        };

        // Remove invalid runtime from the cache if needed
        if let Some(version) = invalid_runtime_version {
            if needs_x86_64 {
                self.x86_64_runtimes.remove(&version);
            } else {
                self.installed_runtimes.remove(&version);
            }
        }

        // Check system Java (skip for x86_64 requirement as system Java might be ARM64)
        if !needs_x86_64
            && let Some(mut system_java) = JavaRuntime::detect_system_java()?
            && system_java.is_compatible_with_minecraft(required_java)
        {
            log_info!("Using system Java {} runtime", system_java.major_version);
            // Set the correct path for system Java
            if system_java.path.as_os_str().is_empty() {
                system_java.path = crate::utils::which("java").unwrap_or_else(|_| "java".into());
            }
            return Ok((system_java.get_executable_path(), false));
        }

        // Download and install the required Java
        if needs_x86_64 {
            log_info!(
                "Downloading x86_64 Java {required_java} runtime for Minecraft {minecraft_version}"
            );

            match self.install_x86_64_java_runtime(required_java).await {
                Ok(()) => {
                    // Get the newly installed runtime
                    self.x86_64_runtimes.get(&required_java).map_or_else(
                        || {
                            Err(simple_error!(
                                "Failed to install x86_64 Java {required_java} runtime"
                            ))
                        },
                        |runtime| Ok((runtime.get_executable_path(), true)),
                    )
                }
                Err(e) => {
                    log_warn!("Failed to download x86_64 Java {required_java}: {e}");
                    log_warn!("Attempting to use system Java as fallback...");

                    // Try system Java as a fallback
                    if let Some(mut system_java) = JavaRuntime::detect_system_java()? {
                        if system_java.is_compatible_with_minecraft(required_java) {
                            log_info!(
                                "Using system Java {} as fallback",
                                system_java.major_version
                            );
                            if system_java.path.as_os_str().is_empty() {
                                system_java.path =
                                    crate::utils::which("java").unwrap_or_else(|_| "java".into());
                            }
                            return Ok((system_java.get_executable_path(), false));
                        }
                        log_warn!(
                            "System Java {} is not compatible with required Java {}",
                            system_java.major_version,
                            required_java
                        );
                    }

                    Err(simple_error!(
                        "Failed to install x86_64 Java {required_java} and no compatible system Java found"
                    ))
                }
            }
        } else {
            log_info!("Downloading Java {required_java} runtime for Minecraft {minecraft_version}");

            match self.install_java_runtime(required_java).await {
                Ok(()) => {
                    // Get the newly installed runtime
                    self.installed_runtimes.get(&required_java).map_or_else(
                        || {
                            Err(simple_error!(
                                "Failed to install Java {required_java} runtime"
                            ))
                        },
                        |runtime| Ok((runtime.get_executable_path(), false)),
                    )
                }
                Err(e) => {
                    log_warn!("Failed to download native Java {required_java}: {e}");

                    // For modern versions requiring Java 21, try x86_64 as a fallback
                    if required_java >= 21 {
                        log_warn!(
                            "Attempting to download x86_64 Java {required_java} as fallback..."
                        );
                        match self.install_x86_64_java_runtime(required_java).await {
                            Ok(()) => {
                                if let Some(runtime) = self.x86_64_runtimes.get(&required_java) {
                                    log_info!(
                                        "Successfully installed x86_64 Java {required_java} as fallback"
                                    );
                                    return Ok((runtime.get_executable_path(), true));
                                }
                            }
                            Err(x86_err) => {
                                log_warn!("x86_64 fallback also failed: {x86_err}");
                            }
                        }
                    }

                    log_warn!("Attempting to use system Java as fallback...");

                    // Try system Java as a fallback
                    if let Some(mut system_java) = JavaRuntime::detect_system_java()? {
                        if system_java.is_compatible_with_minecraft(required_java) {
                            log_info!(
                                "Using system Java {} as fallback",
                                system_java.major_version
                            );
                            if system_java.path.as_os_str().is_empty() {
                                system_java.path =
                                    crate::utils::which("java").unwrap_or_else(|_| "java".into());
                            }
                            return Ok((system_java.get_executable_path(), false));
                        }
                        log_warn!(
                            "System Java {} is not compatible with required Java {}",
                            system_java.major_version,
                            required_java
                        );
                    }

                    Err(simple_error!(
                        "Failed to install Java {required_java} and no compatible system Java found"
                    ))
                }
            }
        }
    }

    pub fn get_compatible_runtime(&self, min_version: u8) -> Option<&JavaRuntime> {
        self.installed_runtimes
            .values()
            .filter(|runtime| runtime.major_version >= min_version)
            .min_by_key(|runtime| runtime.major_version)
    }

    pub fn get_compatible_x86_64_runtime(&self, min_version: u8) -> Option<&JavaRuntime> {
        self.x86_64_runtimes
            .values()
            .filter(|runtime| runtime.major_version >= min_version)
            .min_by_key(|runtime| runtime.major_version)
    }

    /// Determines if a Minecraft version needs `x86_64` Java on Apple Silicon
    /// due to incompatible native libraries.
    fn needs_x86_64_java(minecraft_version: &str) -> bool {
        // Only applies to Apple Silicon (ARM64) systems
        if std::env::consts::ARCH != "aarch64" || std::env::consts::OS != "macos" {
            return false;
        }

        // Handle snapshots and special versions first
        if Self::is_modern_snapshot_or_version(minecraft_version) {
            return false; // Modern snapshots support ARM64 natively
        }

        // Parse Minecraft version
        if let Ok((major, minor, _patch)) = Self::parse_minecraft_version(minecraft_version) {
            // Versions before 1.19 typically have x86_64-only natives
            // This is a conservative approach - some versions between 1.16-1.19 might work
            matches!((major, minor), (1, m) if m < 19)
        } else {
            // For unknown versions, assume modern (ARM64 native support)
            false
        }
    }

    /// Check if this is a modern snapshot or version that supports ARM64 natively
    fn is_modern_snapshot_or_version(version: &str) -> bool {
        let version_lower = version.to_lowercase();

        // Handle snapshots (e.g., "25w31a", "24w44a", "23w31a")
        if version_lower.contains('w')
            && version_lower.len() >= 5
            && let Some(year_str) = version_lower.get(0..2)
            && let Ok(year) = year_str.parse::<u32>()
        {
            // Snapshots from 2021 (21w) onwards support ARM64
            return year >= 21;
        }

        // Handle pre-releases and release candidates
        if (version_lower.contains("-pre") || version_lower.contains("-rc"))
            && let Ok(parsed) =
                Self::parse_minecraft_version(version_lower.split('-').next().unwrap_or(version))
        {
            return parsed >= (1, 19, 0); // 1.19+ support ARM64
        }

        // Handle experimental and special versions
        if version_lower.contains("experimental") || version_lower.contains("snapshot") {
            return true; // Assume modern experimental versions support ARM64
        }

        // Check if it's a regular version that supports ARM64
        if let Ok(parsed) = Self::parse_minecraft_version(version) {
            return parsed >= (1, 19, 0);
        }

        // Default to modern for unknown versions
        true
    }

    fn parse_minecraft_version(version: &str) -> Result<(u8, u8, u8)> {
        let parts: Vec<&str> = version.split('.').collect();

        if parts.len() >= 2 {
            let major = parts[0].parse::<u8>()?;
            let minor = parts[1].parse::<u8>()?;
            let patch = if parts.len() > 2 {
                parts[2].parse::<u8>().unwrap_or(0)
            } else {
                0
            };

            Ok((major, minor, patch))
        } else {
            Err(simple_error!("Invalid Minecraft version format: {version}"))
        }
    }

    pub async fn install_java_runtime(&mut self, java_version: u8) -> Result<()> {
        let manifest = Self::fetch_azul_manifest();

        let os = AzulPackage::get_os_name();
        let arch = AzulPackage::get_arch_name();

        let package = manifest
            .packages
            .iter()
            .find(|pkg| pkg.matches_requirements(java_version, os, arch))
            .ok_or_else(|| {
                simple_error!("No Azul {arch} Java {java_version} package found for {os} {arch}")
            })?;

        log_info!(
            "Found Java {} package: {} ({} MB)",
            java_version,
            package.name,
            package.size / 1024 / 1024
        );

        // Determine file extension from URL
        let file_extension = if Path::new(&package.download_url)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
        {
            "zip"
        } else if package.download_url.ends_with(".tar.gz")
            || Path::new(&package.download_url)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("tgz"))
        {
            "tar.gz"
        } else {
            // Default based on OS
            if cfg!(windows) { "zip" } else { "tar.gz" }
        };

        let download_path = self
            .java_dir
            .join(format!("java-{java_version}-download.{file_extension}"));
        let extract_path = self.java_dir.join(format!("java-{java_version}"));

        // Download the package
        log_info!(
            "Downloading Java {} from: {}",
            java_version,
            package.download_url
        );

        if let Err(e) = self
            .downloader
            .download_file(
                &package.download_url,
                &download_path,
                None, // Disable hash verification for now
            )
            .await
        {
            log_error!("Failed to download Java {java_version}: {e}");
            let _ = remove_file_if_exists(&download_path).await;
            return Err(e);
        }

        // Verify the downloaded file
        let file_size = get_file_size(&download_path).await?;
        log_info!("Downloaded Java {java_version} archive: {file_size} bytes");

        if file_size < 1024 * 1024 {
            log_error!("Downloaded file is too small ({file_size}B), likely corrupted",);
            let _ = async_fs::remove_file(&download_path).await;
            return Err(simple_error!(
                "Downloaded Java archive is too small, likely corrupted"
            ));
        }

        // Extract the package
        log_info!("Extracting Java {java_version} runtime...");
        // Extract the archive
        if let Err(e) = extract_archive(&download_path, &extract_path).await {
            log_error!("Failed to extract Java {java_version}: {e}");
            let _ = remove_file_if_exists(&download_path).await;
            let _ = remove_dir_if_exists(&extract_path).await;
            return Err(e);
        }

        // Clean up download file
        remove_file_if_exists(&download_path).await?;

        // Detect the extracted runtime
        let java_executable = Self::find_java_executable(&extract_path)?;
        if let Some(runtime) = JavaRuntime::from_path(&java_executable)? {
            self.installed_runtimes.insert(java_version, runtime);
            log_info!("Successfully installed Java {java_version} runtime");
        } else {
            log_error!("Failed to detect installed Java {java_version} runtime");
        }

        Ok(())
    }

    pub async fn install_x86_64_java_runtime(&mut self, java_version: u8) -> Result<()> {
        let manifest = Self::fetch_azul_manifest();

        let os = AzulPackage::get_os_name();
        #[allow(clippy::no_effect_underscore_binding)]
        let _arch = "x64"; // Force x86_64 architecture

        let package = manifest
            .packages
            .iter()
            .find(|pkg| pkg.matches_requirements(java_version, os, "x64"))
            .ok_or_else(|| {
                simple_error!("No Azul x64 Java {java_version} package found for {os} x64")
            })?;

        log_info!(
            "Found x86_64 Java {} package: {} ({} MB)",
            java_version,
            package.name,
            package.size / 1024 / 1024
        );

        // Determine file extension from URL
        let file_extension = if Path::new(&package.download_url)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
        {
            "zip"
        } else if package.download_url.ends_with(".tar.gz")
            || Path::new(&package.download_url)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("tgz"))
        {
            "tar.gz"
        } else {
            // Default based on OS
            if cfg!(windows) { "zip" } else { "tar.gz" }
        };

        let download_path = self
            .java_dir
            .join(format!("java-{java_version}-x64-download.{file_extension}"));
        let extract_path = self.java_dir.join(format!("java-{java_version}-x64"));

        // Download the package
        log_info!(
            "Downloading x86_64 Java {} from: {}",
            java_version,
            package.download_url
        );

        if let Err(e) = self
            .downloader
            .download_file(&package.download_url, &download_path, None)
            .await
        {
            log_error!("Failed to download x86_64 Java {java_version}: {e}");
            let _ = remove_file_if_exists(&download_path).await;
            return Err(e);
        }

        // Verify the downloaded file
        let file_size = async_fs::metadata(&download_path).await?.len();
        log_info!("Downloaded x86_64 Java {java_version} archive: {file_size} bytes");

        if file_size < 1024 * 1024 {
            log_error!("Downloaded file is too small ({file_size}B), likely corrupted");
            let _ = async_fs::remove_file(&download_path).await;
            return Err(simple_error!(
                "Downloaded x86_64 Java archive is too small, likely corrupted"
            ));
        }

        // Extract the package
        log_info!("Extracting Java {java_version} runtime...");
        // Extract the archive
        if let Err(e) = extract_archive(&download_path, &extract_path).await {
            log_error!("Failed to extract Java {java_version}: {e}");
            let _ = remove_file_if_exists(&download_path).await;
            let _ = remove_dir_if_exists(&extract_path).await;
            return Err(e);
        }

        // Clean up download file
        remove_file_if_exists(&download_path).await?;

        // Detect the extracted runtime
        let java_executable = Self::find_java_executable(&extract_path)?;
        if let Some(runtime) = JavaRuntime::from_path(&java_executable)? {
            self.x86_64_runtimes.insert(java_version, runtime);
            log_info!("Successfully installed x86_64 Java {java_version} runtime");
        } else {
            log_error!("Failed to detect installed x86_64 Java {java_version} runtime");
        }

        Ok(())
    }

    async fn scan_installed_runtimes(&mut self) -> Result<()> {
        if !self.java_dir.exists() {
            return Ok(());
        }

        let mut entries = async_fs::read_dir(&self.java_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("java-"))
                && let Ok(java_executable) = Self::find_java_executable(&path)
                && let Some(runtime) = JavaRuntime::from_path(&java_executable)?
            {
                let is_x86_64 = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains("-x64"));

                let major_version: u8 = runtime.major_version;

                log_debug!(
                    "Found installed {} Java {} runtime at {:?}",
                    if is_x86_64 { "x86_64" } else { "native" },
                    major_version,
                    path
                );

                if is_x86_64 {
                    self.x86_64_runtimes.insert(major_version, runtime);
                } else {
                    self.installed_runtimes.insert(major_version, runtime);
                }
            }
        }

        log_info!(
            "Found {} native and {} x86_64 installed Java runtimes",
            self.installed_runtimes.len(),
            self.x86_64_runtimes.len()
        );
        Ok(())
    }

    fn fetch_azul_manifest() -> AzulJavaManifest {
        #[allow(clippy::no_effect_underscore_binding)]
        let _manifest_url = "https://api.azul.com/zulu/download/community/v1.0/bundles/";

        Self::create_fallback_manifest()
    }

    fn create_fallback_manifest() -> AzulJavaManifest {
        let packages = vec![
            // Java 8 packages
            AzulPackage {
                id: "zulu8-windows-x64".to_string(),
                name: "Zulu 8 Windows x64".to_string(),
                java_version: vec![8],
                os: "windows".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu8.62.0.19-ca-jdk8.0.332-win_x64.zip".to_string(),
                sha256_hash: String::new(),
                size: 104_857_600,
            },
            AzulPackage {
                id: "zulu8-macos-x64".to_string(),
                name: "Zulu 8 macOS x64".to_string(),
                java_version: vec![8],
                os: "macos".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu8.62.0.19-ca-jdk8.0.332-macosx_x64.tar.gz".to_string(),
                sha256_hash: String::new(),
                size: 104_857_600,
            },
            AzulPackage {
                id: "zulu8-macos-arm64".to_string(),
                name: "Zulu 8 macOS ARM64".to_string(),
                java_version: vec![8],
                os: "macos".to_string(),
                arch: "arm64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu8.62.0.19-ca-jdk8.0.332-macosx_aarch64.tar.gz".to_string(),
                sha256_hash: String::new(),
                size: 104_857_600,
            },
            AzulPackage {
                id: "zulu8-linux-x64".to_string(),
                name: "Zulu 8 Linux x64".to_string(),
                java_version: vec![8],
                os: "linux".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu8.62.0.19-ca-jdk8.0.332-linux_x64.tar.gz".to_string(),
                sha256_hash: String::new(),
                size: 104_857_600,
            },
            // Java 17 packages
            AzulPackage {
                id: "zulu17-windows-x64".to_string(),
                name: "Zulu 17 Windows x64".to_string(),
                java_version: vec![17],
                os: "windows".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu17.34.19-ca-jdk17.0.3-win_x64.zip".to_string(),
                sha256_hash: String::new(),
                size: 183_500_800,
            },
            AzulPackage {
                id: "zulu17-macos-x64".to_string(),
                name: "Zulu 17 macOS x64".to_string(),
                java_version: vec![17],
                os: "macos".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu17.34.19-ca-jdk17.0.3-macosx_x64.tar.gz".to_string(),
                sha256_hash: String::new(),
                size: 183_500_800,
            },
            AzulPackage {
                id: "zulu17-macos-arm64".to_string(),
                name: "Zulu 17 macOS ARM64".to_string(),
                java_version: vec![17],
                os: "macos".to_string(),
                arch: "arm64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu17.34.19-ca-jdk17.0.3-macosx_aarch64.tar.gz".to_string(),
                sha256_hash: String::new(),
                size: 183_500_800,
            },
            AzulPackage {
                id: "zulu17-linux-x64".to_string(),
                name: "Zulu 17 Linux x64".to_string(),
                java_version: vec![17],
                os: "linux".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu17.34.19-ca-jdk17.0.3-linux_x64.tar.gz".to_string(),
                sha256_hash: String::new(),
                size: 183_500_800,
            },
            // Java 21 packages
            AzulPackage {
                id: "zulu21-windows-x64".to_string(),
                name: "Zulu 21 Windows x64".to_string(),
                java_version: vec![21],
                os: "windows".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu21.36.17-ca-jdk21.0.4-win_x64.zip".to_string(),
                sha256_hash: String::new(),
                size: 200_000_000,
            },
            AzulPackage {
                id: "zulu21-macos-x64".to_string(),
                name: "Zulu 21 macOS x64".to_string(),
                java_version: vec![21],
                os: "macos".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu21.36.17-ca-jdk21.0.4-macosx_x64.tar.gz".to_string(),
                sha256_hash: String::new(),
                size: 200_000_000,
            },
            AzulPackage {
                id: "zulu21-macos-arm64".to_string(),
                name: "Zulu 21 macOS ARM64".to_string(),
                java_version: vec![21],
                os: "macos".to_string(),
                arch: "arm64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu21.36.17-ca-jdk21.0.4-macosx_aarch64.tar.gz".to_string(),
                sha256_hash: String::new(),
                size: 200_000_000,
            },
            AzulPackage {
                id: "zulu21-linux-x64".to_string(),
                name: "Zulu 21 Linux x64".to_string(),
                java_version: vec![21],
                os: "linux".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu21.36.17-ca-jdk21.0.4-linux_x64.tar.gz".to_string(),
                sha256_hash: String::new(),
                size: 200_000_000,
            },
        ];

        AzulJavaManifest { packages }
    }

    fn find_java_executable(java_dir: &Path) -> Result<PathBuf> {
        let executable_name = if cfg!(windows) { "java.exe" } else { "java" };

        // Look for common locations within the Java installation
        let possible_paths = vec![
            java_dir.join("bin").join(executable_name),
            java_dir
                .join("Contents")
                .join("Home")
                .join("bin")
                .join(executable_name),
        ];

        for path in possible_paths {
            if path.exists() {
                return Ok(path);
            }
        }

        // If not found in common locations, search recursively
        Self::find_java_recursive(java_dir, executable_name)
    }

    pub fn find_java_recursive(dir: &Path, executable_name: &str) -> Result<PathBuf> {
        for entry in fs::read_dir(dir)
            .map_err(|e| simple_error!("Failed to read dir {}: {}", dir.display(), e))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_file()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
            {
                if name == executable_name {
                    return Ok(path);
                }
            } else if path.is_dir()
                && let Ok(result) = Self::find_java_recursive(&path, executable_name)
            {
                return Ok(result);
            }
        }

        Err(simple_error!(
            "Java executable not found in {}",
            dir.display()
        ))
    }

    pub fn is_java_available(&self, minecraft_version: &str) -> bool {
        let required_java = JavaRuntime::get_required_java_version(minecraft_version);
        let needs_x86_64 = Self::needs_x86_64_java(minecraft_version);

        // For modern snapshots requiring Java 21, always prefer downloading managed Java
        // instead of using system Java to ensure compatibility
        if required_java >= 21 {
            // Only check managed runtimes, not system Java
            return if needs_x86_64 {
                self.get_compatible_x86_64_runtime(required_java).is_some()
            } else {
                self.get_compatible_runtime(required_java).is_some()
            };
        }

        // For older versions (Java 8, 17), allow system Java as fallback
        if needs_x86_64 {
            if self.get_compatible_x86_64_runtime(required_java).is_some() {
                return true;
            }
        } else {
            if self.get_compatible_runtime(required_java).is_some() {
                return true;
            }

            // Check system Java (only for non-x86_64 requirements and older versions)
            if let Ok(Some(system_java)) = JavaRuntime::detect_system_java() {
                return system_java.is_compatible_with_minecraft(required_java);
            }
        }

        false
    }
}
