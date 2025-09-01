//! Thread manager that utilizes tokio for asynchronous task management.

use anyhow::Result;
use log::{debug, error, info, trace, warn};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;

/// Message types for inter-task communication.
#[derive(Debug)]
pub enum ArchonMessage {
    /// Python operation request.
    Python {
        operation: String,
        args: Vec<String>,
        response_tx: Option<tokio::sync::oneshot::Sender<PythonResponse>>,
    },
    /// Log message.
    Log {
        level: String,
        message: String,
        target: String,
    },
    /// File operation.
    File {
        operation: String,
        path: String,
        data: Option<Vec<u8>>,
        response_tx: Option<tokio::sync::oneshot::Sender<FileResponse>>,
    },
    /// Network operation.
    Network {
        operation: String,
        url: String,
        data: Option<Value>,
        response_tx: Option<tokio::sync::oneshot::Sender<NetworkResponse>>,
    },
    /// Shutdown signal.
    Shutdown,
}

/// Python operation response.
#[derive(Debug, Clone)]
pub struct PythonResponse {
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
}

/// File operation response.
#[derive(Debug, Clone)]
pub struct FileResponse {
    pub success: bool,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
}

/// Network operation response.
#[derive(Debug, Clone)]
pub struct NetworkResponse {
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
}

/// Thread manager.
#[derive(Debug, Clone)]
pub struct Archon {
    tx: mpsc::UnboundedSender<ArchonMessage>,
    handles: Arc<RwLock<Vec<JoinHandle<()>>>>,
    running_processes: Arc<RwLock<HashMap<u32, tokio::process::Child>>>,
}

impl Archon {
    /// Create a new Archon instance.
    pub async fn new() -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let handles = Arc::new(RwLock::new(Vec::new()));
        let running_processes = Arc::new(RwLock::new(HashMap::new()));

        let archon = Self {
            tx,
            handles: handles.clone(),
            running_processes: running_processes.clone(),
        };

        // Start the main message processing task
        let main_handle = tokio::spawn(Self::message_processor(rx, running_processes));
        handles.write().await.push(main_handle);

