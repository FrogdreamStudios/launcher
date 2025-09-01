//! Embedded Python bridge for Minecraft operations.

use anyhow::Result;
use log::{error, info};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;
use std::path::Path;

/// Result of Minecraft launch from embedded Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftLaunchResult {
    pub success: bool,
    pub message: String,
    pub exit_code: Option<i32>,
}

/// Log message from the Minecraft process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MinecraftLogMessage {
    LaunchResult {
        success: bool,
        pid: Option<u32>,
        message: String,
    },
    Log {
        line: String,
        pid: Option<u32>,
    },
    Exit {
        pid: u32,
        exit_code: i32,
        message: String,
    },
    Error {
        success: bool,
        message: String,
    },
}

/// Configuration for launching Minecraft.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchConfig {
    pub username: String,
    pub version: String,
    pub java_path: Option<String>,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    pub access_token: String,
    pub uuid: String,
}

/// Embedded Python bridge for Minecraft operations.
pub struct EmbeddedPythonBridge {
    initialized: bool,
}

impl EmbeddedPythonBridge {
    /// Create a new embedded Python bridge.
    pub fn new() -> anyhow::Result<Self> {
        info!("Initializing Python interpreter");
        Python::initialize();
        Ok(EmbeddedPythonBridge { initialized: true })
    }

    /// Install Python dependencies.
    pub fn install_dependencies() -> anyhow::Result<()> {
        info!("Installing Python dependencies");
        Python::attach(|py| {
            let pip_install = py.import("subprocess")?;
            pip_install.call_method1(
                "run",
                (["pip", "install", "minecraft-launcher-lib"],),
            )?;
            Ok(())
        })
    }

    /// Install a Minecraft version.
    pub fn install_version(&self, version: &str, path: &str) -> Result<()> {
        info!("Installing Minecraft version: {version}");
        Python::attach(|py| {
            let launcher_lib = py.import("minecraft_launcher_lib")?;
            launcher_lib.call_method1("install_minecraft_version", (version, path))?;
            Ok(())
        })
    }

    /// Launch Minecraft with the given configuration.
    pub async fn launch_minecraft<F>(
        &self,
        config: LaunchConfig,
        minecraft_dir: &Path,
        game_dir: &Path,
        log_callback: F,
    ) -> Result<i32>
    where
        F: Fn(MinecraftLogMessage) + Send + 'static,
    {
        if !self.initialized {
            return Err(anyhow::anyhow!("Python bridge not initialized"));
        }

        info!("Launching Minecraft version: {}", config.version);

        // Launch Minecraft using Python script
        let python_script = std::env::current_dir()?.join("python").join("launcher.py");

        if !python_script.exists() {
            return Err(anyhow::anyhow!(
                "Python launcher script not found: {:?}",
                python_script
            ));
        }

        let mut command = tokio::process::Command::new("python3");
        command
            .arg(python_script)
            .arg("launch")
            .arg(&config.username)
            .arg(&config.version)
            .arg(minecraft_dir.to_string_lossy().to_string())
            .arg(game_dir.to_string_lossy().to_string())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        info!("Starting Minecraft process with command: {command:?}");

        let mut child = command
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn Minecraft process: {e}"))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;

        // Read output in the main task to avoid Send issues
        use tokio::io::{AsyncBufReadExt, BufReader};
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        // Spawn a task to read lines and send them through a channel
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let reader_task = tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });

        // Process messages in the main task to avoid Send issues
        tokio::spawn(async move {
            while let Some(line) = rx.recv().await {
                if let Ok(json_msg) = serde_json::from_str::<serde_json::Value>(&line) {
                    match json_msg.get("type").and_then(|t| t.as_str()) {
                        Some("launch_result") => {
                            let success = json_msg
                                .get("success")
                                .and_then(|s| s.as_bool())
                                .unwrap_or(false);
                            let pid = json_msg
                                .get("pid")
                                .and_then(|p| p.as_u64())
                                .map(|p| p as u32);
                            let message = json_msg
                                .get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("")
                                .to_string();

                            log_callback(MinecraftLogMessage::LaunchResult {
                                success,
                                pid,
                                message,
                            });
                        }
                        Some("log") => {
                            let log_line = json_msg
                                .get("line")
                                .and_then(|l| l.as_str())
                                .unwrap_or("")
                                .to_string();
                            let pid = json_msg
                                .get("pid")
                                .and_then(|p| p.as_u64())
                                .map(|p| p as u32);

                            log_callback(MinecraftLogMessage::Log {
                                line: log_line,
                                pid,
                            });
                        }
                        Some("exit") => {
                            let pid = json_msg
                                .get("pid")
                                .and_then(serde_json::Value::as_i64)
                                .unwrap_or(0) as u32;
                            let exit_code = json_msg
                                .get("exit_code")
                                .and_then(serde_json::Value::as_i64)
                                .unwrap_or(0) as i32;
                            let message = json_msg
                                .get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("")
                                .to_string();

                            log_callback(MinecraftLogMessage::Exit {
                                pid,
                                exit_code,
                                message,
                            });
                        }
                        Some("error") => {
                            let message = json_msg
                                .get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("")
                                .to_string();

                            log_callback(MinecraftLogMessage::Error {
                                success: false,
                                message,
                            });
                        }
                        _ => {
                            // Unknown message type, log as regular log line
                            log_callback(MinecraftLogMessage::Log { line, pid: None });
                        }
                    }
                } else {
                    // Not JSON, treat as regular log line
                    log_callback(MinecraftLogMessage::Log { line, pid: None });
                }
            }
        });

        // Wait for the process to complete
        let exit_status = child
            .wait()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to wait for Minecraft process: {}", e))?;

        // Wait for the reader task to complete
        let _ = reader_task.await;

        let exit_code = exit_status.code().unwrap_or(-1);
        info!("Minecraft process exited with code: {exit_code}");

        Ok(exit_code)
    }
}

impl Default for EmbeddedPythonBridge {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            error!("Failed to initialize embedded Python bridge: {e}");
            panic!("Critical error: Cannot initialize Python bridge: {e}");
        })
    }
}
