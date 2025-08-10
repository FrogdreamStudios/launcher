//! Main CLI entry point and runner.
//!
//! This is the entry point for running the launcher in CLI mode.

use crate::backend::creeper::cli::selector::interactive_mode;
use crate::backend::creeper::launcher::MinecraftLauncher;
use anyhow::Result;

/// Runs the launcher in interactive CLI mode.
pub async fn run_interactive() -> Result<()> {
    println!("Dream Launcher [CLI mode]");
    let mut launcher = MinecraftLauncher::new(None).await?;
    interactive_mode(&mut launcher).await
}
