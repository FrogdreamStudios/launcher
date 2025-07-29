use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::fs;

/// Cache entry with timestamp.
#[derive(Clone)]
pub struct CacheEntry {
    pub data: Vec<u8>,
    pub timestamp: Instant,
}

/// Cache configuration.
#[derive(Clone)]
pub struct CacheConfig {
    pub ttl_seconds: u64,
    pub max_memory_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl_seconds: 3600,
            max_memory_size: 64 * 1024 * 1024,
        }
    }
}

/// Generic cache manager.
#[derive(Clone)]
pub struct CacheManager {
    memory_cache: Arc<DashMap<String, CacheEntry>>,
    lru_cache: Arc<Mutex<lru::LruCache<String, Vec<u8>>>>,
    config: CacheConfig,
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            memory_cache: Arc::new(DashMap::new()),
            lru_cache: Arc::new(Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(64).unwrap(),
            ))),
            config,
        }
    }

    pub fn new_default() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Check if the cache entry is still valid.
    pub fn is_cache_valid(&self, entry: &CacheEntry) -> bool {
        entry.timestamp.elapsed() < Duration::from_secs(self.config.ttl_seconds)
    }

    /// Get data from the memory cache.
    pub fn get_memory_cache(&self, key: &str) -> Option<Vec<u8>> {
        if let Some(entry) = self.memory_cache.get(key) {
            if self.is_cache_valid(&entry) {
                return Some(entry.data.clone());
            }
        }
        None
    }

    /// Get data from LRU cache.
    pub fn get_lru_cache(&self, key: &str) -> Option<Vec<u8>> {
        if let Ok(mut lru) = self.lru_cache.lock() {
            if let Some(data) = lru.get(key) {
                return Some(data.clone());
            }
        }
        None
    }

    /// Get data from any cache.
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.get_memory_cache(key)
            .or_else(|| self.get_lru_cache(key))
    }

    /// Store data in appropriate cache.
    pub fn store(&self, key: &str, data: Vec<u8>) {
        if data.len() > self.config.max_memory_size {
            // Store large data in LRU cache
            if let Ok(mut lru) = self.lru_cache.lock() {
                lru.put(key.to_string(), data);
            }
        } else {
            // Store small data in the memory cache
            self.memory_cache.insert(
                key.to_string(),
                CacheEntry {
                    data,
                    timestamp: Instant::now(),
                },
            );
        }
    }
}

/// File cache manager for JSON and other files.
pub struct FileCacheManager {
    cache_dir: PathBuf,
    config: CacheConfig,
}

impl FileCacheManager {
    pub fn new(cache_dir: PathBuf, config: CacheConfig) -> Self {
        Self { cache_dir, config }
    }

    pub fn new_default(cache_dir: PathBuf) -> Self {
        Self::new(cache_dir, CacheConfig::default())
    }

    /// Get cached file path.
    pub fn get_cache_path(&self, key: &str) -> PathBuf {
        self.cache_dir.join(format!("{key}.json"))
    }

    /// Check if cached file exists and is valid.
    pub async fn is_cached(&self, key: &str) -> bool {
        let cache_path = self.get_cache_path(key);

        if let Ok(metadata) = fs::metadata(&cache_path).await {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    return elapsed < Duration::from_secs(self.config.ttl_seconds);
                }
            }
        }
        false
    }

    /// Get cached JSON data.
    pub async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Box<dyn std::error::Error>> {
        if !self.is_cached(key).await {
            return Ok(None);
        }

        let cache_path = self.get_cache_path(key);
        let content = fs::read_to_string(&cache_path).await?;
        let data: T = serde_json::from_str(&content)?;
        Ok(Some(data))
    }

    /// Store JSON data in cache.
    pub async fn store_json<T: Serialize>(
        &self,
        key: &str,
        data: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(&self.cache_dir).await?;
        let cache_path = self.get_cache_path(key);
        let content = serde_json::to_string(data)?;
        fs::write(&cache_path, content).await?;
        Ok(())
    }
}