        info!("Archon successfully initialized");
        Ok(archon)
    }

    /// Main message processing loop.
    async fn message_processor(
        mut rx: mpsc::UnboundedReceiver<ArchonMessage>,
        running_processes: Arc<RwLock<HashMap<u32, tokio::process::Child>>>,
    ) {
        while let Some(message) = rx.recv().await {
            match message {
                ArchonMessage::Python {
                    operation,
                    args,
                    response_tx,
                } => {
                    let result =
                        Self::handle_python_operation(operation, args, &running_processes).await;
                    if let Some(tx) = response_tx {
                        let _ = tx.send(result);
                    }
                }
                ArchonMessage::Log {
                    level,
                    message,
                    target,
                } => {
                    Self::handle_log_message(level, message, target).await;
                }
                ArchonMessage::File {
                    operation,
                    path,
                    data,
                    response_tx,
                } => {
                    let result = Self::handle_file_operation(operation, path, data).await;
                    if let Some(tx) = response_tx {
                        let _ = tx.send(result);
                    }
                }
                ArchonMessage::Network {
                    operation,
                    url,
                    data,
                    response_tx,
                } => {
                    let result = Self::handle_network_operation(operation, url, data).await;
                    if let Some(tx) = response_tx {
                        let _ = tx.send(result);
                    }
                }
                ArchonMessage::Shutdown => {
                    info!("Archon shutting down...");
                    break;
                }
            }
        }
    }

    /// Handle Python operations.
    async fn handle_python_operation(
        operation: String,
        args: Vec<String>,
        running_processes: &Arc<RwLock<HashMap<u32, tokio::process::Child>>>,
    ) -> PythonResponse {
        match operation.as_str() {
            "launch_minecraft" => {
                if args.len() < 4 {
                    return PythonResponse {
                        success: false,
                        data: None,
                        error: Some("Insufficient arguments for launch_minecraft".to_string()),
                    };
                }

                let username = &args[0];
                let version = &args[1];
                let minecraft_dir = &args[2];
                let game_dir = &args[3];

                match Self::launch_minecraft_process(
                    username,
                    version,
                    minecraft_dir,
                    game_dir,
                    running_processes,
                )
                .await
                {
                    Ok(pid) => PythonResponse {
                        success: true,
                        data: Some(serde_json::json!({ "pid": pid })),
                        error: None,
                    },
                    Err(e) => PythonResponse {
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            "install_minecraft" => {
                if args.len() < 2 {
                    return PythonResponse {
                        success: false,
                        data: None,
                        error: Some("Insufficient arguments for install_minecraft".to_string()),
                    };
                }

                let version = &args[0];
                let minecraft_dir = &args[1];

                match Self::install_minecraft_process(version, minecraft_dir).await {
                    Ok(_) => PythonResponse {
                        success: true,
                        data: None,
                        error: None,
                    },
                    Err(e) => PythonResponse {
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            _ => PythonResponse {
                success: false,
                data: None,
                error: Some(format!("Unknown operation: {operation}")),
            },
        }
    }

    /// Launch Minecraft process.
    async fn launch_minecraft_process(
        username: &str,
        version: &str,
        minecraft_dir: &str,
        game_dir: &str,
        running_processes: &Arc<RwLock<HashMap<u32, tokio::process::Child>>>,
    ) -> Result<u32> {
        let python_script = std::env::current_dir()?.join("python").join("launcher.py");

        let mut cmd = tokio::process::Command::new("python3");
        cmd.arg(python_script)
            .arg("launch")
            .arg(username)
            .arg(version)
            .arg(minecraft_dir)
            .arg(game_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let child = cmd.spawn()?;
        let pid = child.id().unwrap_or(0);

        // Store the process for later management
        running_processes.write().await.insert(pid, child);

        info!("Minecraft launched with PID: {pid}");
        Ok(pid)
    }

    /// Install Minecraft process.
    async fn install_minecraft_process(version: &str, minecraft_dir: &str) -> Result<()> {
        let python_script = std::env::current_dir()?.join("python").join("launcher.py");

        let output = tokio::process::Command::new("python3")
            .arg(python_script)
            .arg("install")
            .arg(version)
            .arg(minecraft_dir)
            .output()
            .await?;

        if output.status.success() {
            info!("Minecraft {version} installed successfully");
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Installation failed: {error}"))
        }
    }

    /// Handle log messages.
    async fn handle_log_message(level: String, message: String, target: String) {
        match level.as_str() {
            "error" => error!(target: &target, "{message}"),
            "warn" => warn!(target: &target, "{message}"),
            "info" => info!(target: &target, "{message}"),
            "debug" => debug!(target: &target, "{message}"),
            "trace" => trace!(target: &target, "{message}"),
            _ => info!(target: &target, "{message}"),
        }
    }

    /// Handle file operations.
    async fn handle_file_operation(
        operation: String,
        path: String,
        data: Option<Vec<u8>>,
    ) -> FileResponse {
        match operation.as_str() {
            "read" => match tokio::fs::read(&path).await {
                Ok(content) => FileResponse {
                    success: true,
                    data: Some(content),
                    error: None,
                },
                Err(e) => FileResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                },
            },
            "write" => {
                if let Some(content) = data {
                    match tokio::fs::write(&path, content).await {
                        Ok(_) => FileResponse {
                            success: true,
                            data: None,
                            error: None,
                        },
                        Err(e) => FileResponse {
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                        },
                    }
                } else {
                    FileResponse {
                        success: false,
                        data: None,
                        error: Some("No data provided for write operation".to_string()),
                    }
                }
            }
            _ => FileResponse {
                success: false,
                data: None,
                error: Some(format!("Unknown file operation: {operation}")),
            },
        }
    }

    /// Handle network operations.
    async fn handle_network_operation(
        operation: String,
        url: String,
        data: Option<Value>,
    ) -> NetworkResponse {
        match operation.as_str() {
            "get" => match reqwest::get(&url).await {
                Ok(response) => match response.json::<Value>().await {
                    Ok(json) => NetworkResponse {
                        success: true,
                        data: Some(json),
                        error: None,
                    },
                    Err(e) => NetworkResponse {
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                },
                Err(e) => NetworkResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                },
            },
            "post" => {
                let client = reqwest::Client::new();
                let mut request = client.post(&url);

                if let Some(json_data) = data {
                    request = request.json(&json_data);
                }

                match request.send().await {
                    Ok(response) => match response.json::<Value>().await {
                        Ok(json) => NetworkResponse {
                            success: true,
                            data: Some(json),
                            error: None,
                        },
                        Err(e) => NetworkResponse {
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                        },
                    },
                    Err(e) => NetworkResponse {
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            _ => NetworkResponse {
                success: false,
                data: None,
                error: Some(format!("Unknown network operation: {operation}")),
            },
        }
    }

    /// Send a message to the Archon.
    pub async fn send(&self, message: ArchonMessage) -> Result<()> {
        self.tx
            .send(message)
            .map_err(|e| anyhow::anyhow!("Failed to send message: {e}"))?;
        Ok(())
    }

    /// Send a Python operation and wait for response.
    pub async fn python_operation(
        &self,
        operation: String,
        args: Vec<String>,
    ) -> Result<PythonResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.send(ArchonMessage::Python {
            operation,
            args,
            response_tx: Some(tx),
        })
        .await?;

        rx.await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {e}"))
    }

    /// Send a file operation and wait for response.
    pub async fn file_operation(
        &self,
        operation: String,
        path: String,
        data: Option<Vec<u8>>,
    ) -> Result<FileResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.send(ArchonMessage::File {
            operation,
            path,
            data,
            response_tx: Some(tx),
        })
        .await?;

        rx.await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {e}"))
    }

    /// Send a network operation and wait for response.
    pub async fn network_operation(
        &self,
        operation: String,
        url: String,
        data: Option<Value>,
    ) -> Result<NetworkResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.send(ArchonMessage::Network {
            operation,
            url,
            data,
            response_tx: Some(tx),
        })
        .await?;

        rx.await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {}", e))
    }

    /// Log a message.
    pub async fn log(&self, level: String, message: String, target: String) -> Result<()> {
        self.send(ArchonMessage::Log {
            level,
            message,
            target,
        })
        .await
    }

    /// Shutdown the Archon.
    pub async fn shutdown(&self) -> Result<()> {
        self.send(ArchonMessage::Shutdown).await?;

        // Wait for all handles to complete
        let handles = self.handles.read().await;
        for handle in handles.iter() {
            if !handle.is_finished() {
                handle.abort();
            }
        }

        // Kill any running processes
        let mut processes = self.running_processes.write().await;
        for (pid, mut child) in processes.drain() {
            info!("Terminating process with PID: {pid}");
            let _ = child.kill().await;
        }

        info!("Archon shutdown complete");
        Ok(())
    }
}
