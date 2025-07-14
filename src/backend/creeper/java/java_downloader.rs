use dashmap::DashMap;
use reqwest::Client;
use std::fs;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs as async_fs;

#[derive(Debug, Clone)]
pub struct JavaVersion {
    #[allow(dead_code)]
    pub major_version: u8,
    #[allow(dead_code)]
    pub download_url: String,
    #[allow(dead_code)]
    pub filename: String,
}

pub struct JavaManager {
    #[allow(dead_code)]
    version_map: std::collections::HashMap<String, JavaVersion>,
    #[allow(dead_code)]
    default_java: JavaVersion,
    client: Client,
    cache: Arc<DashMap<String, (Vec<u8>, Instant)>>,
}

impl JavaVersion {
    #[allow(dead_code)]
    fn new(major_version: u8) -> Self {
        let (url, filename) = Self::get_download_info(major_version);
        Self {
            major_version,
            download_url: url,
            filename,
        }
    }

    fn get_download_info(major_version: u8) -> (String, String) {
        let (os_part, extension) = if cfg!(target_os = "windows") {
            ("win_x64", "zip")
        } else if cfg!(target_os = "linux") {
            ("linux_x64", "tar.gz")
        } else {
            ("macosx_x64", "tar.gz")
        };

        let (version_str, build_str) = match major_version {
            8 => ("8u392", "8.68.0.21"),
            17 => ("17.0.9", "17.44.17"),
            21 => ("21.0.1", "21.30.15"),
            _ => panic!("Unsupported Java version: {}", major_version),
        };

        let filename = format!(
            "zulu{}-jdk{}_{}.{}",
            build_str, version_str, os_part, extension
        );

        let url = format!("https://cdn.azul.com/zulu/bin/{}", filename);

        (url, filename)
    }
}

impl JavaManager {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let java_versions = [
            (8, Self::get_java8_minecraft_versions()),
            (17, Self::get_java17_minecraft_versions()),
            (21, Self::get_java21_minecraft_versions()),
        ];

        let mut version_map = std::collections::HashMap::new();

        for (java_major, minecraft_versions) in java_versions {
            let java_version = JavaVersion::new(java_major);
            for mc_version in minecraft_versions {
                version_map.insert(mc_version.to_string(), java_version.clone());
            }
        }

