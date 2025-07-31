use dashmap::DashMap;
use glob::glob;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct FileSystem {
    cache: Arc<DashMap<PathBuf, (String, Instant)>>,
}

impl FileSystem {
    pub fn build_classpath(
        &self,
        libraries_dir: &Path,
        client_jar_path: &Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let cache_key = libraries_dir.to_path_buf();

        // Check cache first
        if let Some(entry) = self.cache.get(&cache_key) {
            let (cached_classpath, timestamp) = entry.value();
            if timestamp.elapsed() < Duration::from_secs(300) {
                let mut result = cached_classpath.clone();
                result.push_str(client_jar_path.to_str().ok_or("Invalid client jar path")?);
                return Ok(result);
            }
        }

        let mut classpath = String::new();
        let pattern = format!("{}/**/*.jar", libraries_dir.display());
        for entry in glob(&pattern)? {
            classpath.push_str(&format!("{}:", entry?.display()));
        }

        // Cache without a client jar
        self.cache
            .insert(cache_key, (classpath.clone(), Instant::now()));

        classpath.push_str(client_jar_path.to_str().ok_or("Invalid client jar path")?);
        Ok(classpath)
    }

    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }

    pub fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}
