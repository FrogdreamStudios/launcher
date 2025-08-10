//! Java runtime management utilities.
//!
//! Java installation detection, downloading,
//! and management for running Minecraft with the correct Java version.

/// Java installation manager and detector.
pub mod manager;
/// Java runtime information and utilities.
pub mod runtime;

pub use manager::JavaManager;