        Self {
            version_map,
            default_java: JavaVersion::new(21),
            client: Client::new(),
            cache: Arc::new(DashMap::new()),
        }
    }

    fn get_java8_minecraft_versions() -> Vec<&'static str> {
        vec![
            "1.0", "1.1", "1.2.1", "1.2.2", "1.2.3", "1.2.4", "1.2.5", "1.3.1", "1.3.2", "1.4.2",
            "1.4.4", "1.4.5", "1.4.6", "1.4.7", "1.5", "1.5.1", "1.5.2", "1.6.1", "1.6.2", "1.6.4",
            "1.7.2", "1.7.4", "1.7.5", "1.7.6", "1.7.7", "1.7.8", "1.7.9", "1.7.10", "1.8",
            "1.8.1", "1.8.2", "1.8.3", "1.8.4", "1.8.5", "1.8.6", "1.8.7", "1.8.8", "1.8.9", "1.9",
            "1.9.1", "1.9.2", "1.9.3", "1.9.4", "1.10", "1.10.1", "1.10.2", "1.11", "1.11.1",
            "1.11.2", "1.12", "1.12.1", "1.12.2", "1.13", "1.13.1", "1.13.2", "1.14", "1.14.1",
            "1.14.2", "1.14.3", "1.14.4", "1.15", "1.15.1", "1.15.2", "1.16", "1.16.1", "1.16.2",
            "1.16.3", "1.16.4", "1.16.5",
        ]
    }

    #[allow(dead_code)]
    fn get_java17_minecraft_versions() -> Vec<&'static str> {
        vec![
            "1.17", "1.17.1", "1.18", "1.18.1", "1.18.2", "1.19", "1.19.1", "1.19.2", "1.19.3",
            "1.19.4", "1.20", "1.20.1", "1.20.2", "1.20.3", "1.20.4", "1.20.5", "1.20.6",
        ]
    }

    #[allow(dead_code)]
    fn get_java21_minecraft_versions() -> Vec<&'static str> {
        vec![
            "1.21", "1.21.1", "1.21.2", "1.21.3", "1.21.4", "1.21.5", "1.21.6", "1.21.7",
        ]
    }

    #[allow(dead_code)]
    pub fn get_java_for_minecraft(&self, minecraft_version: Option<&str>) -> &JavaVersion {
        match minecraft_version {
            Some(version) => self.version_map.get(version).unwrap_or(&self.default_java),
            None => &self.default_java,
        }
    }

    #[allow(dead_code)]
    pub fn is_java_installed(&self, java_version: &JavaVersion) -> bool {
        let java_path = format!("java-{}", java_version.major_version);
        Path::new(&java_path).exists()
    }

    #[allow(dead_code)]
    pub async fn download_java(
        &self,
        java_version: &JavaVersion,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Downloading Java {}...", java_version.major_version);

        let java_dir = format!("java-{}", java_version.major_version);
        async_fs::create_dir_all(&java_dir).await?;

        // Check cache first
        let cache_key = java_version.download_url.clone();
        let bytes = if let Some(entry) = self.cache.get(&cache_key) {
            let (cached_bytes, timestamp) = entry.value();
            if timestamp.elapsed() < Duration::from_secs(86400) {
                cached_bytes.clone()
            } else {
                self.fetch_and_cache(&cache_key).await?
            }
        } else {
            self.fetch_and_cache(&cache_key).await?
        };

        async_fs::write(&java_version.filename, &bytes).await?;

        println!(
            "Java {} downloaded to {}",
            java_version.major_version, java_version.filename
        );

        self.extract_java(java_version)?;
        Ok(())
    }

    async fn fetch_and_cache(&self, url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let response = self.client.get(url).send().await?;
        let bytes = response.bytes().await?.to_vec();

        self.cache
            .insert(url.to_string(), (bytes.clone(), Instant::now()));

        Ok(bytes)
    }

    #[allow(dead_code)]
    fn extract_java(&self, java_version: &JavaVersion) -> Result<(), Box<dyn std::error::Error>> {
        println!("Extracting Java {}...", java_version.major_version);

        let java_dir = format!("java-{}", java_version.major_version);
        let output = if java_version.filename.ends_with(".zip") {
            // Windows PowerShit extraction
            Command::new("powershell")
                .arg("-Command")
                .arg(&format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    java_version.filename, java_dir
                ))
                .output()?
        } else {
            // Linux/macOS tar extraction
            Command::new("tar")
                .arg("-xzf")
                .arg(&java_version.filename)
                .arg("-C")
                .arg(&java_dir)
                .output()?
        };

        if !output.status.success() {
            return Err(format!(
                "Extraction error: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        fs::remove_file(&java_version.filename)?;
        println!(
            "Java {} successfully installed in {}",
            java_version.major_version, java_dir
        );
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_java_path(
        &self,
        java_version: &JavaVersion,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let java_dir = format!("java-{}", java_version.major_version);
        let java_exe = if cfg!(target_os = "windows") {
            "java.exe"
        } else {
            "java"
        };

        // Direct path
        let direct_path = Path::new(&java_dir).join("bin").join(java_exe);
        if direct_path.exists() {
            return Ok(direct_path);
        }

        // Recursive search
        if let Some(java_path) = self.find_java_recursive(Path::new(&java_dir), java_exe) {
            println!("Found Java executable at: {}", java_path.display());
            return Ok(java_path);
        }

        // Debug information
        println!("Java executable not found. Directory structure:");
        self.debug_directory_structure(Path::new(&java_dir))?;

        Err(format!("Java executable not found in {}", java_dir).into())
    }

    #[allow(dead_code)]
    fn find_java_recursive(&self, dir: &Path, pattern: &str) -> Option<PathBuf> {
        let entries = fs::read_dir(dir).ok()?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let java_path = path.join("bin").join(pattern);
                if java_path.exists() {
                    return Some(java_path);
                }

                if let Some(found) = self.find_java_recursive(&path, pattern) {
                    return Some(found);
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    fn debug_directory_structure(&self, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        println!("Debugging directory structure for: {}", dir.display());

        if !dir.exists() {
            println!("Directory does not exist!");
            return Ok(());
        }

        self.print_dir_structure(dir, 0)?;
        Ok(())
    }

    #[allow(dead_code)]
    fn print_dir_structure(
        &self,
        dir: &Path,
        depth: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if depth >= 3 {
            // Do not touch this
            return Ok(());
        }

        let indent = "  ".repeat(depth);
        let entries = fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy();

            if path.is_dir() {
                println!("{}DIR: {}", indent, name);
                self.print_dir_structure(&path, depth + 1)?;
            } else {
                println!("{}FILE: {}", indent, name);
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    async fn ensure_java(
        &self,
        minecraft_version: Option<&str>,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let java_version = self.get_java_for_minecraft(minecraft_version);

        println!(
            "Minecraft {} requires Java {}",
            minecraft_version.unwrap_or("(default)"),
            java_version.major_version
        );

        if !self.is_java_installed(java_version) {
            println!(
                "Java {} not found, downloading...",
                java_version.major_version
            );
            self.download_java(java_version).await?;
        } else {
            println!("Java {} already installed", java_version.major_version);
        }

        self.get_java_path(java_version)
    }

    #[allow(dead_code)]
    pub async fn get_java_executable(
        &self,
        minecraft_version: Option<&str>,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        // Check if system Java is available
        if let Ok(output) = Command::new("java").arg("-version").output() {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stderr);
                println!(
                    "Found system Java: {}",
                    version_output.lines().next().unwrap_or("")
                );

                let required_java = self.get_java_for_minecraft(minecraft_version);
                if self.is_system_java_compatible(&version_output, required_java.major_version) {
                    return Ok(PathBuf::from("java"));
                } else {
                    println!(
                        "System Java is not compatible with Minecraft {}",
                        minecraft_version.unwrap_or("(default)")
                    );
                }
            }
        }

        // If system Java is not available or incompatible, download the required version
        self.ensure_java(minecraft_version).await
    }

    // Check if system Java is compatible with the required version
    #[allow(dead_code)]
    fn is_system_java_compatible(&self, version_output: &str, required_major: u8) -> bool {
        if version_output.contains("1.8.") && required_major == 8 {
            return true;
        }

        // Java 9+ versions
        for line in version_output.lines() {
            if line.contains("version") {
                if let Some(version_part) = line.split('"').nth(1) {
                    if let Some(major_str) = version_part.split('.').next() {
                        if let Ok(major) = major_str.parse::<u8>() {
                            return major == required_major;
                        }
                    }
                }
            }
        }

        false
    }
}

impl Default for JavaManager {
    fn default() -> Self {
        Self::new()
    }
}
