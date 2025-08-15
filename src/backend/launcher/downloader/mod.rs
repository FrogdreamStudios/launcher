//! HTTP downloading utilities for Minecraft assets.
//!
//! HTTP downloading functionality with progress
//! tracking for downloading game files, libraries, and assets.

/// HTTP downloader implementation.
pub mod main;
/// Data models for download tasks.
pub mod models;

pub use main::HttpDownloader;
