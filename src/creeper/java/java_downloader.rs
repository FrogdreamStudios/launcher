use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct JavaVersion {
    pub major_version: u8,
    pub download_url: &'static str,
    pub filename: &'static str,
}

pub struct JavaManager {
    version_map: HashMap<String, JavaVersion>,
    default_java: JavaVersion,
}

impl JavaManager {
    pub fn new() -> Self {
        let mut version_map = HashMap::new();

        // Java 8 URLs (OpenJDK)
        let java8 = if cfg!(target_os = "windows") {
            JavaVersion {
                major_version: 8,
                download_url: "https://github.com/adoptium/temurin8-binaries/releases/download/jdk8u392-b08/OpenJDK8U-jdk_x64_windows_hotspot_8u392b08.zip",
                filename: "openjdk-8.zip",
            }
        } else if cfg!(target_os = "linux") {
            JavaVersion {
                major_version: 8,
                download_url: "https://github.com/adoptium/temurin8-binaries/releases/download/jdk8u392-b08/OpenJDK8U-jdk_x64_linux_hotspot_8u392b08.tar.gz",
                filename: "openjdk-8.tar.gz",
            }
        } else {
            JavaVersion {
                major_version: 8,
                download_url: "https://github.com/adoptium/temurin8-binaries/releases/download/jdk8u392-b08/OpenJDK8U-jdk_x64_mac_hotspot_8u392b08.tar.gz",
                filename: "openjdk-8.tar.gz",
            }
        };

        // Java 17 URLs (OpenJDK)
        let java17 = if cfg!(target_os = "windows") {
            JavaVersion {
                major_version: 17,
                download_url: "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.9%2B9/OpenJDK17U-jdk_x64_windows_hotspot_17.0.9_9.zip",
                filename: "openjdk-17.zip",
            }
        } else if cfg!(target_os = "linux") {
            JavaVersion {
                major_version: 17,
                download_url: "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.9%2B9/OpenJDK17U-jdk_x64_linux_hotspot_17.0.9_9.tar.gz",
                filename: "openjdk-17.tar.gz",
            }
        } else {
            JavaVersion {
                major_version: 17,
                download_url: "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.9%2B9/OpenJDK17U-jdk_x64_mac_hotspot_17.0.9_9.tar.gz",
                filename: "openjdk-17.tar.gz",
            }
        };

        // Java 21 URLs (OpenJDK)
        let java21 = if cfg!(target_os = "windows") {
            JavaVersion {
                major_version: 21,
                download_url: "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.1%2B12/OpenJDK21U-jdk_x64_windows_hotspot_21.0.1_12.zip",
                filename: "openjdk-21.zip",
            }
        } else if cfg!(target_os = "linux") {
            JavaVersion {
                major_version: 21,
                download_url: "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.1%2B12/OpenJDK21U-jdk_x64_linux_hotspot_21.0.1_12.tar.gz",
                filename: "openjdk-21.tar.gz",
            }
        } else {
            JavaVersion {
                major_version: 21,
                download_url: "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.1%2B12/OpenJDK21U-jdk_x64_mac_hotspot_21.0.1_12.tar.gz",
                filename: "openjdk-21.tar.gz",
            }
        };

        // Java 8 versions (Minecraft 1.0 to 1.16)
        // TODO: add more versions
        let java8_versions = vec![
            "1.0", "1.1", "1.2.1", "1.2.2", "1.2.3", "1.2.4", "1.2.5",
            "1.3.1", "1.3.2", "1.4.2", "1.4.4", "1.4.5", "1.4.6", "1.4.7",
            "1.5", "1.5.1", "1.5.2", "1.6.1", "1.6.2", "1.6.4",
            "1.7.2", "1.7.4", "1.7.5", "1.7.6", "1.7.7", "1.7.8", "1.7.9", "1.7.10",
            "1.8", "1.8.1", "1.8.2", "1.8.3", "1.8.4", "1.8.5", "1.8.6", "1.8.7", "1.8.8", "1.8.9",
            "1.9", "1.9.1", "1.9.2", "1.9.3", "1.9.4",
            "1.10", "1.10.1", "1.10.2",
            "1.11", "1.11.1", "1.11.2",
            "1.12", "1.12.1", "1.12.2",
            "1.13", "1.13.1", "1.13.2",
            "1.14", "1.14.1", "1.14.2", "1.14.3", "1.14.4",
            "1.15", "1.15.1", "1.15.2",
            "1.16", "1.16.1", "1.16.2", "1.16.3", "1.16.4", "1.16.5",
        ];

        // Java 17 versions (Minecraft 1.17 and above)
        let java17_versions = vec![
            "1.17", "1.17.1",
            "1.18", "1.18.1", "1.18.2",
            "1.19", "1.19.1", "1.19.2", "1.19.3", "1.19.4",
            "1.20", "1.20.1", "1.20.2", "1.20.3", "1.20.4", "1.20.5", "1.20.6",
        ];

        // Java 21 versions (Minecraft 1.21 and above)
        let java21_versions = vec![
            "1.21", "1.21.1", "1.21.2", "1.21.3", "1.21.4", "1.21.5", "1.21.6", "1.21.7",
        ];

        // Fill the version map with Java versions
        for version in java8_versions {
            version_map.insert(version.to_string(), java8.clone());
        }

        for version in java17_versions {
            version_map.insert(version.to_string(), java17.clone());
        }

        for version in java21_versions {
            version_map.insert(version.to_string(), java21.clone());
        }

        Self {
            version_map,
            default_java: java21,
        }
    }

