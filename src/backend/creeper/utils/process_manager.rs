use tokio::io;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child};

/// Process monitoring configuration.
pub struct ProcessConfig {
    pub success_pattern: String,
    pub error_patterns: Vec<String>,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            success_pattern: "Sound engine started".to_string(),
            error_patterns: vec![
                "Error".to_string(),
                "Exception".to_string(),
                "Failed".to_string(),
            ],
        }
    }
}

/// Process manager utility.
pub struct ProcessManager;

impl ProcessManager {
    /// Monitor a process and wait for a success pattern (без проверки на ошибки).
    pub async fn monitor_process_with_pattern(
        mut child: Child,
        config: ProcessConfig,
    ) -> Result<(bool, String), Box<dyn std::error::Error>> {
        let stdout = match child.stdout.take() {
            Some(s) => Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>,
            None => Box::new(io::empty()) as Box<dyn tokio::io::AsyncRead + Unpin + Send>,
        };
        let stderr = match child.stderr.take() {
            Some(s) => Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>,
            None => Box::new(io::empty()) as Box<dyn tokio::io::AsyncRead + Unpin + Send>,
        };

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut success_detected = false;
        let mut last_output = String::new();

        while !success_detected {
            tokio::select! {
            line = stdout_reader.next_line() => {
                if let Some(line) = line? {
                    println!("{line}");
                    last_output = line.clone();

                    if line.contains(&config.success_pattern) {
                        success_detected = true;
                    }
                }
            }
            line = stderr_reader.next_line() => {
                if let Some(line) = line? {
                    println!("{line}");
                    last_output = line.clone();

                    if line.contains(&config.success_pattern) {
                        success_detected = true;
                    }
                }
            }
            status = child.wait() => {
                let status = status?;
                let exit_code = status.code().unwrap_or(-1);
                return Ok((
                    success_detected,
                    format!("Process exited with code: {exit_code}")
                ));
            }
        }
        }

        Ok((success_detected, last_output))
    }

    /// Monitor a process with the default Minecraft configuration.
    pub async fn monitor_minecraft_process(
        child: Child,
    ) -> Result<(bool, String), Box<dyn std::error::Error>> {
        let config = ProcessConfig {
            success_pattern: "Sound engine started".to_string(),
            error_patterns: vec![
                "Error".to_string(),
                "Exception".to_string(),
                "Failed".to_string(),
                "Could not load".to_string(),
            ],
            ..Default::default()
        };

        Self::monitor_process_with_pattern(child, config).await
    }
}
