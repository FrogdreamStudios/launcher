//! Bridge between backend operations and frontend progress updates.

use crate::backend::launcher::progress::ProgressInfo;
use tokio::sync::mpsc;

/// Global progress sender for launcher operations.
static PROGRESS_SENDER: once_cell::sync::OnceCell<mpsc::UnboundedSender<ProgressInfo>> =
    once_cell::sync::OnceCell::new();

/// Initialize the progress channel and return the receiver.
pub fn init_progress_channel() -> mpsc::UnboundedReceiver<ProgressInfo> {
    let (tx, rx) = mpsc::unbounded_channel();
    let _ = PROGRESS_SENDER.set(tx);
    rx
}

/// Get the progress sender (for backend use).
pub fn get_progress_sender() -> Option<mpsc::UnboundedSender<ProgressInfo>> {
    PROGRESS_SENDER.get().cloned()
}

/// Send progress update with specific stage.
pub fn send_progress_stage(stage: crate::backend::launcher::progress::ProgressStage, version: &str) {
    if let Some(sender) = get_progress_sender() {
        let info = ProgressInfo {
            progress: stage.default_progress(),
            message: stage.default_message(version),
            stage,
        };
        let _ = sender.send(info);
    }
}

/// Send progress update with custom progress and message for a stage.
pub fn send_progress_custom(stage: crate::backend::launcher::progress::ProgressStage, progress: f32, message: String) {
    if let Some(sender) = get_progress_sender() {
        let info = ProgressInfo {
            progress,
            message,
            stage,
        };
        let _ = sender.send(info);
    }
}