    pub fn get_java_for_minecraft(&self, minecraft_version: Option<&str>) -> &JavaVersion {
        match minecraft_version {
            Some(version) => self.version_map.get(version).unwrap_or(&self.default_java),
            None => &self.default_java,
        }
    }

    pub fn is_java_installed(&self, java_version: &JavaVersion) -> bool {
        let java_path = format!("java-{}", java_version.major_version);
        Path::new(&java_path).exists()
    }

    pub async fn download_java(&self, java_version: &JavaVersion) -> Result<(), Box<dyn std::error::Error>> {
        println!("Downloading Java {}...", java_version.major_version);

        // TODO: change this
        // Create a directory for the Java version
        let java_dir = format!("java-{}", java_version.major_version);
        fs::create_dir_all(&java_dir)?;

        // Load the Java archive
        // Response from the URL and save it to a file
        let response = reqwest::get(java_version.download_url).await?;
        let bytes = response.bytes().await?;

        // Save the downloaded file
        let mut file = fs::File::create(java_version.filename)?;
        file.write_all(&bytes)?;

        println!("Java {} downloaded to {}", java_version.major_version, java_version.filename);

        // Extracting the downloaded Java archive
        self.extract_java(java_version)?;

        Ok(())
    }

    fn extract_java(&self, java_version: &JavaVersion) -> Result<(), Box<dyn std::error::Error>> {
        println!("Extracting Java {}...", java_version.major_version);

        let java_dir = format!("java-{}", java_version.major_version);

        if java_version.filename.ends_with(".zip") {
            // For Windows, use PowerShit to extract the ZIP file
            if cfg!(target_os = "windows") {
                let output = Command::new("powershell")
                    .arg("-Command")
                    .arg(&format!(
                        "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                        java_version.filename, java_dir
                    ))
                    .output()?;

                if !output.status.success() {
                    return Err(format!("Extraction error: {}", String::from_utf8_lossy(&output.stderr)).into());
                }
            }
        } else if java_version.filename.ends_with(".tar.gz") {
            // For Linux and macOS, use tar to extract
            let output = Command::new("tar")
                .arg("-xzf")
                .arg(java_version.filename)
                .arg("-C")
                .arg(&java_dir)
                .output()?;

            if !output.status.success() {
                return Err(format!("Extraction error: {}", String::from_utf8_lossy(&output.stderr)).into());
            }
        }

        // Delete garbage archive file
        fs::remove_file(java_version.filename)?;

        println!("Java {} successfully installed in {}", java_version.major_version, java_dir);
        Ok(())
    }

