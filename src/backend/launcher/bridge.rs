//! Rust-Python bridge.

use crate::backend::utils::paths::get_shared_dir;
use crate::backend::utils::python::{get_launcher_script_path, check_python_availability, install_python_dependencies};
use crate::frontend::services::instances::get_instance_directory;
use serde::{Deserialize, Serialize};
use serde_json;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::task;

/// Result of Minecraft launch from Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftLaunchResult {
    pub success: bool,
    pub pid: Option<u32>,
    pub message: String,
}

/// Log message from the Python Minecraft process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MinecraftLogMessage {
    #[serde(rename = "launch_result")]
    LaunchResult {
        success: bool,
        pid: u32,
        message: String,
    },
    #[serde(rename = "log")]
    Log { line: String, pid: u32 },
    #[serde(rename = "exit")]
    Exit {
        pid: u32,
        exit_code: i32,
        message: String,
    },
    #[serde(rename = "error")]
    Error { success: bool, message: String },
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
            username: "Player".to_string(),
            version: "1.21.8".to_string(),
        }
    }
}

/// Python bridge for Minecraft launcher.
pub struct PythonMinecraftBridge {
    python_script_path: PathBuf,
}

impl PythonMinecraftBridge {
    /// Get the appropriate Python command for the current OS.
    fn get_python_command() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        match check_python_availability() {
            Ok(cmd) => Ok(cmd),
            Err(e) => {
                log::error!("Python availability check failed: {e}");
                // Fallback to default commands
                if cfg!(target_os = "windows") {
                    Ok("python".to_string())
                } else {
                    Ok("python3".to_string())
                }
            }
        }
    }

    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Try to install Python dependencies first
        if let Err(e) = install_python_dependencies() {
            log::warn!("Failed to install Python dependencies: {e}");
        }
        
        // Create the temporary launcher script from embedded content
        let temp_launcher = get_launcher_script_path()?;
        let launcher_path = temp_launcher.path().to_path_buf();
        
        // Keep the temporary file alive by storing it in a static location
        // This prevents the file from being deleted while the bridge is in use
        std::mem::forget(temp_launcher);
        
        Ok(Self {
            python_script_path: launcher_path,
        })
    }

    /// Install a specific Minecraft version.
    pub async fn install_version(
        &self,
        version: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let script_path = self.python_script_path.clone();
        let version = version.to_string();
        let minecraft_dir = get_shared_dir()?;

        let result = task::spawn_blocking(
            move || -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
                let python_cmd = Self::get_python_command()?;
                let output = std::process::Command::new(&python_cmd)
                    .arg(&script_path)
                    .arg("install")
                    .arg(&version)
                    .arg(&*minecraft_dir.to_string_lossy())
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

    /// Launch Minecraft with log streaming.
    pub async fn launch_minecraft<F>(
        &self,
        config: LaunchConfig,
        instance_id: u32,
        log_callback: F,
    ) -> Result<i32, Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(MinecraftLogMessage) + Send + 'static,
    {
        let script_path = self.python_script_path.clone();
        let username = config.username.clone();
        let version = config.version.clone();
        let minecraft_dir = get_shared_dir()?;
        tokio::fs::create_dir_all(&minecraft_dir).await?;

        let instance_dir = get_instance_directory(instance_id);
        let game_dir = instance_dir;
        tokio::fs::create_dir_all(&game_dir).await?;

        let python_cmd = Self::get_python_command()?;
        let mut command = TokioCommand::new(&python_cmd)
            .arg(&script_path)
            .arg("launch")
            .arg(&username)
            .arg(&version)
            .arg(&*minecraft_dir.to_string_lossy())
            .arg(&*game_dir.to_string_lossy())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start Python process: {e}"))?;

        let stdout = command.stdout.take().unwrap();
        let reader = BufReader::new(stdout);

        // Read lines from stdout in a separate task
        let handle = tokio::spawn(async move {
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(message) = serde_json::from_str::<MinecraftLogMessage>(&line) {
                    log_callback(message);
                }
            }
        });

        // Wait for the process to complete
        let exit_status = command
            .wait()
            .await
            .map_err(|e| format!("Failed to wait for Python process: {e}"))?;

        // Wait for the log reading task to complete
        let _ = handle.await;

        Ok(exit_status.code().unwrap_or(-1))
    }
}
