//! HTTP downloader implementation with progress tracking.

use std::{path::Path, time::Duration};

use crate::backend::utils::stream::ResponseChunkExt;
use crate::utils::Result;
use crate::utils::{Digest, Sha1};
use crate::{log_debug, log_warn, simple_error};
use reqwest::Client;
use tokio::{fs::File, io::AsyncWriteExt};

use super::progress::ProgressTracker;
use crate::backend::{
    creeper::downloader::models::DownloadTask,
    utils::file_utils::{ensure_parent_directory, verify_file},
};

/// HTTP downloader with progress tracking and file verification.
pub struct HttpDownloader {
    client: Client,
}

impl Default for HttpDownloader {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            client: Client::new(),
        })
    }
}

impl HttpDownloader {
    /// Creates a new HTTP downloader with configured timeouts.
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .user_agent("DreamLauncher/1.0.0")
            .build()?;

        Ok(Self { client })
    }

    /// Downloads a single file with optional SHA1 verification and progress tracking.
    pub async fn download_file(
        &self,
        url: &str,
        destination: &Path,
        expected_sha1: Option<&str>,
        mut tracker: Option<&mut ProgressTracker>,
    ) -> Result<()> {
        // Check if the file already exists and has the correct hash
        if verify_file(destination, None, expected_sha1).await? {
            log_debug!("File already exists with correct hash: {destination:?}");
            return Ok(());
        }

        // Create parent directories if they don't exist
        ensure_parent_directory(destination).await?;

        log_debug!("Downloading {url} to {destination:?}");

        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(simple_error!(
                "Failed to download {}: HTTP {}",
                url,
                response.status()
            ));
        }

        let total_size = response.content_length();
        if let (Some(tracker), Some(size)) = (tracker.as_mut(), total_size) {
            tracker.set_total(size);
        }

        let mut file = File::create(destination).await?;
        let mut stream = response.chunk_stream();
        let mut downloaded = 0u64;
        let mut hasher = expected_sha1.map(|_| Sha1::new());

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;

            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            if let Some(ref mut hasher) = hasher {
                hasher.update(&chunk);
            }

            if let Some(tracker) = tracker.as_mut() {
                tracker.update(downloaded);
            }
        }

        file.flush().await?;
        drop(file);

        // Verify file size
        let file_size = tokio::fs::metadata(destination).await?.len();
        log_debug!("Downloaded file size: {file_size} bytes");

        if file_size == 0 {
            tokio::fs::remove_file(destination).await?;
            return Err(simple_error!("Downloaded file is empty: {url}"));
        }

        // Verify archive format for Java downloads
        if (url.contains("java") || destination.to_string_lossy().contains("java"))
            && let Err(e) = self.verify_archive_format(destination).await
        {
            log_warn!("Archive format verification failed for {url}: {e}");
        }

        // Verify SHA1 if provided
        if let (Some(expected), Some(hasher)) = (expected_sha1, hasher) {
            let computed_hash = crate::utils::hex_encode(hasher.finalize());
            if computed_hash != expected {
                tokio::fs::remove_file(destination).await?;
                return Err(simple_error!(
                    "Hash mismatch for {url}: expected {expected}, got {computed_hash}"
                ));
            }
            log_debug!("SHA1 verification passed for {url}");
        }

        Ok(())
    }

    pub async fn download_multiple(
        &self,
        downloads: Vec<DownloadTask>,
        max_concurrent: usize,
    ) -> Result<()> {
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent));
        let mut handles = Vec::new();

        for task in downloads {
            let client = self.client.clone();
            let semaphore = semaphore.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore
                    .acquire()
                    .await
                    .map_err(|_e| simple_error!("Semaphore error: {e}"))?;

                let downloader = Self { client };
                downloader
                    .download_file(
                        &task.url,
                        &task.destination,
                        task.expected_sha1.as_deref(),
                        None,
                    )
                    .await
            });

            handles.push(handle);
        }

        // Wait for all downloads to complete
        for handle in handles {
            handle
                .await
                .map_err(|_e| simple_error!("Task join error: {e}"))??;
        }

        Ok(())
    }

    pub async fn get_json<T>(&self, url: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        const MAX_RETRIES: usize = 3;
        let mut retries = 0;

        loop {
            match self.client.get(url).send().await {
                Ok(response) => match response.status() {
                    reqwest::StatusCode::OK => {
                        let json = response.json::<T>().await?;
                        return Ok(json);
                    }
                    reqwest::StatusCode::TOO_MANY_REQUESTS => {
                        retries += 1;
                        if retries > MAX_RETRIES {
                            return Err(simple_error!(
                                "Failed to fetch {url} after {MAX_RETRIES} retries: HTTP 429 Too Many Requests"
                            ));
                        }

                        let wait_time =
                            Duration::from_secs(2_u64.pow(u32::try_from(retries).unwrap_or(10)));
                        log_warn!(
                            "Rate limited, waiting {wait_time:?} before retry {retries}/{MAX_RETRIES}"
                        );
                        tokio::time::sleep(wait_time).await;
                    }
                    _status => {
                        return Err(simple_error!("HTTP {status} for {url}"));
                    }
                },
                Err(e) => {
                    retries += 1;
                    if retries > MAX_RETRIES {
                        return Err(simple_error!(
                            "Network error after {MAX_RETRIES} retries for {url}: {e}"
                        ));
                    }

                    log_warn!("Network error, retrying {retries}/{MAX_RETRIES} for {url}: {e}");
                    tokio::time::sleep(Duration::from_secs(1 + retries as u64)).await;
                }
            }
        }
    }

    async fn verify_archive_format(&self, file_path: &Path) -> Result<()> {
        use tokio::io::AsyncReadExt;

        let mut file = File::open(file_path).await?;
        let mut header = [0u8; 4];

        if file.read_exact(&mut header).await.is_err() {
            return Err(simple_error!("File too small to read header"));
        }

        // Check for known archive formats
        match &header {
            // ZIP signature
            [0x50, 0x4B, 0x03, 0x04] | [0x50, 0x4B, 0x05, 0x06] | [0x50, 0x4B, 0x07, 0x08] => {
                log_debug!("Detected ZIP archive format");
                Ok(())
            }
            // TAR.GZ signature (gzip magic number)
            [0x1F, 0x8B, _, _] => {
                log_debug!("Detected TAR.GZ archive format");
                Ok(())
            }
            // TAR signature (check at offset 257)
            _ => {
                let mut tar_header = [0u8; 5];
                let _ = file.read_exact(&mut tar_header).await;
                if &tar_header == b"ustar" {
                    log_debug!("Detected TAR archive format");
                    Ok(())
                } else {
                    Err(simple_error!("Unknown archive format"))
                }
            }
        }
    }
}
