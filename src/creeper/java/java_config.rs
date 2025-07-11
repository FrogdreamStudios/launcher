use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// Configuration for the Java command to launch Minecraft.
/// This struct holds the JVM arguments, platform-specific arguments, and game arguments.
pub struct JavaConfig {
    #[allow(dead_code)]
    jvm_args: Vec<String>,
    platform_args: Vec<String>,
    game_args: Vec<(String, String)>,
    version: String,
}

impl JavaConfig {
    /// Creates a new JavaConfig with given Minecraft version.
    pub fn new(version: &str) -> Self {
        let mut jvm_args = vec!["-Xmx6G".to_string(), "-Xms1G".to_string()];

        #[cfg(target_os = "macos")]
        {
            jvm_args.push("-XstartOnFirstThread".to_string());

            // Add additional macOS-specific arguments for older Minecraft versions
            if Self::needs_legacy_macos_args(version) {
                // Core AWT/display settings for proper window creation
                jvm_args.push("-Djava.awt.headless=false".to_string());

                // Apple/macOS specific settings for proper window handling
                jvm_args.push("-Dapple.awt.application.name=Minecraft".to_string());

                // OS compatibility
                jvm_args.push("-Dos.name=Mac OS X".to_string());
                jvm_args.push("-Dos.version=10.15".to_string());

                // Character encoding
                jvm_args.push("-Dfile.encoding=UTF-8".to_string());
            }
        }

        let game_args = Self::get_version_specific_args(version);

        Self {
            jvm_args,
            platform_args: vec![],
            game_args,
            version: version.to_string(),
        }
    }

    /// Gets version-specific game arguments for different Minecraft versions
    fn get_version_specific_args(version: &str) -> Vec<(String, String)> {
        let mut args = vec![
            ("--username".to_string(), "Player".to_string()),
            (
                "--uuid".to_string(),
                "00000000-0000-0000-0000-000000000000".to_string(),
            ),
            ("--accessToken".to_string(), "0".to_string()),
            ("--userType".to_string(), "legacy".to_string()),
            ("--versionType".to_string(), "release".to_string()),
            ("--version".to_string(), version.to_string()),
        ];

        // Add userProperties for older versions that require it
        if Self::needs_user_properties(version) {
            args.push(("--userProperties".to_string(), "{}".to_string()));
        }

        args
    }

    /// Determines if this Minecraft version needs the --userProperties argument.
    fn needs_user_properties(version: &str) -> bool {
        // Parse version to determine if it's an older version that needs userProperties
        if let Some((major, minor, _)) = Self::parse_version_number_static(version) {
            // Versions that need userProperties based on testing and community knowledge:
            // - 1.8.x series definitely needs it
            // - 1.7.x series also needs it
            // - Some 1.6.x versions may need it
            // - Later versions (1.9+) generally don't need it
            if major == 1 {
                if minor <= 8 {
                    return true;
                }
            }
        }

        // For unparseable versions, snapshots, or very old versions, assume they might need it
        // This is safer than missing a required argument
        if version.contains("w")
            || version.contains("pre")
            || version.contains("rc")
            || version.len() < 3
        {
            return true;
        }

        // Special cases for known problematic versions
        if version == "1.6.4" || version == "1.7.2" || version == "1.7.10" {
            return true;
        }

        false
    }

    /// Determines if this Minecraft version needs legacy macOS arguments for window display.
    fn needs_legacy_macos_args(version: &str) -> bool {
        // Parse version to determine if it's an older version that needs legacy macOS args
        if let Some((major, minor, _)) = Self::parse_version_number_static(version) {
            // Versions 1.12.x and earlier have window display issues on modern macOS
            if major == 1 && minor <= 12 {
                return true;
            }
        }

        // Special handling for known problematic versions
        if version == "1.12.2" || version == "1.12.1" || version == "1.12" {
            return true;
        }

        // For unparseable versions or very old versions, assume they might need it
        if version.contains("w") || version.len() < 3 || version.starts_with("1.") {
            // Check if it's a very old version string
            if let Some((major, minor, _)) = Self::parse_version_number_static(version) {
                if major == 1 && minor <= 12 {
                    return true;
                }
            } else {
                // If we can't parse it and it starts with 1., assume it's old
                return version.starts_with("1.");
            }
        }

        false
    }

    /// Builds the command with a custom Java executable path
    pub fn build_command_with_executable(
        &self,
        java_executable: &Path,
        classpath: &str,
        main_class: &str,
        minecraft_dir: &Path,
        asset_index_id: &str,
    ) -> Command {
        let mut command = Command::new(java_executable);

        let mut jvm_args = self.jvm_args.clone();

        // Add natives library path only for versions < 1.21
        if self.needs_natives_directory() {
            let natives_dir = minecraft_dir
                .join("versions")
                .join(&self.version)
                .join("natives");
            jvm_args.push(format!("-Djava.library.path={}", natives_dir.display()));
        }

        command
            .args(&jvm_args)
            .args(&[
                &format!("-Dminecraft.launcher.brand={}", "Dream Launcher"),
                &format!("-Dminecraft.launcher.version={}", "1.0.0"),
                "-cp",
                classpath,
                main_class,
            ])
            .args(
                self.game_args
                    .iter()
                    .flat_map(|(k, v)| vec![k.clone(), v.clone()]),
            )
            .arg("--gameDir")
            .arg(minecraft_dir)
            .arg("--assetsDir")
            .arg(minecraft_dir.join("assets"))
            .arg("--assetIndex")
            .arg(asset_index_id)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        command
    }

    /// Determines if this Minecraft version needs a natives directory.
    /// Returns true for versions < 1.21, false for versions >= 1.21.
    fn needs_natives_directory(&self) -> bool {
        // Parse version string to determine if it's < 1.21
        if let Some(version_number) = self.parse_version_number() {
            version_number < (1, 21, 0)
        } else {
            // If we can't parse the version, assume it needs natives (safer)
            true
        }
    }

    /// Parses the version string into a tuple (major, minor, patch).
    /// Returns None if the version cannot be parsed.
    fn parse_version_number(&self) -> Option<(u32, u32, u32)> {
        Self::parse_version_number_static(&self.version)
    }

    /// Static version of parse_version_number for use in static contexts.
    fn parse_version_number_static(version: &str) -> Option<(u32, u32, u32)> {
        // Handle snapshot versions (e.g., "24w45a" -> treat as 1.21+)
        if version.contains('w') || version.contains("pre") || version.contains("rc") {
            // For snapshots, we'll assume they're recent (1.21+) unless proven otherwise
            if version.starts_with("24w") || version.starts_with("25w") {
                return Some((1, 21, 0)); // Treat recent snapshots as 1.21+
            }
        }

        // Parse normal version strings like "1.16.5", "1.21", "1.21.1"
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() >= 2 {
            if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                let patch = if parts.len() >= 3 {
                    parts[2].parse::<u32>().unwrap_or(0)
                } else {
                    0
                };
                return Some((major, minor, patch));
            }
        }

        None
    }
}
