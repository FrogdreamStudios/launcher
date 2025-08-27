//! HTTP downloader implementation with progress tracking.

use crate::utils::{Digest, Result, Sha1};
use std::{path::Path, time::Duration};
use tokio::{fs::File, io::AsyncWriteExt};

use crate::backend::launcher::downloader::models::DownloadTask;
use crate::backend::utils::net::http::{Client, StatusCode};
use crate::backend::utils::net::stream::ResponseChunkExt;
use crate::backend::utils::system::files::{
    batch_ensure_parent_directories, ensure_parent_directory, verify_file,
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
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(10))
                .user_agent("DreamLauncher/1.0.0")
                .build()?,
        })
    }

    /// Creates a downloader.
    pub fn new_for_java() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(300))
                .connect_timeout(Duration::from_secs(60))
                .user_agent("DreamLauncher/1.0.0")
                .build()?,
        })
    }

    /// Downloads a Java archive.
    pub async fn download_java_archive(
        &self,
        url: &str,
        destination: &Path,
        expected_sha1: Option<&str>,
    ) -> Result<()> {
        // Use a special Java downloader with longer timeouts
        let java_downloader = Self::new_for_java()?;
        java_downloader
            .download_file(url, destination, expected_sha1)
            .await
    }

    /// Downloads a file with optional SHA1 verification.
    pub async fn download_file(
        &self,
        url: &str,
        destination: &Path,
        expected_sha1: Option<&str>,
    ) -> Result<()> {
        if verify_file(destination, None, expected_sha1).await? {
            return Ok(());
        }

        ensure_parent_directory(destination).await?;
        let response = self.client.get(url).await?;

        if !response.status().is_success() {
            return Err(crate::simple_error!(
                "Failed to download {}: HTTP {}",
                url,
                response.status()
            ));
        }

        let mut file = File::create(destination).await?;
        let mut stream = response.chunk_stream();
        let mut hasher = expected_sha1.map(|_| Sha1::new());
        let mut buffer = Vec::with_capacity(8192);

        while let Some(chunk_result) = stream.next() {
            let chunk = chunk_result?;
            buffer.extend_from_slice(&chunk);

            // Write in larger batches for better I/O performance
            if buffer.len() >= 65536 {
                file.write_all(&buffer).await?;
                if let Some(ref mut h) = hasher {
                    h.update(&buffer);
                }
                buffer.clear();
            }
        }

        // Write remaining data
        if !buffer.is_empty() {
            file.write_all(&buffer).await?;
            if let Some(ref mut h) = hasher {
                h.update(&buffer);
            }
        }
        file.flush().await?;

        let file_size = tokio::fs::metadata(destination).await?.len();
        if file_size == 0 {
            tokio::fs::remove_file(destination).await?;
            return Err(crate::simple_error!("Downloaded file is empty: {url}"));
        }

        // Archive format check for Java
        if url.contains("java") || destination.to_string_lossy().contains("java") {
            self.verify_archive_format(destination).await.ok();
        }

        if let (Some(expected), Some(hasher)) = (expected_sha1, hasher) {
            let computed_hash = crate::utils::hex_encode(hasher.finalize());
            if computed_hash != expected {
                tokio::fs::remove_file(destination).await?;
                return Err(crate::simple_error!(
                    "Hash mismatch for {url}: expected {expected}, got {computed_hash}"
                ));
            }
        }
        Ok(())
    }

    /// Downloads multiple files concurrently with batch directory creation.
    pub async fn download_multiple(
        &self,
        downloads: Vec<DownloadTask>,
        max_concurrent: usize,
    ) -> Result<()> {
        if downloads.is_empty() {
            return Ok(());
        }

        // Batch create all parent directories first for optimal I/O performance
        let file_paths: Vec<_> = downloads.iter().map(|task| &task.destination).collect();
        batch_ensure_parent_directories(file_paths).await?;

        // Use higher concurrency for better performance
        let effective_concurrent = max_concurrent.max(16).min(64);
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(effective_concurrent));
        let mut handles = Vec::new();

        for task in downloads {
            let client = self.client.clone();
            let semaphore = semaphore.clone();
            let handle = tokio::spawn(async move {
                let _permit = semaphore
                    .acquire()
                    .await
                    .map_err(|_| crate::simple_error!("Semaphore error"))?;
                let downloader = Self { client };
                downloader
                    .download_file(&task.url, &task.destination, task.expected_sha1.as_deref())
                    .await
            });
            handles.push(handle);
        }
        for handle in handles {
            handle.await??;
        }
        Ok(())
    }

    /// Fetches and deserializes JSON from a URL, with retry logic.
    pub async fn get_json<T>(&self, url: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        use crate::log_debug;

        const MAX_RETRIES: usize = 2; // Reduced retries for faster failure detection
        let mut retries = 0;

        log_debug!("Fetching JSON from: {}", url);

        while retries <= MAX_RETRIES {
            match self.client.get(url).await {
                Ok(response) if response.status() == StatusCode::Ok => {
                    log_debug!("Successfully fetched JSON from: {}", url);
                    return response
                        .json::<T>()
                        .map_err(|e| crate::simple_error!("JSON parse error: {}", e));
                }
                Ok(response) if response.status() == StatusCode::TooManyRequests => {
                    if retries >= MAX_RETRIES {
                        return Err(crate::simple_error!(
                            "Rate limited after {} retries for {}: HTTP 429",
                            MAX_RETRIES,
                            url
                        ));
                    }
                    let delay = Duration::from_millis(1000 + (retries as u64 * 500));
                    log_debug!(
                        "Rate limited, retrying in {:?} (attempt {}/{})",
                        delay,
                        retries + 1,
                        MAX_RETRIES + 1
                    );
                    tokio::time::sleep(delay).await;
                    retries += 1;
                }
                Ok(response) => {
                    return Err(crate::simple_error!(
                        "HTTP {} error for {}",
                        response.status(),
                        url
                    ));
                }
                Err(e) => {
                    if retries >= MAX_RETRIES {
                        return Err(crate::simple_error!(
                            "Network error after {} retries for {}: {}",
                            MAX_RETRIES,
                            url,
                            e
                        ));
                    }
                    let delay = Duration::from_millis(500 + (retries as u64 * 200));
                    log_debug!(
                        "Network error, retrying in {:?} (attempt {}/{}): {}",
                        delay,
                        retries + 1,
                        MAX_RETRIES + 1,
                        e
                    );
                    tokio::time::sleep(delay).await;
                    retries += 1;
                }
            }
        }

        // This should never be reached due to the loop structure above
        Err(crate::simple_error!(
            "Unexpected end of retry loop for {}",
            url
        ))
    }

    async fn verify_archive_format(&self, file_path: &Path) -> Result<()> {
        use tokio::io::AsyncReadExt;
        let mut file = File::open(file_path).await?;
        let mut header = [0u8; 4];
        if file.read_exact(&mut header).await.is_err() {
            return Err(crate::simple_error!("File too small to read header"));
        }
        match &header {
            [0x50, 0x4B, 0x03, 0x04] | [0x50, 0x4B, 0x05, 0x06] | [0x50, 0x4B, 0x07, 0x08] => {
                Ok(())
            } // ZIP
            [0x1F, 0x8B, _, _] => Ok(()), // GZIP
            _ => Err(crate::simple_error!("Unknown archive format")),
        }
    }
}
