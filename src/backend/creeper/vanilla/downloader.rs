use dashmap::DashMap;
use lru::LruCache;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::StreamExt;
use hyper::body::HttpBody as _;
use hyper::client::HttpConnector;
use hyper::{Body, Client, Request, Uri};
use hyper_rustls::HttpsConnectorBuilder;
use serde::de::DeserializeOwned;
use tokio::fs as tokio_fs;
use tokio::process::Command;

use crate::backend::creeper::utils::progress_bar::ProgressBar;
use crate::backend::creeper::vanilla::models::{AssetIndex, AssetIndexManifest, Library};

#[derive(Clone)]
struct CacheEntry {
    data: Vec<u8>,
    timestamp: Instant,
}

/// Handles downloading of Minecraft assets, libraries, and JSON data using HTTP.
#[derive(Clone)]
pub struct Downloader {
    client: Client<hyper_rustls::HttpsConnector<HttpConnector>, Body>,
    cache: Arc<DashMap<String, CacheEntry>>,
    lru_cache: Arc<Mutex<LruCache<String, Vec<u8>>>>,
    max_concurrent: usize,
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
        Self {
            client,
            cache: Arc::new(DashMap::new()),
            lru_cache: Arc::new(Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(64).unwrap(),
            ))),
            max_concurrent: 96,
        }
    }

    /// Check if the cached entry is still valid (1-hour TTL)
    fn is_cache_valid(&self, entry: &CacheEntry) -> bool {
        entry.timestamp.elapsed() < Duration::from_secs(3600)
    }

    /// Get from cache if valid
    fn get_cached(&self, url: &str) -> Option<Vec<u8>> {
        // Check DashMap cache first
        if let Some(entry) = self.cache.get(url) {
            if self.is_cache_valid(&entry) {
                return Some(entry.data.clone());
            }
        }

        // Check LRU cache for large files
        if let Ok(mut lru) = self.lru_cache.lock() {
            if let Some(data) = lru.get(url) {
                return Some(data.clone());
            }
        }
        None
    }

    /// Store in cache
    fn store_cache(&self, url: &str, data: Vec<u8>) {
        // For large files (>1MB), use LRU cache
        if data.len() > 1_048_576 {
            if let Ok(mut lru) = self.lru_cache.lock() {
                lru.put(url.to_string(), data);
            }
        } else {
            // For smaller files, use DashMap with TTL
            self.cache.insert(
                url.to_string(),
                CacheEntry {
                    data,
                    timestamp: Instant::now(),
                },
            );
        }
    }

    /// Creates an HTTP request with proper headers.
    fn create_request(&self, url: &str) -> Result<Request<Body>, Box<dyn std::error::Error>> {
        let uri: Uri = url.parse()?;
        let req = Request::get(uri)
            .header("User-Agent", "Mozilla/5.0 (compatible; hyper/0.14)")
            .body(Body::empty())?;
        Ok(req)
    }

    /// Executes HTTP request and returns response body as bytes with caching.
    async fn execute_request(
        &self,
        req: Request<Body>,
        url: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Check cache first
        if let Some(cached_data) = self.get_cached(url) {
            return Ok(cached_data);
        }

        let mut resp = self.client.request(req).await?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {} error", resp.status()).into());
        }

        let mut body_bytes = Vec::new();
        while let Some(chunk) = resp.body_mut().data().await {
            body_bytes.extend_from_slice(&chunk?);
        }

        // Store in cache
        self.store_cache(url, body_bytes.clone());
        Ok(body_bytes)
    }

    /// Fetches and deserializes JSON data from a specified URL.
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let req = self.create_request(url)?;
        let body_bytes = self
            .execute_request(req, url)
            .await
            .map_err(|e| format!("Failed to fetch {url}: {e}"))?;

        serde_json::from_slice::<T>(&body_bytes)
            .map_err(|e| format!("Failed to parse JSON from {url}: {e}").into())
    }

    /// Downloads a file from a URL to the specified path if it doesn't exist.
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

        let req = self.create_request(url)?;
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
                return Err(
                    format!("Size mismatch for {url}: expected {size}, got {total}").into(),
                );
            }
        }

        if let Some(pb) = progress_bar {
            pb.increment();
        }
        Ok(())
    }

    /// Generic parallel download function with progress tracking.
    async fn download_parallel<T, F, Fut>(
        &self,
        items: Vec<T>,
        task_name: &str,
        concurrency: usize,
        task_fn: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        T: Send + 'static,
        F: Fn(T, Downloader, Arc<ProgressBar>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        if items.is_empty() {
            return Ok(());
        }

        println!("Downloading {} {}", items.len(), task_name);
        let progress_bar = Arc::new(ProgressBar::new(
            items.len(),
            format!("Downloading {task_name}"),
        ));

        tokio::spawn({
            let progress_bar = progress_bar.clone();
            async move { progress_bar.as_ref().start_periodic_update().await }
        });

        let task_fn = Arc::new(task_fn);
        let results: Vec<_> = futures::stream::iter(items)
            .map(|item| {
                let downloader = Downloader {
                    client: self.client.clone(),
                    cache: self.cache.clone(),
                    lru_cache: self.lru_cache.clone(),
                    max_concurrent: self.max_concurrent,
                };
                let progress_bar = progress_bar.clone();
                let task_fn = task_fn.clone();

                async move { task_fn(item, downloader, progress_bar).await }
            })
            .buffer_unordered(concurrency.min(128))
            .collect()
            .await;

        let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();
        if !errors.is_empty() {
            for error in &errors {
                eprintln!("Error: {error}");
            }
            eprintln!("Warning: {} {} failed to download", errors.len(), task_name);
        } else {
            println!("\nAll {task_name} downloaded successfully");
        }

        Ok(())
    }

    /// Downloads Minecraft assets based on the provided asset index.
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
                .map_err(|e| format!("Failed to parse asset index: {e}"))?
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
                .into_iter()
                .filter_map(|(_, asset)| {
                    if downloaded_hashes.insert(asset.hash.clone()) {
                        Some(asset)
                    } else {
                        None
                    }
                })
                .collect()
        };

        self.download_parallel(
            unique_assets,
            "assets",
            self.max_concurrent,
            move |asset, downloader, progress_bar| {
                let assets_objects_dir = assets_objects_dir.clone();
                async move {
                    let hash = &asset.hash;
                    let subdir = &hash[0..2];
                    let file_path = assets_objects_dir.join(subdir).join(hash);
                    let url = format!("https://resources.download.minecraft.net/{subdir}/{hash}");

                    downloader
                        .download_file_if_not_exists(
                            &url,
                            &file_path,
                            Some(asset.size),
                            Some(progress_bar.as_ref()),
                        )
                        .await
                        .map_err(|e| format!("Failed to download asset {url}: {e}"))
                }
            },
        )
        .await
    }

    /// Downloads Minecraft libraries to the specified directory.
    pub async fn download_libraries(
        &self,
        libraries: &[Library],
        libraries_dir: &Path,
        version_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.download_regular_libraries(libraries, libraries_dir)
            .await?;
        self.download_natives(libraries, version_dir).await?;
        Ok(())
    }

    /// Downloads regular library JARs (not natives).
    async fn download_regular_libraries(
        &self,
        libraries: &[Library],
        libraries_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create owned data to avoid lifetime issues
        let valid_libs: Vec<_> = libraries
            .iter()
            .filter(|lib| self.should_use_library(lib))
            .filter_map(|lib| {
                lib.downloads
                    .as_ref()
                    .and_then(|d| d.artifact.as_ref().map(|a| a.clone()))
            })
            .collect();

        // Convert libraries_dir to owned PathBuf to avoid lifetime issues
        let libraries_dir = libraries_dir.to_path_buf();

        self.download_parallel(
            valid_libs,
            "libraries",
            self.max_concurrent,
            move |artifact, downloader, progress_bar| {
                let libraries_dir = libraries_dir.clone();
                async move {
                    let path = libraries_dir.join(&artifact.path);
                    downloader
                        .download_file_if_not_exists(
                            &artifact.url,
                            &path,
                            None,
                            Some(progress_bar.as_ref()),
                        )
                        .await
                        .map_err(|e| format!("Failed to download library {}: {}", artifact.url, e))
                }
            },
        )
        .await
    }

    /// Downloads and extracts native libraries.
    async fn download_natives(
        &self,
        libraries: &[Library],
        version_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let current_os = self.get_current_os();
        let natives_dir = version_dir.join("natives");

        tokio_fs::create_dir_all(&natives_dir).await?;

        let natives_to_download: Vec<(
            Library,
            crate::backend::creeper::vanilla::models::Artifact,
        )> = libraries
            .iter()
            .filter(|lib| self.should_use_library(lib))
            .filter_map(|lib| {
                if let (Some(natives), Some(downloads)) = (&lib.natives, &lib.downloads) {
                    if let Some(classifier) = natives.get(&current_os) {
                        if let Some(classifiers) = &downloads.classifiers {
                            if let Some(artifact) = classifiers.get(classifier) {
                                // FUCKING BORROWING, I SPENT HOURS ON THIS. JUST FOR THIS SHIT
                                let owned_lib = lib.clone();
                                let owned_artifact = artifact.clone();
                                return Some((owned_lib, owned_artifact));
                            }
                        }
                    }
                }
                None
            })
            .collect();

        if natives_to_download.is_empty() {
            println!("No natives to download for current OS: {current_os}");
            return Ok(());
        }

        let natives_dir = natives_dir.to_path_buf();

        self.download_parallel(
            natives_to_download,
            "natives",
            32,
            move |data, downloader, progress_bar| {
                let natives_dir = natives_dir.clone();
                async move {
                    let (lib, artifact) = data;
                    let temp_path =
                        natives_dir.join(format!("temp_{}.jar", artifact.path.replace("/", "_")));

                    // Download native JAR
                    downloader
                        .download_file_if_not_exists(
                            &artifact.url,
                            &temp_path,
                            None,
                            Some(progress_bar.as_ref()),
                        )
                        .await
                        .map_err(|e| {
                            format!("Failed to download native {}: {}", artifact.url, e)
                        })?;

                    // Extract native JAR
                    downloader
                        .extract_native_jar(&temp_path, &natives_dir, &lib)
                        .await
                        .map_err(|e| {
                            format!("Failed to extract native {}: {}", temp_path.display(), e)
                        })?;

                    // Clean up
                    if temp_path.exists() {
                        tokio_fs::remove_file(&temp_path).await.map_err(|e| {
                            format!("Failed to remove temp file {}: {}", temp_path.display(), e)
                        })?;
                    }

                    Ok(())
                }
            },
        )
        .await?;

        println!("\nAll natives downloaded and extracted successfully");
        Ok(())
    }

    /// Extracts a native JAR file to the natives' directory.
    async fn extract_native_jar(
        &self,
        jar_path: &Path,
        natives_dir: &Path,
        library: &Library,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let jar_path_str = jar_path.to_string_lossy();
        let natives_dir_str = natives_dir.to_string_lossy();

        let mut cmd = if cfg!(target_os = "windows") {
            self.create_windows_extract_command(&jar_path_str, &natives_dir_str)
        } else {
            self.create_unix_extract_command(&jar_path_str, &natives_dir_str, library)
        };

        let output = cmd.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to extract native JAR {jar_path_str}: {stderr}").into());
        }

        Ok(())
    }

    /// Creates Windows PowerShell extraction command.
    fn create_windows_extract_command(&self, jar_path: &str, natives_dir: &str) -> Command {
        let mut cmd = Command::new("powershell");
        cmd.arg("-Command")
           .arg(format!(
               "Add-Type -AssemblyName System.IO.Compression.FileSystem; \
                $zip = [System.IO.Compression.ZipFile]::OpenRead('{jar_path}'); \
                foreach ($entry in $zip.Entries) {{ \
                    if (-not $entry.Name.EndsWith('/') -and -not $entry.FullName.StartsWith('META-INF/')) {{ \
                        $destinationPath = Join-Path '{natives_dir}' $entry.FullName; \
                        $destinationDir = Split-Path $destinationPath -Parent; \
                        if (-not (Test-Path $destinationDir)) {{ \
                            New-Item -ItemType Directory -Path $destinationDir -Force | Out-Null; \
                        }}; \
                        [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $destinationPath, $true); \
                    }} \
                }}; \
                $zip.Dispose();"
           ));
        cmd
    }

    /// Creates Unix unzip extraction command.
    fn create_unix_extract_command(
        &self,
        jar_path: &str,
        natives_dir: &str,
        library: &Library,
    ) -> Command {
        let mut cmd = Command::new("unzip");
        cmd.arg("-o") // Overwrite files without prompting
            .arg("-j") // Flatten directory structure
            .arg(jar_path)
            .arg("-d")
            .arg(natives_dir);

        let default_exclude = vec!["META-INF/*".to_string()];
        let exclude_patterns = if let Some(extract) = &library.extract {
            extract
                .exclude
                .as_ref()
                .map(|e| e.as_slice())
                .unwrap_or(&default_exclude)
        } else {
            &default_exclude
        };

        for pattern in exclude_patterns {
            cmd.arg("-x").arg(pattern);
        }

        cmd
    }

    /// Determines if a library should be used based on OS rules.
    fn should_use_library(&self, library: &Library) -> bool {
        if let Some(rules) = &library.rules {
            let current_os = self.get_current_os();
            let mut should_use = false;

            for rule in rules {
                let matches_rule = rule
                    .os
                    .as_ref()
                    .map(|os_rule| {
                        os_rule
                            .name
                            .as_ref()
                            .map_or(true, |name| name == &current_os)
                    })
                    .unwrap_or(true);

                if matches_rule {
                    should_use = rule.action == "allow";
                }
            }

            should_use
        } else {
            true // No rules mean the library applies to all platforms
        }
    }

    /// Gets the current operating system name in Minecraft format.
    fn get_current_os(&self) -> String {
        match std::env::consts::OS {
            "windows" => "windows",
            "linux" => "linux",
            "macos" => "osx",
            _ => "unknown",
        }
        .to_string()
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}
