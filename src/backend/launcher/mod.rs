//! Creeper. Core Minecraft launcher functionality.

/// Common utilities and platform information.
pub mod common;
/// HTTP downloading and progress tracking.
pub mod downloader;
/// Java runtime management and detection.
pub mod java;
/// Core launcher functionality.
pub mod launcher;
/// Data models and structures.
pub mod models;
/// Version management functionality.
pub mod versions;

// Re-export the main launcher types
pub use common::{DownloadHelper, FileValidator, PlatformInfo, SystemInfo};
pub use launcher::MinecraftLauncher;
pub use versions::VersionManager;
