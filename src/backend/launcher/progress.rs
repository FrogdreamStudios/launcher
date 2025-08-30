//! Progress tracking.

/// Progress information for a specific operation.
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub progress: f32,   // 0.0 to 1.0
    pub message: String, // Current operation description
    pub stage: ProgressStage,
}

/// Different stages of the launch process.
#[derive(Debug, Clone, PartialEq)]
pub enum ProgressStage {
    /// Preparing for installation
    Preparing,
    /// Downloading Minecraft files
    Downloading,
    /// Installing Minecraft version
    Installing,
    /// Launching Minecraft
    Launching,
    /// Minecraft is running
    Running,
    /// Process completed successfully
    Completed,
    /// Process failed
    Failed,
}

impl ProgressStage {
    /// Get the default progress percentage for this stage
    pub fn default_progress(&self) -> f32 {
        match self {
            Self::Preparing => 0.0,
            Self::Downloading => 0.2,
            Self::Installing => 0.6,
            Self::Launching => 0.9,
            Self::Running => 1.0,
            Self::Completed => 1.0,
            Self::Failed => 0.0,
        }
    }

    /// Get the default message for this stage
    pub fn default_message(&self, version: &str) -> String {
        match self {
            Self::Preparing => "Preparing...".to_string(),
            Self::Downloading => format!("Downloading Minecraft {}...", version),
            Self::Installing => format!("Installing Minecraft {}...", version),
            Self::Launching => format!("Launching Minecraft {}...", version),
            Self::Running => format!("Minecraft {} is running", version),
            Self::Completed => "Completed!".to_string(),
            Self::Failed => "Failed!".to_string(),
        }
    }
}
