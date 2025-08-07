use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;
use tracing::{debug, error, info, warn};

use super::runtime::{AzulJavaManifest, AzulPackage, JavaRuntime};
use crate::backend::creeper::downloader::{HttpDownloader, ProgressTracker};
use crate::backend::utils::paths::get_java_dir;

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
        async_fs::create_dir_all(&java_dir).await?;

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
        let needs_x86_64 = self.needs_x86_64_java(minecraft_version);

        info!(
            "Minecraft version {} requires Java {} (x86_64: {})",
            minecraft_version, required_java, needs_x86_64
        );

        // Check if we already have a compatible runtime
        let invalid_runtime_version = if needs_x86_64 {
            if let Some(runtime) = self.get_compatible_x86_64_runtime(required_java) {
                info!(
                    "Using existing x86_64 Java {} runtime",
                    runtime.major_version
                );
                let exe_path = runtime.get_executable_path();
                if exe_path.exists() {
                    return Ok((exe_path, true));
                } else {
                    info!(
                        "Installed x86_64 Java runtime not found at {:?}, removing from cache",
                        exe_path
                    );
                    Some(runtime.major_version)
                }
            } else {
                None
            }
        } else if let Some(runtime) = self.get_compatible_runtime(required_java) {
            info!("Using existing Java {} runtime", runtime.major_version);
            let exe_path = runtime.get_executable_path();
            if exe_path.exists() {
                return Ok((exe_path, false));
            } else {
                info!(
                    "Installed Java runtime not found at {:?}, removing from cache",
                    exe_path
                );
                Some(runtime.major_version)
            }
        } else {
            None
        };

        // Remove invalid runtime from cache if needed
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
            info!("Using system Java {} runtime", system_java.major_version);
            // Set the correct path for system Java
            if system_java.path.as_os_str().is_empty() {
                system_java.path = which::which("java").unwrap_or_else(|_| "java".into());
            }
            return Ok((system_java.get_executable_path(), false));
        }

        // Download and install required Java
        if needs_x86_64 {
            info!(
                "Downloading x86_64 Java {} runtime for Minecraft {}",
                required_java, minecraft_version
            );

            match self.install_x86_64_java_runtime(required_java).await {
                Ok(()) => {
                    // Get the newly installed runtime
                    if let Some(runtime) = self.x86_64_runtimes.get(&required_java) {
                        Ok((runtime.get_executable_path(), true))
                    } else {
                        Err(anyhow::anyhow!(
                            "Failed to install x86_64 Java {} runtime",
                            required_java
                        ))
                    }
                }
                Err(e) => {
                    warn!("Failed to download x86_64 Java {}: {}", required_java, e);
                    warn!("Attempting to use system Java as fallback...");

                    // Try system Java as fallback
                    if let Some(mut system_java) = JavaRuntime::detect_system_java()? {
                        if system_java.is_compatible_with_minecraft(required_java) {
                            info!(
                                "Using system Java {} as fallback",
                                system_java.major_version
                            );
                            if system_java.path.as_os_str().is_empty() {
                                system_java.path =
                                    which::which("java").unwrap_or_else(|_| "java".into());
                            }
                            return Ok((system_java.get_executable_path(), false));
                        } else {
                            warn!(
                                "System Java {} is not compatible with required Java {}",
                                system_java.major_version, required_java
                            );
                        }
                    }

                    Err(anyhow::anyhow!(
                        "Failed to install x86_64 Java {} and no compatible system Java found",
                        required_java
                    ))
                }
            }
        } else {
            info!(
                "Downloading Java {} runtime for Minecraft {}",
                required_java, minecraft_version
            );

            match self.install_java_runtime(required_java).await {
                Ok(()) => {
                    // Get the newly installed runtime
                    if let Some(runtime) = self.installed_runtimes.get(&required_java) {
                        Ok((runtime.get_executable_path(), false))
                    } else {
                        Err(anyhow::anyhow!(
                            "Failed to install Java {} runtime",
                            required_java
                        ))
                    }
                }
                Err(e) => {
                    warn!("Failed to download native Java {}: {}", required_java, e);

                    // For modern versions requiring Java 21, try x86_64 as fallback
                    if required_java >= 21 {
                        warn!(
                            "Attempting to download x86_64 Java {} as fallback...",
                            required_java
                        );
                        match self.install_x86_64_java_runtime(required_java).await {
                            Ok(()) => {
                                if let Some(runtime) = self.x86_64_runtimes.get(&required_java) {
                                    info!(
                                        "Successfully installed x86_64 Java {} as fallback",
                                        required_java
                                    );
                                    return Ok((runtime.get_executable_path(), true));
                                }
                            }
                            Err(x86_err) => {
                                warn!("x86_64 fallback also failed: {}", x86_err);
                            }
                        }
                    }

                    warn!("Attempting to use system Java as fallback...");

                    // Try system Java as fallback
                    if let Some(mut system_java) = JavaRuntime::detect_system_java()? {
                        if system_java.is_compatible_with_minecraft(required_java) {
                            info!(
                                "Using system Java {} as fallback",
                                system_java.major_version
                            );
                            if system_java.path.as_os_str().is_empty() {
                                system_java.path =
                                    which::which("java").unwrap_or_else(|_| "java".into());
                            }
                            return Ok((system_java.get_executable_path(), false));
                        } else {
                            warn!(
                                "System Java {} is not compatible with required Java {}",
                                system_java.major_version, required_java
                            );
                        }
                    }

                    Err(anyhow::anyhow!(
                        "Failed to install Java {} and no compatible system Java found",
                        required_java
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

    /// Determines if a Minecraft version needs x86_64 Java on Apple Silicon
    /// due to incompatible native libraries.
    fn needs_x86_64_java(&self, minecraft_version: &str) -> bool {
        // Only applies to Apple Silicon (ARM64) systems
        if std::env::consts::ARCH != "aarch64" || std::env::consts::OS != "macos" {
            return false;
        }

        // Handle snapshots and special versions first
        if self.is_modern_snapshot_or_version(minecraft_version) {
            return false; // Modern snapshots support ARM64 natively
        }

        // Parse Minecraft version
        if let Ok((major, minor, _patch)) = self.parse_minecraft_version(minecraft_version) {
            // Versions before 1.19 typically have x86_64-only natives
            // This is a conservative approach - some versions between 1.16-1.19 might work
            matches!((major, minor), (1, m) if m < 19)
        } else {
            // For unknown versions, assume modern (ARM64 native support)
            false
        }
    }

    /// Check if this is a modern snapshot or version that supports ARM64 natively
    fn is_modern_snapshot_or_version(&self, version: &str) -> bool {
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
                self.parse_minecraft_version(version_lower.split('-').next().unwrap_or(version))
        {
            return parsed >= (1, 19, 0); // 1.19+ support ARM64
        }

        // Handle experimental and special versions
        if version_lower.contains("experimental") || version_lower.contains("snapshot") {
            return true; // Assume modern experimental versions support ARM64
        }

        // Check if it's a regular version that supports ARM64
        if let Ok(parsed) = self.parse_minecraft_version(version) {
            return parsed >= (1, 19, 0);
        }

        // Default to modern for unknown versions
        true
    }

    fn parse_minecraft_version(&self, version: &str) -> Result<(u8, u8, u8)> {
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
            Err(anyhow::anyhow!(
                "Invalid Minecraft version format: {}",
                version
            ))
        }
    }

    pub async fn install_java_runtime(&mut self, java_version: u8) -> Result<()> {
        let manifest = self.fetch_azul_manifest()?;

        let os = AzulPackage::get_os_name();
        let arch = AzulPackage::get_arch_name();

        let package = manifest
            .packages
            .iter()
            .find(|pkg| pkg.matches_requirements(java_version, os, arch))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No Azul {} Java {} package found for {} {}",
                    arch,
                    java_version,
                    os,
                    arch
                )
            })?;

        info!(
            "Found Java {} package: {} ({} MB)",
            java_version,
            package.name,
            package.size / 1024 / 1024
        );

        // Determine file extension from URL
        let file_extension = if package.download_url.ends_with(".zip") {
            "zip"
        } else if package.download_url.ends_with(".tar.gz")
            || package.download_url.ends_with(".tgz")
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
        info!(
            "Downloading Java {} from: {}",
            java_version, package.download_url
        );

        let mut progress = ProgressTracker::new(format!("Java {java_version}"));
        if let Err(e) = self
            .downloader
            .download_file(
                &package.download_url,
                &download_path,
                None, // Disable hash verification for now
                Some(&mut progress),
            )
            .await
        {
            error!("Failed to download Java {}: {}", java_version, e);
            if download_path.exists() {
                let _ = async_fs::remove_file(&download_path).await;
            }
            return Err(e);
        }

        // Verify downloaded file
        let file_size = async_fs::metadata(&download_path).await?.len();
        info!("Downloaded Java {java_version} archive: {file_size} bytes");

        if file_size < 1024 * 1024 {
            error!("Downloaded file is too small ({file_size}B), likely corrupted",);
            let _ = async_fs::remove_file(&download_path).await;
            return Err(anyhow::anyhow!(
                "Downloaded Java archive is too small, likely corrupted"
            ));
        }

        // Extract the package
        info!("Extracting Java {java_version} runtime...");
        if let Err(e) = self
            .extract_java_archive(&download_path, &extract_path)
            .await
        {
            error!("Failed to extract Java {}: {}", java_version, e);
            let _ = async_fs::remove_file(&download_path).await;
            let _ = async_fs::remove_dir_all(&extract_path).await;
            return Err(e);
        }

        // Clean up download file
        if download_path.exists() {
            async_fs::remove_file(&download_path).await?;
        }

        // Detect the extracted runtime
        let java_executable = self.find_java_executable(&extract_path)?;
        if let Some(runtime) = JavaRuntime::from_path(&java_executable)? {
            self.installed_runtimes.insert(java_version, runtime);
            info!("Successfully installed Java {} runtime", java_version);
        } else {
            error!("Failed to detect installed Java {} runtime", java_version);
        }

        Ok(())
    }

    pub async fn install_x86_64_java_runtime(&mut self, java_version: u8) -> Result<()> {
        let manifest = self.fetch_azul_manifest()?;

        let os = AzulPackage::get_os_name();
        let _arch = "x64"; // Force x86_64 architecture

        let package = manifest
            .packages
            .iter()
            .find(|pkg| pkg.matches_requirements(java_version, os, "x64"))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No Azul x64 Java {} package found for {} x64",
                    java_version,
                    os
                )
            })?;

        info!(
            "Found x86_64 Java {} package: {} ({} MB)",
            java_version,
            package.name,
            package.size / 1024 / 1024
        );

        // Determine file extension from URL
        let file_extension = if package.download_url.ends_with(".zip") {
            "zip"
        } else if package.download_url.ends_with(".tar.gz")
            || package.download_url.ends_with(".tgz")
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
        info!(
            "Downloading x86_64 Java {} from: {}",
            java_version, package.download_url
        );
        let mut progress = ProgressTracker::new(format!("x86_64 Java {java_version}"));

        if let Err(e) = self
            .downloader
            .download_file(
                &package.download_url,
                &download_path,
                None, // Disable hash verification for now
                // TODO: add hash verification
                Some(&mut progress),
            )
            .await
        {
            error!("Failed to download x86_64 Java {}: {}", java_version, e);
            if download_path.exists() {
                let _ = async_fs::remove_file(&download_path).await;
            }
            return Err(e);
        }

        // Verify downloaded file
        let file_size = async_fs::metadata(&download_path).await?.len();
        info!(
            "Downloaded x86_64 Java {} archive: {} bytes",
            java_version, file_size
        );

        if file_size < 1024 * 1024 {
            error!(
                "Downloaded file is too small ({}B), likely corrupted",
                file_size
            );
            let _ = async_fs::remove_file(&download_path).await;
            return Err(anyhow::anyhow!(
                "Downloaded x86_64 Java archive is too small, likely corrupted"
            ));
        }

        // Extract the package
        info!("Extracting Java {java_version} runtime...");
        if let Err(e) = self
            .extract_java_archive(&download_path, &extract_path)
            .await
        {
            error!("Failed to extract Java {}: {}", java_version, e);
            let _ = async_fs::remove_file(&download_path).await;
            let _ = async_fs::remove_dir_all(&extract_path).await;
            return Err(e);
        }

        // Clean up download file
        if download_path.exists() {
            async_fs::remove_file(&download_path).await?;
        }

        // Detect the extracted runtime
        let java_executable = self.find_java_executable(&extract_path)?;
        if let Some(runtime) = JavaRuntime::from_path(&java_executable)? {
            self.x86_64_runtimes.insert(java_version, runtime);
            info!(
                "Successfully installed x86_64 Java {} runtime",
                java_version
            );
        } else {
            error!(
                "Failed to detect installed x86_64 Java {} runtime",
                java_version
            );
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
                && let Ok(java_executable) = self.find_java_executable(&path)
                && let Some(runtime) = JavaRuntime::from_path(&java_executable)?
            {
                let is_x86_64 = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains("-x64"));

                debug!(
                    "Found installed {} Java {} runtime at {:?}",
                    if is_x86_64 { "x86_64" } else { "native" },
                    runtime.major_version,
                    path
                );

                if is_x86_64 {
                    self.x86_64_runtimes.insert(runtime.major_version, runtime);
                } else {
                    self.installed_runtimes
                        .insert(runtime.major_version, runtime);
                }
            }
        }

        info!(
            "Found {} native and {} x86_64 installed Java runtimes",
            self.installed_runtimes.len(),
            self.x86_64_runtimes.len()
        );
        Ok(())
    }

    fn fetch_azul_manifest(&self) -> Result<AzulJavaManifest> {
        let _manifest_url = "https://api.azul.com/zulu/download/community/v1.0/bundles/";

        Ok(self.create_fallback_manifest())
    }

    fn create_fallback_manifest(&self) -> AzulJavaManifest {
        let packages = vec![
            // Java 8 packages
            AzulPackage {
                id: "zulu8-windows-x64".to_string(),
                name: "Zulu 8 Windows x64".to_string(),
                java_version: vec![8],
                os: "windows".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu8.62.0.19-ca-jdk8.0.332-win_x64.zip".to_string(),
                sha256_hash: "".to_string(),
                size: 104_857_600,
            },
            AzulPackage {
                id: "zulu8-macos-x64".to_string(),
                name: "Zulu 8 macOS x64".to_string(),
                java_version: vec![8],
                os: "macos".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu8.62.0.19-ca-jdk8.0.332-macosx_x64.tar.gz".to_string(),
                sha256_hash: "".to_string(),
                size: 104_857_600,
            },
            AzulPackage {
                id: "zulu8-macos-arm64".to_string(),
                name: "Zulu 8 macOS ARM64".to_string(),
                java_version: vec![8],
                os: "macos".to_string(),
                arch: "arm64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu8.62.0.19-ca-jdk8.0.332-macosx_aarch64.tar.gz".to_string(),
                sha256_hash: "".to_string(),
                size: 104_857_600,
            },
            AzulPackage {
                id: "zulu8-linux-x64".to_string(),
                name: "Zulu 8 Linux x64".to_string(),
                java_version: vec![8],
                os: "linux".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu8.62.0.19-ca-jdk8.0.332-linux_x64.tar.gz".to_string(),
                sha256_hash: "".to_string(),
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
                sha256_hash: "".to_string(),
                size: 183_500_800,
            },
            AzulPackage {
                id: "zulu17-macos-x64".to_string(),
                name: "Zulu 17 macOS x64".to_string(),
                java_version: vec![17],
                os: "macos".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu17.34.19-ca-jdk17.0.3-macosx_x64.tar.gz".to_string(),
                sha256_hash: "".to_string(),
                size: 183_500_800,
            },
            AzulPackage {
                id: "zulu17-macos-arm64".to_string(),
                name: "Zulu 17 macOS ARM64".to_string(),
                java_version: vec![17],
                os: "macos".to_string(),
                arch: "arm64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu17.34.19-ca-jdk17.0.3-macosx_aarch64.tar.gz".to_string(),
                sha256_hash: "".to_string(),
                size: 183_500_800,
            },
            AzulPackage {
                id: "zulu17-linux-x64".to_string(),
                name: "Zulu 17 Linux x64".to_string(),
                java_version: vec![17],
                os: "linux".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu17.34.19-ca-jdk17.0.3-linux_x64.tar.gz".to_string(),
                sha256_hash: "".to_string(),
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
                sha256_hash: "".to_string(),
                size: 200_000_000,
            },
            AzulPackage {
                id: "zulu21-macos-x64".to_string(),
                name: "Zulu 21 macOS x64".to_string(),
                java_version: vec![21],
                os: "macos".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu21.36.17-ca-jdk21.0.4-macosx_x64.tar.gz".to_string(),
                sha256_hash: "".to_string(),
                size: 200_000_000,
            },
            AzulPackage {
                id: "zulu21-macos-arm64".to_string(),
                name: "Zulu 21 macOS ARM64".to_string(),
                java_version: vec![21],
                os: "macos".to_string(),
                arch: "arm64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu21.36.17-ca-jdk21.0.4-macosx_aarch64.tar.gz".to_string(),
                sha256_hash: "".to_string(),
                size: 200_000_000,
            },
            AzulPackage {
                id: "zulu21-linux-x64".to_string(),
                name: "Zulu 21 Linux x64".to_string(),
                java_version: vec![21],
                os: "linux".to_string(),
                arch: "x64".to_string(),
                download_url: "https://cdn.azul.com/zulu/bin/zulu21.36.17-ca-jdk21.0.4-linux_x64.tar.gz".to_string(),
                sha256_hash: "".to_string(),
                size: 200_000_000,
            },
        ];

        AzulJavaManifest { packages }
    }

    async fn extract_java_archive(&self, archive_path: &Path, extract_path: &Path) -> Result<()> {
        async_fs::create_dir_all(extract_path).await?;

        let filename = archive_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        info!("Extracting archive: {}", filename);

        // Check for compound extensions first, then single extensions
        if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
            info!("Detected TAR.GZ format");
            self.extract_tar_gz(archive_path, extract_path).await
        } else if filename.ends_with(".zip") {
            info!("Detected ZIP format");
            self.extract_zip(archive_path, extract_path).await
        } else {
            // Fallback to single extension check
            let extension = archive_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            match extension {
                "zip" => self.extract_zip(archive_path, extract_path).await,
                "gz" => self.extract_tar_gz(archive_path, extract_path).await,
                _ => {
                    // Try to detect format by reading file header
                    self.extract_archive_by_content(archive_path, extract_path)
                        .await
                }
            }
        }
    }

    async fn extract_archive_by_content(
        &self,
        archive_path: &Path,
        extract_path: &Path,
    ) -> Result<()> {
        // Read first few bytes to detect file type
        let mut file = async_fs::File::open(archive_path).await?;
        let mut header = [0u8; 4];

        use tokio::io::AsyncReadExt;
        file.read_exact(&mut header).await?;

        // ZIP files start with "PK" (0x504B)
        if header[0] == 0x50 && header[1] == 0x4B {
            info!("Detected ZIP format by header");
            self.extract_zip(archive_path, extract_path).await
        }
        // GZIP files start with 0x1F 0x8B
        else if header[0] == 0x1F && header[1] == 0x8B {
            info!("Detected GZIP format by header");
            self.extract_tar_gz(archive_path, extract_path).await
        } else {
            Err(anyhow::anyhow!(
                "Unsupported or corrupted archive format. Header bytes: {:02X} {:02X} {:02X} {:02X}",
                header[0],
                header[1],
                header[2],
                header[3]
            ))
        }
    }

    async fn extract_zip(&self, archive_path: &Path, extract_path: &Path) -> Result<()> {
        use std::io::Read;

        let file = std::fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = extract_path.join(file.mangled_name());

            if file.name().ends_with('/') {
                async_fs::create_dir_all(&outpath).await?;
            } else {
                if let Some(p) = outpath.parent() {
                    async_fs::create_dir_all(p).await?;
                }

                let mut outfile = async_fs::File::create(&outpath).await?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;

                use tokio::io::AsyncWriteExt;
                outfile.write_all(&buffer).await?;

                // Set executable permissions on Unix systems
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if file.unix_mode().unwrap_or(0) & 0o111 != 0 {
                        let metadata = std::fs::metadata(&outpath)?;
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o755);
                        std::fs::set_permissions(&outpath, perms)?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn extract_tar_gz(&self, archive_path: &Path, extract_path: &Path) -> Result<()> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let file = std::fs::File::open(archive_path)?;
        let gz = GzDecoder::new(file);
        let mut archive = Archive::new(gz);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            let target_path = extract_path.join(&*path);

            if let Some(parent) = target_path.parent() {
                async_fs::create_dir_all(parent).await?;
            }

            entry.unpack(&target_path)?;
        }

        Ok(())
    }

    fn find_java_executable(&self, java_dir: &Path) -> Result<PathBuf> {
        let executable_name = if cfg!(windows) { "java.exe" } else { "java" };

        // Look in common locations within the Java installation
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
        for entry in
            fs::read_dir(dir).with_context(|| format!("Failed to read dir: {}", dir.display()))?
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

        Err(anyhow::anyhow!(
            "Java executable not found in {}",
            dir.display()
        ))
    }

    #[allow(dead_code)]
    pub fn list_installed_runtimes(&self) -> &HashMap<u8, JavaRuntime> {
        &self.installed_runtimes
    }

    #[allow(dead_code)]
    pub async fn remove_runtime(&mut self, java_version: u8) -> Result<()> {
        let runtime_dir = self.java_dir.join(format!("java-{java_version}"));
        let x64_runtime_dir = self.java_dir.join(format!("java-{java_version}-x64"));

        if runtime_dir.exists() {
            async_fs::remove_dir_all(&runtime_dir).await?;
            self.installed_runtimes.remove(&java_version);
            info!("Removed Java {java_version} runtime");
        }

        if x64_runtime_dir.exists() {
            async_fs::remove_dir_all(&x64_runtime_dir).await?;
            self.x86_64_runtimes.remove(&java_version);
            info!("Removed x86_64 Java {} runtime", java_version);
        }

        Ok(())
    }

    pub fn is_java_available(&self, minecraft_version: &str) -> bool {
        let required_java = JavaRuntime::get_required_java_version(minecraft_version);
        let needs_x86_64 = self.needs_x86_64_java(minecraft_version);

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
