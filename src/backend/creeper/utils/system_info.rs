use std::env;

/// Represents the current operating system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatingSystem {
    Windows,
    Linux,
    MacOS,
    Unknown,
}

/// System information utility.
pub struct SystemInfo;

impl SystemInfo {
    /// Get the current operating system.
    pub fn get_os() -> OperatingSystem {
        match env::consts::OS {
            "windows" => OperatingSystem::Windows,
            "linux" => OperatingSystem::Linux,
            "macos" => OperatingSystem::MacOS,
            _ => OperatingSystem::Unknown,
        }
    }

    /// Get OS name in Minecraft format.
    pub fn get_minecraft_os() -> String {
        match Self::get_os() {
            OperatingSystem::Windows => "windows".to_string(),
            OperatingSystem::Linux => "linux".to_string(),
            OperatingSystem::MacOS => "osx".to_string(),
            OperatingSystem::Unknown => "unknown".to_string(),
        }
    }

    /// Get OS name for Java downloads.
    pub fn get_java_os() -> String {
        match Self::get_os() {
            OperatingSystem::Windows => "win_x64".to_string(),
            OperatingSystem::Linux => "linux_x64".to_string(),
            OperatingSystem::MacOS => "macosx_x64".to_string(),
            OperatingSystem::Unknown => "unknown".to_string(),
        }
    }

    /// Get file extension for Java downloads.
    pub fn get_java_extension() -> String {
        match Self::get_os() {
            OperatingSystem::Windows => "zip".to_string(),
            OperatingSystem::Linux | OperatingSystem::MacOS => "tar.gz".to_string(),
            OperatingSystem::Unknown => "zip".to_string(),
        }
    }

    /// Check if the current OS is macOS.
    pub fn is_macos() -> bool {
        Self::get_os() == OperatingSystem::MacOS
    }
}
