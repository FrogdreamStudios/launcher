//! Utility modules for the launcher backend.
//!
//! This module contains various utility functions.
//! OS detection, path management, and asset loading as well.

/// Archive extraction utilities for ZIP and TAR.GZ files.
pub mod archive_utils;
/// Asset loading and caching for embedded images.
pub mod assets;
/// Minecraft command building utilities.
pub mod command;
/// CSS loading and caching utilities.
pub mod css_loader;
/// File system operations and utilities.
pub mod file_utils;
/// Simple HTTP client utilities.
pub mod http;
/// OS detection and compatibility.
pub mod os;
/// Path utilities for Minecraft directories.
pub mod paths;
/// Application routing system.
pub mod route;
/// Lightweight stream utilities.
pub mod stream;
