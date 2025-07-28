use dashmap::DashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs as async_fs;

pub struct FileManager {
    cache: Arc<DashMap<PathBuf, (String, Instant)>>,
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }

    fn is_cache_valid(&self, timestamp: &Instant) -> bool {
        timestamp.elapsed() < Duration::from_secs(3600) // 1 hour TTL
    }

    fn get_cached(&self, path: &Path) -> Option<String> {
        if let Some(entry) = self.cache.get(path) {
            let (content, timestamp) = entry.value();
            if self.is_cache_valid(timestamp) {
                return Some(content.clone());
            }
        }
        None
    }

    fn store_cache(&self, path: &Path, content: String) {
        self.cache
            .insert(path.to_path_buf(), (content, Instant::now()));
    }
    
    pub fn ensure_dir_exists(path: &Path) -> io::Result<()> {
        if !path.exists() {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }
    
    pub async fn ensure_dir_exists_async(path: &Path) -> io::Result<()> {
        if !async_fs::try_exists(path).await? {
            async_fs::create_dir_all(path).await?;
        }
        Ok(())
    }
    
    pub fn copy_file(from: &Path, to: &Path) -> io::Result<()> {
        if let Some(parent) = to.parent() {
            Self::ensure_dir_exists(parent)?;
        }
        fs::copy(from, to)?;
        Ok(())
    }
    
    pub fn read_file_to_string(&self, path: &Path) -> io::Result<String> {
        // Check cache first
        if let Some(cached_content) = self.get_cached(path) {
            return Ok(cached_content);
        }

        let content = fs::read_to_string(path)?;
        self.store_cache(path, content.clone());
        Ok(content)
    }
    
    pub fn write_string_to_file(&self, path: &Path, content: &str) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            Self::ensure_dir_exists(parent)?;
        }
        fs::write(path, content)?;

        // Update cache
        self.store_cache(path, content.to_string());
        Ok(())
    }
    
    pub fn file_exists(path: &Path) -> bool {
        path.exists() && path.is_file()
    }
    
    pub fn dir_exists(path: &Path) -> bool {
        path.exists() && path.is_dir()
    }
    
    pub fn get_app_data_dir() -> Option<PathBuf> {
        dirs::data_dir()
    }
}
