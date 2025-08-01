#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub destination: std::path::PathBuf,
    pub expected_sha1: Option<String>,
}
