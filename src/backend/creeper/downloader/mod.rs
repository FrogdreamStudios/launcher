//! HTTP downloading utilities for Minecraft assets.
//!
//! HTTP downloading functionality with progress
//! tracking for downloading game files, libraries, and assets.

/// HTTP downloader implementation.
pub mod http;
/// Data models for download tasks.
pub mod models;
/// Progress tracking for downloads.
pub mod progress;

pub use http::HttpDownloader;
pub use progress::ProgressTracker;
