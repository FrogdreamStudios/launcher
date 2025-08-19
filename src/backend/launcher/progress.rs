//! Progress tracking and callback system for launcher operations.

use std::sync::Arc;
use tokio::sync::RwLock;

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
    Initializing,
    CheckingJava,
    InstallingJava,
    UpdatingManifest,
    DownloadingVersion,
    DownloadingLibraries,
    DownloadingAssets,
    ExtractingNatives,
    PreparingLaunch,
    Launching,
    Completed,
}

impl ProgressStage {
    /// Get a human-readable description of the stage.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Initializing => "Initializing launch...",
            Self::CheckingJava => "Checking Java installation...",
            Self::InstallingJava => "Installing Java runtime...",
            Self::UpdatingManifest => "Updating version manifest...",
            Self::DownloadingVersion => "Downloading game version...",
            Self::DownloadingLibraries => "Downloading libraries...",
            Self::DownloadingAssets => "Downloading game assets...",
            Self::ExtractingNatives => "Extracting native libraries...",
            Self::PreparingLaunch => "Preparing game launch...",
            Self::Launching => "Starting Minecraft...",
            Self::Completed => "Launch completed",
        }
    }

    /// Get the base progress value for this stage (0.0 to 1.0).
    pub fn base_progress(&self) -> f32 {
        match self {
            Self::Initializing => 0.0,
            Self::CheckingJava => 0.05,
            Self::InstallingJava => 0.1,
            Self::UpdatingManifest => 0.15,
            Self::DownloadingVersion => 0.2,
            Self::DownloadingLibraries => 0.4,
            Self::DownloadingAssets => 0.6,
            Self::ExtractingNatives => 0.8,
            Self::PreparingLaunch => 0.9,
            Self::Launching => 0.95,
            Self::Completed => 1.0,
        }
    }

    /// Get the progress range for this stage.
    pub fn progress_range(&self) -> f32 {
        match self {
            Self::Initializing => 0.05,
            Self::CheckingJava => 0.05,
            Self::InstallingJava => 0.05,
            Self::UpdatingManifest => 0.05,
            Self::DownloadingVersion => 0.2,
            Self::DownloadingLibraries => 0.2,
            Self::DownloadingAssets => 0.2,
            Self::ExtractingNatives => 0.1,
            Self::PreparingLaunch => 0.05,
            Self::Launching => 0.05,
            Self::Completed => 0.0,
        }
    }
}

/// Trait for progress callbacks.
pub trait ProgressCallback: Send + Sync {
    /// Called when progress is updated.
    fn on_progress(&self, info: ProgressInfo);

    /// Called when an error occurs.
    fn on_error(&self, error: String);

    /// Called when the operation completes successfully.
    fn on_complete(&self);
}

/// A thread-safe progress tracker.
#[derive(Clone)]
pub struct ProgressTracker {
    callback: Option<Arc<dyn ProgressCallback>>,
    current_stage: Arc<RwLock<ProgressStage>>,
    stage_progress: Arc<RwLock<f32>>, // 0.0 to 1.0 within the current stage
}

impl ProgressTracker {
    /// Create a new progress tracker.
    pub fn new(callback: Option<Arc<dyn ProgressCallback>>) -> Self {
        Self {
            callback,
            current_stage: Arc::new(RwLock::new(ProgressStage::Initializing)),
            stage_progress: Arc::new(RwLock::new(0.0)),
        }
    }

    /// Create a progress tracker without callback (for internal use).
    pub fn silent() -> Self {
        Self::new(None)
    }

    /// Set the current stage.
    pub async fn set_stage(&self, stage: ProgressStage) {
        *self.current_stage.write().await = stage.clone();
        *self.stage_progress.write().await = 0.0;

        if let Some(callback) = &self.callback {
            callback.on_progress(ProgressInfo {
                progress: stage.base_progress(),
                message: stage.description().to_string(),
                stage,
            });
        }
    }

    /// Set the current stage with a custom message.
    pub async fn set_stage_with_message(&self, stage: ProgressStage, message: String) {
        *self.current_stage.write().await = stage.clone();
        *self.stage_progress.write().await = 0.0;

        if let Some(callback) = &self.callback {
            callback.on_progress(ProgressInfo {
                progress: stage.base_progress(),
                message,
                stage,
            });
        }
    }

