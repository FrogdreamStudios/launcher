//! Download task models for HTTP downloading.
//!
//! Data structures are used to represent
//! download tasks with URLs, destinations, and verification hashes.

/// Represents a single file download task.
///
/// Contains all information needed to download a file, including
/// the source URL, destination path, and optional SHA1 hash for verification.
#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub destination: std::path::PathBuf,
    pub expected_sha1: Option<String>,
}

impl DownloadTask {
    /// Creates a new download task without SHA1 verification.
    pub const fn new(url: String, destination: std::path::PathBuf) -> Self {
        Self {
            url,
            destination,
            expected_sha1: None,
        }
    }

    /// Adds SHA1 hash verification to the download task.
    pub fn with_sha1(mut self, sha1: String) -> Self {
        self.expected_sha1 = Some(sha1);
        self
    }
}
