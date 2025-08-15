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
        let runtime_opt = if needs_x86_64 {
            self.get_compatible_x86_64_runtime(required_java)
        } else {
            self.get_compatible_runtime(required_java)
        };

        if let Some(runtime) = runtime_opt {
            let exe_path = runtime.get_executable_path();
            if exe_path.exists() {
                return Ok((exe_path, needs_x86_64));
            }
            log_info!("Installed Java runtime not found at {exe_path:?}, removing from cache");
            let major_version = runtime.major_version;
            if needs_x86_64 {
                self.x86_64_runtimes.remove(&major_version);
            } else {
                self.installed_runtimes.remove(&major_version);
            }
        }

        // Check system Java (skip for x86_64 requirement as system Java might be ARM64)
        if !needs_x86_64
            && let Ok(Some(mut system_java)) = JavaRuntime::detect_system_java()
            && system_java.is_compatible_with_minecraft(required_java)
        {
            log_info!("Using system Java {} runtime", system_java.major_version);
            if system_java.path.as_os_str().is_empty() {
                system_java.path = crate::utils::which("java").unwrap_or_else(|_| "java".into());
            }
            return Ok((system_java.get_executable_path(), false));
        }

        // Java installation
        let arch = if needs_x86_64 {
            "x64"
        } else {
            AzulPackage::get_arch_name()
        };
        match self.install_java(required_java, arch).await {
            Ok(runtime) => Ok((runtime.get_executable_path(), needs_x86_64)),
            Err(e) => {
                log_warn!("Failed to install Java {required_java} ({arch}): {e}");
                // Fallback to system Java
                if let Ok(Some(mut system_java)) = JavaRuntime::detect_system_java()
                    && system_java.is_compatible_with_minecraft(required_java)
                {
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
                Err(simple_error!(
                    "Failed to install Java {required_java} ({arch}) and no compatible system Java found"
                ))
            }
        }
    }

    // Universal Java runtime
    async fn install_java(&mut self, java_version: u8, arch: &str) -> Result<&JavaRuntime> {
        let manifest = Self::fetch_azul_manifest();
        let os = AzulPackage::get_os_name();

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

        let file_extension = if package.download_url.ends_with(".zip") {
            "zip"
        } else if package.download_url.ends_with(".tar.gz")
            || package.download_url.ends_with(".tgz")
        {
            "tar.gz"
        } else if cfg!(windows) {
            "zip"
        } else {
            "tar.gz"
        };

        let download_path = self.java_dir.join(format!(
            "java-{java_version}-{arch}-download.{file_extension}"
        ));
        let extract_path = self.java_dir.join(format!("java-{java_version}-{arch}"));

        // Downloading
        log_info!(
            "Downloading Java {} from: {}",
            java_version,
            package.download_url
        );
        self.downloader
            .download_file(&package.download_url, &download_path, None)
            .await
            .map_err(|e| {
                let _ = remove_file_if_exists(&download_path);
                log_error!("Failed to download Java {java_version}: {e}");
                e
            })?;

        // Check file size
        let file_size = get_file_size(&download_path).await?;
        log_info!("Downloaded Java {java_version} archive: {file_size} bytes");
        if file_size < 1024 * 1024 {
            let _ = async_fs::remove_file(&download_path).await;
            return Err(simple_error!(
                "Downloaded Java archive is too small, likely corrupted"
            ));
        }

        // Unpacking
        log_info!("Extracting Java {java_version} runtime...");
        extract_archive(&download_path, &extract_path)
            .await
            .map_err(|e| {
                let _ = remove_file_if_exists(&download_path);
                let _ = remove_dir_if_exists(&extract_path);
                log_error!("Failed to extract Java {java_version}: {e}");
                e
            })?;

        remove_file_if_exists(&download_path).await?;

        // Find Java
        let java_executable = Self::find_java_executable(&extract_path)?;
        let runtime = JavaRuntime::from_path(&java_executable)?.ok_or_else(|| {
            simple_error!("Failed to detect installed Java {java_version} runtime")
        })?;

        if arch == "x64" {
            self.x86_64_runtimes.insert(java_version, runtime);
            Ok(self.x86_64_runtimes.get(&java_version).unwrap())
        } else {
            self.installed_runtimes.insert(java_version, runtime);
            Ok(self.installed_runtimes.get(&java_version).unwrap())
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

    fn needs_x86_64_java(minecraft_version: &str) -> bool {
        if std::env::consts::ARCH != "aarch64" || std::env::consts::OS != "macos" {
            return false;
        }
        if Self::is_modern_snapshot_or_version(minecraft_version) {
            return false;
        }
        if let Ok((major, minor, _)) = Self::parse_minecraft_version(minecraft_version) {
            matches!((major, minor), (1, m) if m < 19)
        } else {
            false
        }
    }

    fn is_modern_snapshot_or_version(version: &str) -> bool {
        let version_lower = version.to_lowercase();
        if version_lower.contains('w')
            && version_lower.len() >= 5
            && version_lower
                .get(0..2)
                .and_then(|s| s.parse::<u32>().ok())
                .is_some_and(|year| year >= 21)
        {
            return true;
        }
        if (version_lower.contains("-pre") || version_lower.contains("-rc"))
            && Self::parse_minecraft_version(version_lower.split('-').next().unwrap_or(version))
                .is_ok_and(|parsed| parsed >= (1, 19, 0))
        {
            return true;
        }
        if version_lower.contains("experimental") || version_lower.contains("snapshot") {
            return true;
        }
        Self::parse_minecraft_version(version).map_or(true, |parsed| parsed >= (1, 19, 0))
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
                && Self::find_java_executable(&path)
                    .and_then(|exe| JavaRuntime::from_path(&exe))
                    .ok()
                    .flatten()
                    .is_some()
            {
                let runtime = JavaRuntime::from_path(&Self::find_java_executable(&path)?)?.unwrap();
                let is_x86_64 = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains("-x64"));
                let major_version = runtime.major_version;
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
        Self::create_fallback_manifest()
    }

    fn create_fallback_manifest() -> AzulJavaManifest {
        fn make_package(java_version: u8, os: &str, arch: &str, size: u64) -> AzulPackage {
            let id = format!("zulu{java_version}-{os}-{arch}");
            let name = format!(
                "Zulu {java_version} {} {}",
                os.to_ascii_uppercase(),
                arch.to_ascii_uppercase()
            );
            let base_url = "https://cdn.azul.com/zulu/bin";
            let file = match (os, arch) {
                ("windows", "x64") => format!(
                    "zulu{java_version}.{}-ca-jdk{java_version}.0.{}-win_x64.zip",
                    get_build(java_version),
                    get_patch(java_version)
                ),
                ("macos", "x64") => format!(
                    "zulu{java_version}.{}-ca-jdk{java_version}.0.{}-macosx_x64.tar.gz",
                    get_build(java_version),
                    get_patch(java_version)
                ),
                ("macos", "arm64") => format!(
                    "zulu{java_version}.{}-ca-jdk{java_version}.0.{}-macosx_aarch64.tar.gz",
                    get_build(java_version),
                    get_patch(java_version)
                ),
                ("linux", "x64") => format!(
                    "zulu{java_version}.{}-ca-jdk{java_version}.0.{}-linux_x64.tar.gz",
                    get_build(java_version),
                    get_patch(java_version)
                ),
                _ => unreachable!(),
            };
            AzulPackage {
                id,
                name,
                java_version: vec![java_version],
                os: os.to_string(),
                arch: arch.to_string(),
                download_url: format!("{base_url}/{file}"),
                sha256_hash: String::new(),
                size,
            }
        }
        fn get_build(java_version: u8) -> &'static str {
            match java_version {
                8 => "62.0.19",
                17 => "34.19",
                21 => "36.17",
                _ => "",
            }
        }
        fn get_patch(java_version: u8) -> &'static str {
            match java_version {
                8 => "332",
                17 => "3",
                21 => "4",
                _ => "",
            }
        }
        let mut packages = Vec::new();
        for &java_version in &[8, 17, 21] {
            let size = match java_version {
                8 => 104_857_600,
                17 => 183_500_800,
                21 => 200_000_000,
                _ => 0,
            };
            for &(os, arch) in &[
                ("windows", "x64"),
                ("macos", "x64"),
                ("macos", "arm64"),
                ("linux", "x64"),
            ] {
                if java_version == 8 && arch == "arm64" {
                    continue;
                }
                packages.push(make_package(java_version, os, arch, size));
            }
        }
        AzulJavaManifest { packages }
    }

    fn find_java_executable(java_dir: &Path) -> Result<PathBuf> {
        let executable_name = if cfg!(windows) { "java.exe" } else { "java" };
        let possible_paths = [
            java_dir.join("bin").join(executable_name),
            java_dir
                .join("Contents")
                .join("Home")
                .join("bin")
                .join(executable_name),
        ];
        for path in &possible_paths {
            if path.exists() {
                return Ok(path.clone());
            }
        }
        Self::find_java_recursive(java_dir, executable_name)
    }

    pub fn find_java_recursive(dir: &Path, executable_name: &str) -> Result<PathBuf> {
        for entry in fs::read_dir(dir)
            .map_err(|e| simple_error!("Failed to read dir {}: {}", dir.display(), e))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some(executable_name)
            {
                return Ok(path);
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

        if required_java >= 21 {
            if needs_x86_64 {
                self.get_compatible_x86_64_runtime(required_java).is_some()
            } else {
                self.get_compatible_runtime(required_java).is_some()
            }
        } else if needs_x86_64 {
            self.get_compatible_x86_64_runtime(required_java).is_some()
        } else {
            self.get_compatible_runtime(required_java).is_some()
                || JavaRuntime::detect_system_java()
                    .map(|opt| {
                        opt.is_some_and(|sys| sys.is_compatible_with_minecraft(required_java))
                    })
                    .unwrap_or(false)
        }
    }
}