    /// Update progress within the current stage.
    pub async fn update_stage_progress(&self, progress: f32) {
        let progress = progress.clamp(0.0, 1.0);
        *self.stage_progress.write().await = progress;

        if let Some(callback) = &self.callback {
            let stage = self.current_stage.read().await.clone();
            let overall_progress = stage.base_progress() + (stage.progress_range() * progress);

            callback.on_progress(ProgressInfo {
                progress: overall_progress,
                message: stage.description().to_string(),
                stage,
            });
        }
    }

    /// Update progress with a custom message.
    pub async fn update_with_message(&self, progress: f32, message: String) {
        let progress = progress.clamp(0.0, 1.0);
        *self.stage_progress.write().await = progress;

        if let Some(callback) = &self.callback {
            let stage = self.current_stage.read().await.clone();
            let overall_progress = stage.base_progress() + (stage.progress_range() * progress);

            callback.on_progress(ProgressInfo {
                progress: overall_progress,
                message,
                stage,
            });
        }
    }

    /// Update progress for downloading with count information.
    pub async fn update_download_progress(
        &self,
        downloaded: usize,
        total: usize,
        current_file: &str,
    ) {
        if total == 0 {
            return;
        }

        let progress = downloaded as f32 / total as f32;
        let message = if total > 1 {
            format!(
                "Downloading {} ({}/{})",
                current_file,
                downloaded + 1,
                total
            )
        } else {
            format!("Downloading {}", current_file)
        };

        self.update_with_message(progress, message).await;
    }

    /// Report an error.
    pub fn report_error(&self, error: String) {
        if let Some(callback) = &self.callback {
            callback.on_error(error);
        }
    }

    /// Report completion.
    pub async fn complete(&self) {
        self.set_stage(ProgressStage::Completed).await;
        if let Some(callback) = &self.callback {
            callback.on_complete();
        }
    }

    /// Get current overall progress (0.0 to 1.0).
    pub async fn get_progress(&self) -> f32 {
        let stage = self.current_stage.read().await.clone();
        let stage_progress = *self.stage_progress.read().await;
        stage.base_progress() + (stage.progress_range() * stage_progress)
    }

    /// Get current stage.
    pub async fn get_stage(&self) -> ProgressStage {
        self.current_stage.read().await.clone()
    }
}

/// Helper for calculating download progress based on file sizes.
#[derive(Debug)]
pub struct DownloadProgressCalculator {
    total_bytes: u64,
    downloaded_bytes: u64,
    file_count: usize,
    completed_files: usize,
}

impl DownloadProgressCalculator {
    /// Create a new download progress calculator.
    pub fn new(total_bytes: u64, file_count: usize) -> Self {
        Self {
            total_bytes,
            downloaded_bytes: 0,
            file_count,
            completed_files: 0,
        }
    }

    /// Update progress for a file download.
    pub fn update_file_progress(&mut self, file_bytes: u64) {
        self.downloaded_bytes += file_bytes;
        self.completed_files += 1;
    }

    /// Get current progress (0.0 to 1.0).
    pub fn get_progress(&self) -> f32 {
        if self.total_bytes == 0 {
            if self.file_count == 0 {
                0.0
            } else {
                self.completed_files as f32 / self.file_count as f32
            }
        } else {
            self.downloaded_bytes as f32 / self.total_bytes as f32
        }
    }

    /// Get progress message.
    pub fn get_message(&self, current_file: &str) -> String {
        if self.file_count > 1 {
            format!(
                "Downloading {} ({}/{}) - {:.1} MB / {:.1} MB",
                current_file,
                self.completed_files + 1,
                self.file_count,
                self.downloaded_bytes as f32 / 1024.0 / 1024.0,
                self.total_bytes as f32 / 1024.0 / 1024.0
            )
        } else {
            format!(
                "Downloading {} - {:.1} MB / {:.1} MB",
                current_file,
                self.downloaded_bytes as f32 / 1024.0 / 1024.0,
                self.total_bytes as f32 / 1024.0 / 1024.0
            )
        }
    }
}

impl Default for DownloadProgressCalculator {
    fn default() -> Self {
        Self::new(0, 0)
    }
}
