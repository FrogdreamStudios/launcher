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

/// Helper function to update progress from backend operations.
pub fn update_global_progress(progress: f32, message: String) {
    if let Some(sender) = get_progress_sender() {
        use crate::backend::launcher::progress::ProgressStage;

        let info = ProgressInfo {
            progress,
            message,
            stage: ProgressStage::Launching,
        };
        let _ = sender.send(info);
    }
}