    pub fn get_java_path(&self, java_version: &JavaVersion) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let java_dir = format!("java-{}", java_version.major_version);

        // Name of the Java executable based on the OS
        let pattern = if cfg!(target_os = "windows") {
            "java.exe"
        } else {
            "java"
        };

        // FUNCTION to find Java executable recursively
        fn find_java_recursive(dir: &Path, pattern: &str) -> Option<PathBuf> {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        // Check if the directory contains the Java executable
                        let java_path = path.join("bin").join(pattern);
                        if java_path.exists() {
                            return Some(java_path);
                        }

                        // Recursively search in subdirectories
                        if let Some(found) = find_java_recursive(&path, pattern) {
                            return Some(found);
                        }
                    }
                }
            }
            None
        }

        // Check if the Java directory in general exists
        let direct_path = Path::new(&java_dir).join("bin").join(pattern);
        if direct_path.exists() {
            return Ok(direct_path);
        }

        // Find Java executable recursively
        if let Some(java_path) = find_java_recursive(Path::new(&java_dir), pattern) {
            println!("Found Java executable at: {}", java_path.display());
            return Ok(java_path);
        }

        // Debugging information
        println!("Java executable not found. Directory structure:");
        self.debug_directory_structure(Path::new(&java_dir))?;

        Err(format!("Java executable not found in {}", java_dir).into())
    }

    fn debug_directory_structure(&self, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        println!("Debugging directory structure for: {}", dir.display());

        if !dir.exists() {
            println!("Directory does not exist!");
            return Ok(());
        }

        fn print_dir_structure(dir: &Path, depth: usize) -> Result<(), Box<dyn std::error::Error>> {
            let indent = "  ".repeat(depth);
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries {
                    let entry = entry?;
                    let path = entry.path();
                    let name = path.file_name().unwrap().to_string_lossy();

                    if path.is_dir() {
                        println!("{}DIR: {}", indent, name);

                        // DO NOT CHANGE THIS
                        if depth < 3 {
                            print_dir_structure(&path, depth + 1)?;
                        }
                    } else {
                        println!("{}FILE: {}", indent, name);
                    }
                }
            }

            Ok(())
        }

        print_dir_structure(dir, 0)?;
        Ok(())


    }

    async fn ensure_java(&self, minecraft_version: Option<&str>) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let java_version = self.get_java_for_minecraft(minecraft_version);

        println!("Minecraft {} requires Java {}",
                 minecraft_version.unwrap_or("(default)"),
                 java_version.major_version);

        if !self.is_java_installed(java_version) {
            println!("Java {} not found, downloading...", java_version.major_version);
            self.download_java(java_version).await?;
        } else {
            println!("Java {} already installed", java_version.major_version);
        }

        self.get_java_path(java_version)
    }

    pub async fn get_java_executable(&self, minecraft_version: Option<&str>) -> Result<PathBuf, Box<dyn std::error::Error>> {
        // Check if system Java is available
        if let Ok(output) = Command::new("java").arg("-version").output() {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stderr);
                println!("Found system Java: {}", version_output.lines().next().unwrap_or(""));

                // Check if system Java is compatible with the required version
                let required_java = self.get_java_for_minecraft(minecraft_version);
                if self.is_system_java_compatible(&version_output, required_java.major_version) {
                    return Ok(PathBuf::from("java"));
                } else {
                    println!("System Java is not compatible with Minecraft {}",
                             minecraft_version.unwrap_or("(default)"));
                }
            }
        }

        // If system Java is not compatible or not found, download the required version
        self.ensure_java(minecraft_version).await
    }

    /// Check system Java compatibility with a Minecraft version
    fn is_system_java_compatible(&self, version_output: &str, required_major: u8) -> bool {
        if version_output.contains("1.8.") && required_major == 8 {
            return true;
        }

        // Java 9 and above. Fucking hell, why they did it?
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

// Note for me: use this #[test] thing and test Java
// #[cfg(test)], etc.