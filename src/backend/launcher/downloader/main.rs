//! HTTP downloader implementation with progress tracking.

use crate::utils::{Digest, Result, Sha1};
use std::{path::Path, time::Duration};
use tokio::{fs::File, io::AsyncWriteExt};

use crate::backend::launcher::downloader::models::DownloadTask;
use crate::backend::utils::net::http::{Client, StatusCode};
use crate::backend::utils::net::stream::ResponseChunkExt;
use crate::backend::utils::system::files::{ensure_parent_directory, verify_file};

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
                .timeout(Duration::from_secs(60))
                .connect_timeout(Duration::from_secs(30))
                .user_agent("DreamLauncher/1.0.0")
                .build()?,
        })
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

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
            if let Some(ref mut h) = hasher {
                h.update(&chunk);
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

    /// Downloads multiple files concurrently.
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
        const MAX_RETRIES: usize = 3;
        let mut retries = 0;
        loop {
            match self.client.get(url).await {
                Ok(response) if response.status() == StatusCode::Ok => {
                    return response
                        .json::<T>()
                        .map_err(|_e| crate::simple_error!("JSON error: {e}"));
                }
                Ok(response) if response.status() == StatusCode::TooManyRequests => {
                    retries += 1;
                    if retries > MAX_RETRIES {
                        return Err(crate::simple_error!(
                            "Failed to fetch {url} after {MAX_RETRIES} retries: HTTP 429"
                        ));
                    }
                    tokio::time::sleep(Duration::from_secs(2_u64.pow(retries as u32))).await;
                }
                Ok(response) => {
                    return Err(crate::simple_error!(
                        "HTTP {} for {}",
                        response.status(),
                        url
                    ));
                }
                Err(_) => {
                    retries += 1;
                    if retries > MAX_RETRIES {
                        return Err(crate::simple_error!(
                            "Network error after {MAX_RETRIES} retries for {url}"
                        ));
                    }
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
