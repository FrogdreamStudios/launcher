use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaRuntime {
    pub path: PathBuf,
    pub version: String,
    pub major_version: u8,
    pub vendor: String,
    pub architecture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzulJavaManifest {
    pub packages: Vec<AzulPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzulPackage {
    pub id: String,
    pub name: String,
    pub java_version: Vec<u8>,
    pub os: String,
    pub arch: String,
    pub download_url: String,
    pub sha256_hash: String,
    pub size: u64,
}

impl JavaRuntime {
    pub fn detect_system_java() -> Result<Option<Self>> {
        // Try to find java using which/where
        let java_path = if cfg!(windows) {
            which::which("java.exe").or_else(|_| which::which("java"))
        } else {
            which::which("java")
        };

        match java_path {
            Ok(path) => {
                let output = Command::new(&path).args(["-version"]).output();
                match output {
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        Self::parse_java_version(&stderr).map(|opt| {
                            opt.map(|mut runtime| {
                                runtime.path = path;
                                runtime
                            })
                        })
                    }
                    Err(_) => {
                        debug!("Failed to execute java at {:?}", path);
                        Ok(None)
                    }
                }
            }
            Err(_) => {
                debug!("System Java not found in PATH");
                Ok(None)
            }
        }
    }

    pub fn from_path<P: AsRef<Path>>(java_path: P) -> Result<Option<Self>> {
        let java_path = java_path.as_ref();

        if !java_path.exists() {
            return Ok(None);
        }

        let output = Command::new(java_path).args(["-version"]).output()?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        Self::parse_java_version(&stderr).map(|opt| {
            opt.map(|mut runtime| {
                // If java_path is already the executable, store the parent directory
                // If it's a directory, store it as is
                runtime.path = if java_path.is_file() {
                    // For executable files like /path/bin/java, store /path
                    java_path
                        .parent()
                        .and_then(|p| p.parent())
                        .unwrap_or(java_path.parent().unwrap_or(java_path))
                        .to_path_buf()
                } else {
                    java_path.to_path_buf()
                };
                runtime
            })
        })
    }

    fn parse_java_version(version_output: &str) -> Result<Option<Self>> {
        let lines: Vec<&str> = version_output.lines().collect();
        if lines.is_empty() {
            return Ok(None);
        }

        // Parse version line (first line)
        let version_line = lines[0];
        let version = Self::extract_version_number(version_line)?;
        let major_version = Self::get_major_version(&version);

        // Parse vendor (usually second line)
        let vendor = lines
            .get(1)
            .and_then(|line| Self::extract_vendor(line))
            .unwrap_or_else(|| "Unknown".to_string());

        // Detect architecture
        let architecture = Self::detect_architecture();

        Ok(Some(JavaRuntime {
            path: PathBuf::new(),
            version,
            major_version,
            vendor,
            architecture,
        }))
    }

    fn extract_version_number(version_line: &str) -> Result<String> {
        if let Some(start) = version_line.find('"') {
            if let Some(end) = version_line[start + 1..].find('"') {
                return Ok(version_line[start + 1..start + 1 + end].to_string());
            }
        }

        Err(anyhow::anyhow!(
            "Could not parse Java version from: {}",
            version_line
        ))
    }
    
    
    
    /// Extract the major version from a Java version string.
    fn get_major_version(version: &str) -> u8 {
        if version.starts_with("1.") {
            // Java 8 and below use format "1.X.Y"
            version
                .chars()
                .nth(2)
                .and_then(|c| c.to_digit(10))
                .unwrap_or(8) as u8
        } else {
            // Java 9+ use format "X.Y.Z"
            version
                .split('.')
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8)
        }
    }

    fn extract_vendor(vendor_line: &str) -> Option<String> {
        if vendor_line.contains("OpenJDK") {
            Some("OpenJDK".to_string())
        } else if vendor_line.contains("Oracle") {
            Some("Oracle".to_string())
        } else if vendor_line.contains("Azul") {
            Some("Azul".to_string())
        } else if vendor_line.contains("Eclipse") {
            Some("Eclipse Adoptium".to_string())
        } else {
            None
        }
    }

    fn detect_architecture() -> String {
        match std::env::consts::ARCH {
            "x86_64" => "x64".to_string(),
            "aarch64" => "arm64".to_string(),
            "x86" => "x86".to_string(),
            arch => arch.to_string(),
        }
    }

    pub fn is_compatible_with_minecraft(&self, required_major: u8) -> bool {
        // Java compatibility ranges for Minecraft versions
        match required_major {
            8 => self.major_version >= 8 && self.major_version <= 21, // Java 8-21 for older versions
            17 => self.major_version >= 17 && self.major_version <= 21, // Java 17-21 for newer versions
            21 => self.major_version >= 21 && self.major_version <= 25, // Java 21+ for latest versions
            _ => self.major_version >= required_major && self.major_version <= 21, // Default range
        }
    }

    pub fn get_executable_path(&self) -> PathBuf {
        // If path is already a java executable, return it directly
        if self.path.file_name() == Some(std::ffi::OsStr::new("java"))
            || self.path.file_name() == Some(std::ffi::OsStr::new("java.exe"))
        {
            return self.path.clone();
        }

        // Otherwise, assume it's a Java home directory
        if cfg!(windows) {
            self.path.join("bin").join("java.exe")
        } else {
            self.path.join("bin").join("java")
        }
    }

    /// Get required Java version for Minecraft version.
    pub fn get_required_java_version(minecraft_version: &str) -> u8 {
        // Parse version to determine required Java
        if let Ok(version) = Self::parse_minecraft_version(minecraft_version) {
            match version {
                // Minecraft 1.21+ requires Java 21
                v if v >= (1, 21, 0) => 21,
                // Minecraft 1.20.5+ requires Java 21
                v if v >= (1, 20, 5) => 21,
                // Minecraft 1.18+ requires Java 17
                v if v >= (1, 18, 0) => 17,
                // Minecraft 1.17+ requires Java 16/17
                v if v >= (1, 17, 0) => 17,
                // Minecraft 1.12-1.16 works with Java 8
                v if v >= (1, 12, 0) => 8,
                // Older versions (1.11 and below) use Java 8
                _ => 8,
            }
        } else {
            8 // Default to Java 8 for unknown versions
        }
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
            Err(anyhow::anyhow!(
                "Invalid Minecraft version format: {}",
                version
            ))
        }
    }
}

impl AzulPackage {
    pub fn matches_requirements(&self, java_version: u8, os: &str, arch: &str) -> bool {
        self.java_version.contains(&java_version) && self.os == os && self.arch == arch
    }

    pub fn get_os_name() -> &'static str {
        match std::env::consts::OS {
            "windows" => "windows",
            "macos" => "macos",
            "linux" => "linux",
            _ => "linux", // Default to linux for unknown OS
        }
    }

    pub fn get_arch_name() -> &'static str {
        match std::env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            "x86" => "x86",
            _ => "x64", // Default to x64
        }
    }
}
