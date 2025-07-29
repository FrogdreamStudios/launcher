use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

use crate::backend::creeper::utils::version_checker::{
    needs_legacy_macos_args, needs_user_properties, parse_version_number,
};

/// Configuration for the Java command to launch Minecraft.
#[allow(dead_code)]
pub struct JavaConfig {
    jvm_args: Vec<String>,
    platform_args: Vec<String>,
    game_args: Vec<(String, String)>,
    version: String,
}

impl JavaConfig {
    pub fn new(version: &str) -> Self {
        let mut jvm_args = vec!["-Xmx6G".to_string(), "-Xms1G".to_string()];

        #[cfg(target_os = "macos")]
        {
            jvm_args.push("-XstartOnFirstThread".to_string());

            if needs_legacy_macos_args(version) {
                jvm_args.extend(
                    [
                        "-Djava.awt.headless=false",
                        "-Dapple.awt.application.name=Minecraft",
                        "-Dos.name=Mac OS X",
                        "-Dos.version=10.15",
                        "-Dfile.encoding=UTF-8",
                    ]
                    .iter()
                    .map(|s| s.to_string()),
                );
            }
        }

        let mut game_args = JavaConfig::get_version_specific_args(version);

        #[cfg(target_os = "macos")]
        {
            if needs_legacy_macos_args(version) {
                game_args.extend([
                    ("--width".to_string(), "854".to_string()),
                    ("--height".to_string(), "480".to_string()),
                ]);

                jvm_args.push("-Dorg.lwjgl.opengl.Display.allowSoftwareOpenGL=true".to_string());
            }
        }

        Self {
            jvm_args,
            platform_args: vec![],
            game_args,
            version: version.to_string(),
        }
    }

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

        if needs_user_properties(version) {
            args.push(("--userProperties".to_string(), "{}".to_string()));
        }

        args
    }

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

        if self.needs_natives_directory() {
            let natives_dir = minecraft_dir
                .join("versions")
                .join(&self.version)
                .join("natives");
            jvm_args.push(format!("-Djava.library.path={}", natives_dir.display()));
        }

        command
            .args(&jvm_args)
            .args([
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
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        command
    }

    fn needs_natives_directory(&self) -> bool {
        self.parse_version_number()
            .map(|v| v < (1, 21, 0))
            .unwrap_or(true)
    }

    fn parse_version_number(&self) -> Option<(u32, u32, u32)> {
        parse_version_number(&self.version)
    }
}
