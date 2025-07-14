use std::io::{self, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time;

/// A simple progress bar for tracking the completion of tasks.
/// This only works in a terminal environment.
pub struct ProgressBar {
    total: usize,
    completed: Arc<AtomicUsize>,
    title: String,
}

impl ProgressBar {
    pub fn new(total: usize, title: String) -> Self {
        Self {
            total,
            completed: Arc::new(AtomicUsize::new(0)),
            title,
        }
    }

    pub fn increment(&self) {
        self.completed.fetch_add(1, Ordering::SeqCst);
    }

    async fn update_display(&self, current: usize) {
        let percentage = (current as f64 / self.total as f64 * 100.0) as usize;
        let bar_width = 40;
        let filled = (percentage * bar_width) / 100;
        let bar = "█".repeat(filled) + &"░".repeat(bar_width - filled);
        print!("\r{}: [{}] {}/{} ({}%)", self.title, bar, current, self.total, percentage);
        io::stdout().flush().unwrap();
        if current >= self.total {
            println!();
        }
    }

    // Update the progress bar every second
    pub async fn start_periodic_update(&self) {
        let mut interval = time::interval(time::Duration::from_millis(1000));
        loop {
            interval.tick().await;
            let current = self.completed.load(Ordering::SeqCst);
            self.update_display(current).await;
            if current >= self.total {
                break;
            }
        }
    }
}