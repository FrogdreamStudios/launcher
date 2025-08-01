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
        if let Some(sha1) = expected_sha1 {
            if self.verify_file_hash(destination, sha1).await? {
                debug!("File already exists with correct hash: {:?}", destination);
                if let Some(tracker) = tracker {
                    tracker.complete();
                }
                return Ok(());
            }
        }

        // Create parent directories if they don't exist
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        debug!("Downloading {} to {:?}", url, destination);

        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
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

        // Verify hash if provided
        if let (Some(expected), Some(hasher)) = (expected_sha1, hasher) {
            let computed_hash = hex::encode(hasher.finalize());
            if computed_hash != expected {
                tokio::fs::remove_file(destination).await?;
                return Err(anyhow::anyhow!(
                    "Hash mismatch for {}: expected {}, got {}",
                    url,
                    expected,
                    computed_hash
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
        debug!("Fetching JSON from {}", url);

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

    #[allow(dead_code)]
    pub async fn head_request(&self, url: &str) -> Result<reqwest::Response> {
        let response = self.client.head(url).send().await?;
        Ok(response)
    }
}

impl Default for HttpDownloader {
    fn default() -> Self {
        Self::new().expect("Failed to create HTTP client")
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
