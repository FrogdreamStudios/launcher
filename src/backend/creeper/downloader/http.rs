use super::progress::ProgressTracker;
use crate::backend::creeper::downloader::models::DownloadTask;
use anyhow::Result;
use futures_util::StreamExt;
use reqwest::Client;
use sha1::{Digest, Sha1};
use std::path::Path;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, warn};

pub struct HttpDownloader {
    client: Client,
}

impl HttpDownloader {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(60))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()?;

        Ok(Self { client })
    }

    pub async fn download_file(
        &self,
        url: &str,
        destination: &Path,
        expected_sha1: Option<&str>,
        mut tracker: Option<&mut ProgressTracker>,
    ) -> Result<()> {
        // Check if file already exists and has correct hash
        if let Some(sha1) = expected_sha1
            && self.verify_file_hash(destination, sha1).await?
        {
            debug!("File already exists with correct hash: {:?}", destination);
            if let Some(tracker) = tracker {
                tracker.complete();
            }
            return Ok(());
        }

        // Create parent directories if they don't exist
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        debug!("Downloading {url} to {destination:?}");

        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download {}: HTTP {}",
                url,
                response.status()
            ));
        }

        // Log response headers for debugging
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");
        debug!("Content-Type: {content_type}");

        // Warn if content type doesn't match expected archive format
        if url.contains("java")
            && !content_type.contains("application/")
            && !content_type.contains("octet-stream")
        {
            warn!("Unexpected content type for Java archive: {content_type}");
        }

        let total_size = response.content_length();
        if let (Some(tracker), Some(size)) = (tracker.as_mut(), total_size) {
            tracker.set_total(size);
        }

        let mut file = File::create(destination).await?;
        let mut stream = response.bytes_stream();
        let mut downloaded = 0u64;
        let mut hasher = expected_sha1.map(|_| Sha1::new());

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;

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
        debug!("Downloaded file size: {file_size} bytes");

        if file_size == 0 {
            tokio::fs::remove_file(destination).await?;
            return Err(anyhow::anyhow!("Downloaded file is empty: {url}"));
        }

        // For Java archives, do a basic format check
        if (url.contains("java") || destination.to_string_lossy().contains("java"))
            && let Err(e) = self.verify_archive_format(destination).await
        {
            warn!("Archive format verification failed: {e}");
        }

        // Verify hash if provided
        if let (Some(expected), Some(hasher)) = (expected_sha1, hasher) {
            let computed_hash = hex::encode(hasher.finalize());
            if computed_hash != expected {
                tokio::fs::remove_file(destination).await?;
                return Err(anyhow::anyhow!(
                    "Hash mismatch for {url}: expected {expected}, got {computed_hash}"
                ));
            }
        }

        if let Some(tracker) = tracker {
            tracker.complete();
        }

        Ok(())
    }

    pub async fn download_multiple(
        &self,
        downloads: Vec<DownloadTask>,
        max_concurrent: usize,
    ) -> Result<()> {
        use futures_util::stream::{self, StreamExt};

        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent));

        let tasks = downloads.into_iter().map(|task| {
            let client = self.client.clone();
            let semaphore = semaphore.clone();

            async move {
                let _permit = semaphore.acquire().await?;

                let downloader = HttpDownloader { client };
                downloader
                    .download_file(
                        &task.url,
                        &task.destination,
                        task.expected_sha1.as_deref(),
                        None,
                    )
                    .await
            }
        });

        let mut stream = stream::iter(tasks).buffer_unordered(max_concurrent);

        while let Some(result) = stream.next().await {
            result?; // Propagate errors instead of just warning
        }

        Ok(())
    }

    async fn verify_file_hash(&self, file_path: &Path, expected_sha1: &str) -> Result<bool> {
        if !file_path.exists() {
            return Ok(false);
        }

        let content = tokio::fs::read(file_path).await?;
        let mut hasher = Sha1::new();
        hasher.update(&content);
        let computed_hash = hex::encode(hasher.finalize());

        Ok(computed_hash == expected_sha1)
    }

    pub async fn get_json<T>(&self, url: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        debug!("Fetching JSON from {url}");

        // Try with retries for rate limiting
        let mut retries = 0;
        const MAX_RETRIES: u32 = 3;

        loop {
            let response = self.client.get(url).send().await?;

            match response.status() {
                reqwest::StatusCode::OK => {
                    let json = response.json::<T>().await?;
                    return Ok(json);
                }
                reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    retries += 1;
                    if retries > MAX_RETRIES {
                        return Err(anyhow::anyhow!(
                            "Failed to fetch {url} after {MAX_RETRIES} retries: HTTP 429 Too Many Requests"
                        ));
                    }

                    let wait_time = Duration::from_secs(1 + (retries as u64));
                    warn!(
                        "Rate limited, waiting {wait_time:?} before retry {retries}/{MAX_RETRIES}"
                    );
                    tokio::time::sleep(wait_time).await;
                    continue;
                }
                status => {
                    return Err(anyhow::anyhow!("Failed to fetch {url}: HTTP {status}"));
                }
            }
        }
    }

    async fn verify_archive_format(&self, file_path: &Path) -> Result<()> {
        use tokio::io::AsyncReadExt;

        let mut file = File::open(file_path).await?;
        let mut header = [0u8; 4];

        if file.read_exact(&mut header).await.is_err() {
            return Err(anyhow::anyhow!("File too small to read header"));
        }

        // Check for known archive formats
        if header[0] == 0x50 && header[1] == 0x4B {
            // ZIP format (PK header)
            debug!("Detected ZIP format");
            Ok(())
        } else if header[0] == 0x1F && header[1] == 0x8B {
            // GZIP format
            debug!("Detected GZIP format");
            Ok(())
        } else {
            // Could be a different format or corrupted
            debug!(
                "Unknown format - header: {:02X} {:02X} {:02X} {:02X}",
                header[0], header[1], header[2], header[3]
            );
            Err(anyhow::anyhow!(
                "Unrecognized archive format. Header: {:02X} {:02X} {:02X} {:02X}",
                header[0],
                header[1],
                header[2],
                header[3]
            ))
        }
    }
}

impl Default for HttpDownloader {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            client: reqwest::Client::new(),
        })
    }
}

impl DownloadTask {
    pub fn new(url: String, destination: std::path::PathBuf) -> Self {
        Self {
            url,
            destination,
            expected_sha1: None,
        }
    }

    pub fn with_sha1(mut self, sha1: String) -> Self {
        self.expected_sha1 = Some(sha1);
        self
    }
}
