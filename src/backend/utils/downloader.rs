use reqwest;
use std::io;
use std::path::Path;
use tokio::fs;

pub struct Downloader {
    client: reqwest::Client,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn download_file(&self, url: &str, destination: &Path) -> Result<(), DownloadError> {
        let response = self.client.get(url).send().await?;

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).await?;
        }

        let bytes = response.bytes().await?;
        fs::write(destination, bytes).await?;

        Ok(())
    }

    pub async fn download_file_with_progress<F>(
        &self,
        url: &str,
        destination: &Path,
        progress_callback: F,
    ) -> Result<(), DownloadError>
    where
        F: Fn(u64, u64),
    {
        let response = self.client.get(url).send().await?;

        let total_size = response.content_length().unwrap_or(0);

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).await?;
        }

        let bytes = response.bytes().await?;
        fs::write(destination, &bytes).await?;
        let downloaded = bytes.len() as u64;
        progress_callback(downloaded, total_size);

        Ok(())
    }

    pub async fn get_file_size(&self, url: &str) -> Result<Option<u64>, DownloadError> {
        let response = self.client.head(url).send().await?;

        if !response.status().is_success() {
        }

        Ok(response.content_length())
    }
}

#[derive(Debug)]
pub enum DownloadError {
    IoError(io::Error),
    ReqwestError(reqwest::Error),
}

impl From<reqwest::Error> for DownloadError {
    fn from(error: reqwest::Error) -> Self {
        DownloadError::ReqwestError(error)
    }
}

impl From<io::Error> for DownloadError {
    fn from(error: io::Error) -> Self {
        DownloadError::IoError(error)
    }
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::IoError(err) => write!(f, "IO error: {}", err),
            DownloadError::ReqwestError(err) => write!(f, "Request error: {}", err),
        }
    }
}

impl std::error::Error for DownloadError {}
