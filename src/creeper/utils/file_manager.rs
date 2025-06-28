use std::path::{Path};
use glob::glob;

pub struct FileSystem;

impl FileSystem {
    pub fn build_classpath(
        &self,
        libraries_dir: &Path,
        client_jar_path: &Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut classpath = String::new();
        let pattern = format!("{}/**/*.jar", libraries_dir.display());
        for entry in glob(&pattern)? {
            classpath.push_str(&format!("{}:", entry?.display()));
        }
        classpath.push_str(client_jar_path.to_str().ok_or("Invalid client jar path")?);
        Ok(classpath)
    }
    
    pub fn new() -> Self {
        Self
    }

    pub fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}