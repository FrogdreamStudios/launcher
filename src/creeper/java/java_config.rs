use std::path::Path;
use tokio::process::Command;
use std::process::Stdio;

/// Configuration for the Java command to launch Minecraft.
/// This struct holds the JVM arguments, platform-specific arguments, and game arguments.
pub struct JavaConfig {
    jvm_args: Vec<String>,
    platform_args: Vec<String>,
    game_args: Vec<(String, String)>,
}

impl JavaConfig {
    /// Creates a new JavaConfig with given Minecraft version.
    pub fn new(version: &str) -> Self {
        let mut jvm_args = vec![
            "-Xmx6G".to_string(),
            "-Xms1G".to_string(),
            "-Djava.library.path=natives".to_string(),
        ];

        #[cfg(target_os = "macos")]
        jvm_args.push("-XstartOnFirstThread".to_string());

        Self {
            jvm_args,
            platform_args: vec![],
            game_args: vec![
                ("--username".to_string(), "Player".to_string()),
                ("--uuid".to_string(), "00000000-0000-0000-0000-000000000000".to_string()),
                ("--accessToken".to_string(), "0".to_string()),
                ("--userType".to_string(), "legacy".to_string()),
                ("--versionType".to_string(), "release".to_string()),
                ("--version".to_string(), version.to_string()),
            ],
        }
    }

    /// Builds the command with a custom Java executable path.
    pub fn build_command_with_executable(
        &self,
        java_executable: &Path,
        classpath: &str,
        main_class: &str,
        minecraft_dir: &Path,
        asset_index_id: &str,
    ) -> Command {
        let mut command = Command::new(java_executable);

        command
            .args(&self.jvm_args)
            .args(&[
                &format!("-Dminecraft.launcher.brand={}", "Dream Launcher"),
                &format!("-Dminecraft.launcher.version={}", "1.0.0"),
                "-cp",
                classpath,
                main_class,
            ])
            .args(self.game_args.iter().flat_map(|(k, v)| vec![k.clone(), v.clone()]))
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
}