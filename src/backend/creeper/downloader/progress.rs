use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::info;

pub struct ProgressTracker {
    current: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
    start_time: Instant,
    last_update: Instant,
    name: String,
    completed: bool,
}

impl ProgressTracker {
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

    pub fn set_total(&self, total: u64) {
        self.total.store(total, Ordering::Relaxed);
    }

    pub fn update(&mut self, current: u64) {
        self.current.store(current, Ordering::Relaxed);

        // Only update display every 500ms to avoid spam
        if self.last_update.elapsed() >= Duration::from_millis(500) {
            self.display();
            self.last_update = Instant::now();
        }
    }

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

    fn display(&self) {
        let current = self.current.load(Ordering::Relaxed);
        let total = self.total.load(Ordering::Relaxed);

        if total == 0 {
            info!("{}: {} bytes", self.name, self.format_bytes(current));
        } else {
            let percentage = (current as f64 / total as f64 * 100.0).round() as u8;
            info!(
                "{}: {}% ({}/{})",
                self.name,
                percentage,
                self.format_bytes(current),
                self.format_bytes(total)
            );
        }
    }

    fn display_completed(&self) {
        let current = self.current.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed();

        info!(
            "{}: Complete - {} in {:.1}s",
            self.name,
            self.format_bytes(current),
            elapsed.as_secs_f64()
        );
    }

    fn format_bytes(&self, bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
}

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        if !self.completed {
            self.complete();
        }
    }
}
