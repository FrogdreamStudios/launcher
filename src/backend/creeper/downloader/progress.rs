use console::{style};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

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
        Self {
            current: Arc::new(AtomicU64::new(0)),
            total: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
            last_update: Instant::now(),
            name,
            completed: false,
        }
    }

    pub fn set_total(&mut self, total: u64) {
        self.total.store(total, Ordering::Relaxed);
    }

    pub fn update(&mut self, current: u64) {
        self.current.store(current, Ordering::Relaxed);

        // Only update display every 100ms to avoid spam
        if self.last_update.elapsed() >= Duration::from_millis(100) {
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
            print!("\r{}: {} bytes", style(&self.name).bold(), current);
        } else {
            let percentage = (current as f64 / total as f64 * 100.0) as u8;
            let progress_bar = self.create_progress_bar(percentage);
            let speed = self.calculate_speed(current);

            print!(
                "\r{}: {} [{}] {}/{} {}",
                style(&self.name).bold(),
                style(format!("{percentage}%")).green(),
                progress_bar,
                self.format_bytes(current),
                self.format_bytes(total),
                style(speed).dim()
            );
        }

        let _ = std::io::Write::flush(&mut std::io::stdout());
    }

    fn display_completed(&self) {
        let current = self.current.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed();

        print!(
            "\r{}: {} {} in {:.1}s\n",
            style(&self.name).bold(),
            style("Done").green(),
            self.format_bytes(current),
            elapsed.as_secs_f64()
        );
    }

    fn create_progress_bar(&self, percentage: u8) -> String {
        let width = 20;
        let filled = (percentage as usize * width) / 100;
        let empty = width - filled;

        format!("{}{}", "█".repeat(filled), "░".repeat(empty))
    }

    fn calculate_speed(&self, current: u64) -> String {
        let elapsed = self.start_time.elapsed();
        if elapsed.as_secs() == 0 {
            return "-- B/s".to_string();
        }

        let speed = current as f64 / elapsed.as_secs_f64();
        self.format_speed(speed)
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

    fn format_speed(&self, speed: f64) -> String {
        const UNITS: &[&str] = &["B/s", "KB/s", "MB/s", "GB/s"];
        let mut speed = speed;
        let mut unit_index = 0;

        while speed >= 1024.0 && unit_index < UNITS.len() - 1 {
            speed /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{:.0} {}", speed, UNITS[unit_index])
        } else {
            format!("{:.1} {}", speed, UNITS[unit_index])
        }
    }
}

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        if !self.completed {
            println!();
        }
    }
}

pub struct MultiProgressTracker {
}

impl MultiProgressTracker {
    pub fn new() -> Self {
        Self {
        }
    }
}

impl Default for MultiProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}
