//! Progress tracking utilities for downloads.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::info;

/// Tracks download progress and display updates.
pub struct ProgressTracker {
    current: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
    start_time: Instant,
    last_update: Instant,
    name: String,
    completed: bool,
}

impl ProgressTracker {
    /// Creates a new progress tracker.
    pub fn new(name: String) -> Self {
        let now = Instant::now();
        Self {
            current: Arc::new(AtomicU64::new(0)),
            total: Arc::new(AtomicU64::new(0)),
            start_time: now,
            last_update: now,
            name,
            completed: false,
        }
    }

    /// Sets the total size for progress calculation.
    pub fn set_total(&self, total: u64) {
        self.total.store(total, Ordering::Relaxed);
    }

    /// Updates the current progress amount.
    ///
    /// Only displays progress every 500 ms to avoid spam.
    pub fn update(&mut self, current: u64) {
        self.current.store(current, Ordering::Relaxed);

        // Only update display every 500 ms to avoid spam
        if self.last_update.elapsed() >= Duration::from_millis(500) {
            self.display();
            self.last_update = Instant::now();
        }
    }

    /// Marks the download as completed and shows final statistics.
    pub fn complete(&mut self) {
        if !self.completed {
            self.completed = true;
            let total = self.total.load(Ordering::Relaxed);
            if total > 0 {
                self.current.store(total, Ordering::Relaxed);
            }
            self.display_completed();
        }
    }

    /// Displays current progress information.
    fn display(&self) {
        let current = self.current.load(Ordering::Relaxed);
        let total = self.total.load(Ordering::Relaxed);

        if total == 0 {
            info!("{}: {} bytes", self.name, Self::format_bytes(current));
        } else {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            let percentage = ((current as f64 / total as f64) * 100.0).round() as u8;
            info!(
                "{}: {}% ({}/{})",
                self.name,
                percentage,
                Self::format_bytes(current),
                Self::format_bytes(total)
            );
        }
    }

    /// Displays a completion message with total time elapsed.
    fn display_completed(&self) {
        let current = self.current.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed();

        info!(
            "{}: Complete - {} in {:.1}s",
            self.name,
            Self::format_bytes(current),
            elapsed.as_secs_f64()
        );
    }

    /// Formats byte count into human-readable size.
    fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        #[allow(clippy::cast_precision_loss)]
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                format!("{} {}", size as u64, UNITS[unit_index])
            }
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
}

impl Drop for ProgressTracker {
    /// Automatically completes progress tracking when dropped.
    fn drop(&mut self) {
        if !self.completed {
            self.complete();
        }
    }
}
