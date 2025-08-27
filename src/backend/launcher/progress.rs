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
    Launching,
    Completed,
}
