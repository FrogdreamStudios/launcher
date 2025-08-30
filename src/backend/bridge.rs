//! Python bridge for Minecraft launcher integration.

use serde::{Deserialize, Serialize};
use serde_json;
use std::path::PathBuf;
use tokio::task;

/// Result of Minecraft launch from Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftLaunchResult {
    pub success: bool,
    pub pid: Option<u32>,
    pub message: String,
}

/// Configuration for Minecraft launch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchConfig {
    pub username: String,
    pub version: String,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            username: "TestPlayer".to_string(),
            version: "1.20.1".to_string(),
        }
    }
}

/// Python bridge for Minecraft launcher.
pub struct PythonMinecraftBridge {
    python_script_path: PathBuf,
}

impl PythonMinecraftBridge {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Try multiple possible paths for the Python script
        let possible_paths = vec![
            // Current directory + python/launcher.py
            std::env::current_dir()?.join("python").join("launcher.py"),
            // Relative to executable + python/launcher.py
            std::env::current_exe()?
                .parent()
                .unwrap()
                .join("python")
                .join("launcher.py"),
            // Launcher subdirectory + python/launcher.py
            std::env::current_dir()?
                .join("launcher")
                .join("python")
                .join("launcher.py"),
        ];

        for script_path in possible_paths {
            if script_path.exists() {
                return Ok(Self {
                    python_script_path: script_path,
                });
            }
        }

        Err("Python script launcher.py not found in any expected location".into())
    }

    /// Launch Minecraft through Python script with command line arguments.
    pub async fn launch_minecraft(
        &self,
        config: LaunchConfig,
    ) -> Result<MinecraftLaunchResult, Box<dyn std::error::Error + Send + Sync>> {
        use std::process::Command;

        let script_path = self.python_script_path.clone();
        let username = config.username.clone();
        let version = config.version.clone();

        // Execute Python script with command line arguments
        let result = task::spawn_blocking(
            move || -> Result<MinecraftLaunchResult, Box<dyn std::error::Error + Send + Sync>> {
                let output = Command::new("python3")
                    .arg(&script_path)
                    .arg("--username")
                    .arg(&username)
                    .arg("--version")
                    .arg(&version)
                    .arg("--install") // Auto-install version if needed
                    .output()?;

                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);

                    // Try to parse JSON output from the last line
                    if let Some(last_line) = stdout.lines().last() {
                        if let Ok(result) = serde_json::from_str::<MinecraftLaunchResult>(last_line)
                        {
                            return Ok(result);
                        }
                    }

                    // Fallback if JSON parsing fails
                    Ok(MinecraftLaunchResult {
                        success: true,
                        pid: None,
                        message: format!("Minecraft {version} launched successfully"),
                    })
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Ok(MinecraftLaunchResult {
                        success: false,
                        pid: None,
                        message: format!("Failed to launch Minecraft: {stderr}"),
                    })
                }
            },
        )
        .await??;

        Ok(result)
    }

    /// Get full version manifest from Mojang.
    pub async fn get_version_manifest(
        &self,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let script_path = self.python_script_path.clone();

        let result = task::spawn_blocking(
            move || -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
                let output = std::process::Command::new("python3")
                    .arg(&script_path)
                    .arg("--get-manifest")
                    .output()?;

                if !output.status.success() {
                    let error = String::from_utf8_lossy(&output.stderr);
                    return Err(format!("Failed to get manifest: {error}").into());
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                let manifest: serde_json::Value = serde_json::from_str(&stdout)
                    .map_err(|e| format!("Failed to parse manifest JSON: {e}"))?;

                Ok(manifest)
            },
        )
        .await??;

        Ok(result)
    }

    /// Install a specific Minecraft version.
    pub async fn install_version(
        &self,
        version: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let script_path = self.python_script_path.clone();
        let version = version.to_string();

        let result = task::spawn_blocking(
            move || -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
                let output = std::process::Command::new("python3")
                    .arg(&script_path)
                    .arg("--install")
                    .arg("--version")
                    .arg(&version)
                    .output()?;

                if !output.status.success() {
                    let error = String::from_utf8_lossy(&output.stderr);
                    return Err(format!("Failed to install version {version}: {error}").into());
                }

                Ok(true)
            },
        )
        .await??;

        Ok(result)
    }
}
