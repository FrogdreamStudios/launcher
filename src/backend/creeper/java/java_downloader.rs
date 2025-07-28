use dashmap::DashMap;
use reqwest::Client;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs as async_fs;
use crate::backend::creeper::java::models::{VersionJson, VersionManifest};

#[derive(Debug, Clone)]
pub struct JavaVersion {
    pub major_version: u8,
    pub download_url: String,
    pub filename: String,
}

pub struct JavaManager {
    client: Client,
    cache: Arc<DashMap<String, (Vec<u8>, Instant)>>,
}

impl JavaVersion {
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
            8 => ("8.0.362", "8.68.0.19"),
            17 => ("17.0.16", "17.60.17"),
            21 => ("21.0.8", "21.44.17"),
            _ => panic!("Unsupported Java version: {}", major_version),
        };

        let filename = format!(
            "zulu{}-ca-jdk{}-{}.{}",
            build_str, version_str, os_part, extension
        );

        let url = format!("https://cdn.azul.com/zulu/bin/{}", filename);

        (url, filename)
    }
}

impl JavaManager {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            cache: Arc::new(DashMap::new()),
        }
    }

    /// Get the major Java version for a specific Minecraft version using Mojang Meta.
    pub async fn get_java_major(
        &self,
        mc_version: &str,
    ) -> Result<u8, Box<dyn std::error::Error>> {
        // 1. Available versions
        let manifest: VersionManifest = self
            .client
            .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
            .send()
            .await?
            .json()
            .await?;

        // 2. Find the version by ID
        let version = manifest
            .versions
            .iter()
            .find(|v| v.id == mc_version)
            .ok_or("Version not found")?;

        // 3. Get the version JSON
        let version_json = self
            .client
            .get(&version.url)
            .send()
            .await?
            .json::<VersionJson>()
            .await?;

        // 4. Return the major version, otherwise default to 8
        Ok(version_json.java_version.map(|jv| jv.major_version).unwrap_or(8))
    }

    /// JavaVersion using Mojang Meta.
    pub async fn get_java_for_minecraft(
        &self,
        minecraft_version: Option<&str>,
    ) -> Result<JavaVersion, Box<dyn std::error::Error>> {
        let major_version = if let Some(version) = minecraft_version {
            self.get_java_major(version).await?
        } else {
            21 // Default to Java 21 if no version specified
        };
        Ok(JavaVersion::new(major_version))
    }
    
    pub fn is_java_installed(&self, java_version: &JavaVersion) -> bool {
        let java_path = format!("java-{}", java_version.major_version);
        Path::new(&java_path).exists()
    }

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
    
    fn debug_directory_structure(&self, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        println!("Debugging directory structure for: {}", dir.display());

        if !dir.exists() {
            println!("Directory does not exist!");
            return Ok(());
        }

        self.print_dir_structure(dir, 0)?;
        Ok(())
    }
    
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

    // Guarantee that the correct Java version is installed for Minecraft
    pub async fn ensure_java(
        &self,
        minecraft_version: Option<&str>,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let java_version = self.get_java_for_minecraft(minecraft_version).await?;

        println!(
            "Minecraft {} requires Java {}",
            minecraft_version.unwrap_or("(default)"),
            java_version.major_version
        );

        if !self.is_java_installed(&java_version) {
            println!(
                "We don't have Java {}, downloading...",
                java_version.major_version
            );
            self.download_java(&java_version).await?;
        } else {
            println!("Java {} already in use", java_version.major_version);
        }

        self.get_java_path(&java_version)
    }

    // Receive the Java executable path, checking system Java first
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

                let required_java = self.get_java_for_minecraft(minecraft_version).await?;
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
