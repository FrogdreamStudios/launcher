/// Download context for managing file downloads with verification.
use super::{HttpDownloader, models::DownloadTask};
use crate::backend::launcher::downloader::helper::DownloadHelper;
use crate::log_info;
use crate::utils::Result;
use std::path::PathBuf;

/// Download context for managing file downloads with verification.
pub struct DownloadContext<'a> {
    downloader: &'a HttpDownloader,
}

impl<'a> DownloadContext<'a> {
    pub const fn new(downloader: &'a HttpDownloader) -> Self {
        Self { downloader }
    }

    /// Download the file if needed (with verification).
    pub async fn download_if_needed(
        &self,
        url: &str,
        path: &PathBuf,
        expected_size: Option<u64>,
        expected_sha1: Option<&str>,
    ) -> Result<bool> {
        if DownloadHelper::needs_download(path, expected_size, expected_sha1).await? {
            self.downloader
                .download_file(url, path, expected_sha1)
                .await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Execute download tasks with optimized concurrent processing and progress tracking.
    pub async fn execute_downloads(&self, tasks: Vec<DownloadTask>, item_type: &str) -> Result<()> {
        use crate::backend::utils::progress_bridge::update_global_progress;

        if tasks.is_empty() {
            return Ok(());
        }

        let total = tasks.len();
        let batch_size = DownloadHelper::calculate_batch_size(total, 128);

        let max_concurrent = match total {
            0..=50 => 24,
            51..=200 => 48,
            201..=500 => 64,
            _ => 96,
        };

        log_info!(
            "Downloading {total} {item_type} with {max_concurrent} concurrent connections..."
        );

        // Process batches sequentially with high concurrency within each batch
        for (i, chunk) in tasks.chunks(batch_size).enumerate() {
            let completed = i * batch_size;
            let progress_percent = completed as f32 / total as f32;

            // Calculate base progress based on the item type
            let base_progress = match item_type {
                "libraries" => 0.4,
                "natives" => 0.5,
                "assets" => 0.55,
                _ => 0.3,
            };
            let stage_range = 0.05; // Each download stage gets 5% of total progress
            let current_progress = base_progress + (progress_percent * stage_range);

            update_global_progress(
                current_progress,
                format!("Downloading {} ({}/{})", item_type, completed + 1, total),
            );

            // Download batch with high concurrency
            self.downloader
                .download_multiple(chunk.to_vec(), max_concurrent)
                .await?;

            let batch_completed = (i + 1) * chunk.len().min(total - i * batch_size);
            DownloadHelper::log_progress(batch_completed, total, item_type);
        }

        Ok(())
    }
}
