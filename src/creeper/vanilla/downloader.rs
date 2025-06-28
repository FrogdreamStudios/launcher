use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use futures::StreamExt;
use hyper::{Body, Client, Request, Uri};
use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnectorBuilder;
use hyper::body::HttpBody as _;
use serde::de::DeserializeOwned;
use tokio::fs as tokio_fs;

use crate::creeper::vanilla::models::{AssetIndex, AssetIndexManifest, Library};
use crate::creeper::utils::progress_bar::ProgressBar;

/// Handles downloading of Minecraft assets, libraries, and JSON data using HTTP.
pub struct Downloader {
    client: Client<hyper_rustls::HttpsConnector<HttpConnector>, Body>,
}

impl Downloader {
    /// Creates a new `Downloader` with an HTTP/2 client configured for HTTPS requests.
    pub fn new() -> Self {
        let https = HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_or_http()
            .enable_http2()
            .build();
        let client = Client::builder()
            .http2_only(true)
            .pool_max_idle_per_host(48)
            .build(https);
        Self { client }
    }

    /// Fetches and deserializes JSON data from a specified URL.
    ///
    /// # Arguments
    /// * `url` - the URL to fetch JSON data from.
    ///
    /// # Returns
    /// Deserialized JSON data or an error if the request or parsing fails.
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let uri: Uri = url.parse()?;
        let req = Request::get(uri.clone())
            .header("User-Agent", "Mozilla/5.0 (compatible; hyper/0.14)")
            .body(Body::empty())?;
        let mut resp = self.client.request(req).await?;
        if !resp.status().is_success() {
            return Err(format!("Failed to fetch {}: HTTP {}", url, resp.status()).into());
        }
        let mut body_bytes = Vec::new();
        while let Some(chunk) = resp.body_mut().data().await {
            let chunk = chunk?;
            body_bytes.extend_from_slice(&chunk);
        }
        let parsed = serde_json::from_slice::<T>(&body_bytes)?;
        Ok(parsed)
    }

    /// Downloads a file from a URL to the specified path if it doesn't exist.
    ///
    /// # Arguments
    /// * `url` - the URL to download the file from.
    /// * `path` - the filesystem path to save the file.
    /// * `expected_size` - optional expected file size for verification.
    /// * `progress_bar` - optional progress bar to update during download.
    ///
    /// # Returns
    /// Result indicating success or failure.
    pub async fn download_file_if_not_exists(
        &self,
        url: &str,
        path: &Path,
        expected_size: Option<u64>,
        progress_bar: Option<&ProgressBar>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if tokio_fs::metadata(path).await.is_ok() {
            if let Some(pb) = progress_bar {
                pb.increment();
            }
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            tokio_fs::create_dir_all(parent).await?;
        }
        let uri: Uri = url.parse()?;
        let req = Request::get(uri.clone())
            .header("User-Agent", "Mozilla/5.0 (compatible; hyper/0.14)")
            .body(Body::empty())?;
        let mut resp = self.client.request(req).await?;
        if !resp.status().is_success() {
            return Err(format!("Failed to download {}: HTTP {}", url, resp.status()).into());
        }
        let mut file = tokio_fs::File::create(path).await?;
        let mut total = 0u64;
        while let Some(chunk) = resp.body_mut().data().await {
            let chunk = chunk?;
            total += chunk.len() as u64;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
        }
        if let Some(size) = expected_size {
            if total != size {
                return Err(format!(
                    "Size mismatch for {}: expected {}, got {}",
                    url, size, total
                )
                    .into());
            }
        }
        if let Some(pb) = progress_bar {
            pb.increment();
        }
        Ok(())
    }

    /// Downloads Minecraft assets based on the provided asset index.
    ///
    /// # Arguments
    /// * `asset_index` - metadata containing URLs and IDs for assets.
    /// * `minecraft_dir` - root directory for Minecraft files.
    ///
    /// # Returns
    /// Result indicating success or failure.
    pub async fn download_assets(
        &self,
        asset_index: &AssetIndex,
        minecraft_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Downloading asset index from {}", asset_index.url);
        let indexes_dir = minecraft_dir.join("assets/indexes");
        tokio_fs::create_dir_all(&indexes_dir).await?;
        let index_path = indexes_dir.join(format!("{}.json", asset_index.id));

        let asset_index_manifest: AssetIndexManifest = if index_path.exists() {
            println!("Using cached asset index: {}", index_path.display());
            serde_json::from_str(&tokio_fs::read_to_string(&index_path).await?)
                .map_err(|e| format!("Failed to parse asset index: {}", e))?
        } else {
            let manifest: AssetIndexManifest = self.get_json(&asset_index.url).await?;
            tokio_fs::write(&index_path, serde_json::to_string(&manifest)?).await?;
            println!("Asset index saved to {}", index_path.display());
            manifest
        };

        let assets_objects_dir = minecraft_dir.join("assets/objects");
        tokio_fs::create_dir_all(&assets_objects_dir).await?;

        let unique_assets: Vec<_> = {
            let mut downloaded_hashes = HashSet::new();
            asset_index_manifest
                .objects
                .iter()
                .filter(|(_, asset)| downloaded_hashes.insert(asset.hash.clone()))
                .collect()
        };

        let total_assets = unique_assets.len();
        println!("Downloading {} assets", total_assets);
        let progress_bar = Arc::new(ProgressBar::new(
            total_assets,
            "Downloading assets".to_string(),
        ));
        tokio::spawn({
            let progress_bar = progress_bar.clone();
            async move { progress_bar.as_ref().start_periodic_update().await }
        });

        let results: Vec<_> = futures::stream::iter(unique_assets)
            .map(|(_path, asset)| {
                let downloader = Downloader {
                    client: self.client.clone(),
                };
                let assets_objects_dir = assets_objects_dir.clone();
                let hash = asset.hash.clone();
                let size = asset.size;
                let progress_bar = progress_bar.clone();

                async move {
                    let subdir = &hash[0..2];
                    let file_path = assets_objects_dir.join(subdir).join(&hash);
                    let url =
                        format!("https://resources.download.minecraft.net/{}/{}", subdir, hash);
                    downloader
                        .download_file_if_not_exists(
                            &url,
                            &file_path,
                            Some(size),
                            Some(progress_bar.as_ref()),
                        )
                        .await
                        .map_err(|e| format!("Failed to download asset {}: {}", url, e))
                }
            })
            .buffer_unordered(96)
            .collect()
            .await;

        let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();
        if !errors.is_empty() {
            for error in &errors {
                eprintln!("Error: {}", error);
            }
            eprintln!("Warning: {} assets failed to download", errors.len());
        } else {
            println!("\nAll assets downloaded successfully");
        }
        Ok(())
    }

    /// Downloads Minecraft libraries to the specified directory.
    ///
    /// # Arguments
    /// * `libraries` - list of libraries to download.
    /// * `libraries_dir` - directory to store the libraries.
    ///
    /// # Returns
    /// Result indicating success or failure.
    pub async fn download_libraries(
        &self,
        libraries: &[Library],
        libraries_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let valid_libs: Vec<_> = libraries
            .iter()
            .filter_map(|lib| lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()))
            .collect();

        if valid_libs.is_empty() {
            return Ok(());
        }

        println!("Downloading {} libraries", valid_libs.len());
        let progress_bar = Arc::new(ProgressBar::new(
            valid_libs.len(),
            "Downloading libraries".to_string(),
        ));
        tokio::spawn({
            let progress_bar = progress_bar.clone();
            async move { progress_bar.as_ref().start_periodic_update().await }
        });

        let results: Vec<_> = futures::stream::iter(valid_libs)
            .map(|artifact| {
                let downloader = Downloader {
                    client: self.client.clone(),
                };
                let path = libraries_dir.join(&artifact.path);
                let url = artifact.url.clone();
                let progress_bar = progress_bar.clone();
                async move {
                    downloader
                        .download_file_if_not_exists(
                            &url,
                            &path,
                            None,
                            Some(progress_bar.as_ref()),
                        )
                        .await
                        .map_err(|e| format!("Failed to download library {}: {}", url, e))
                }
            })
            .buffer_unordered(96)
            .collect()
            .await;

        let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();
        if !errors.is_empty() {
            for error in &errors {
                eprintln!("Error: {}", error);
            }
            eprintln!("Warning: {} libraries failed to download", errors.len());
        } else {
            println!("\nAll libraries downloaded successfully");
        }
        Ok(())
    }
}