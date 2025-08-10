//! File and directory removal utilities for launcher cleanup.

use crate::backend::creeper::launcher::MinecraftLauncher;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// Removes all launcher-created instances and files with user confirmation.
///
/// This function displays all directories that will be deleted, calculates
/// the total size, and asks for user confirmation before proceeding with deletion.
pub(crate) async fn remove_instances(launcher: &MinecraftLauncher) -> anyhow::Result<()> {
    use crate::backend::utils::formater::format_size;
    use crate::backend::utils::paths::*;
    use crate::backend::utils::sizer::calculate_directory_size;

    println!("Delete Instances");
    println!("This will delete all files created by this launcher!");

    let game_dir = launcher.get_game_dir();

    // List of directories to delete with their descriptions
    let mut paths_to_delete: Vec<(&str, PathBuf)> = vec![
        ("Minecraft versions", get_versions_dir(game_dir)),
        ("Game libraries", get_libraries_dir(game_dir)),
        ("Game assets", get_assets_dir(game_dir)),
        ("Game logs", get_logs_dir(game_dir)),
        ("Natives", game_dir.join("natives")),
    ];

    // Check for additional launcher-specific directories
    if let Ok(java_dir) = get_java_dir()
        && java_dir.exists()
    {
        paths_to_delete.push(("Java runtimes", java_dir));
    }
    if let Ok(cache_dir) = get_cache_dir()
        && cache_dir.exists()
    {
        paths_to_delete.push(("Launcher cache", cache_dir));
    }

    // Filter to only include paths that actually exist
    paths_to_delete.retain(|(_, path)| path.exists());

    if paths_to_delete.is_empty() {
        println!("No launcher files found to delete");
        return Ok(());
    }

    // Calculate the total size of all directories to be deleted
    let total_size: u64 = paths_to_delete
        .iter()
        .map(|(_, path)| calculate_directory_size(path).unwrap_or(0))
        .sum();

    println!("The following paths will be deleted:");
    for (name, path) in &paths_to_delete {
        println!("  • {}: {}", name, path.display());
    }
    println!("\nTotal size: {}", format_size(total_size as f64));

    print!("Are you sure you want to delete all these files? (y/N): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let confirm = input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes";
    if !confirm {
        println!("Deletion cancelled");
        return Ok(());
    }

    println!("Deleting files...");
    let (mut deleted, mut failed) = (0, 0); // Track success and failure counts

    // Delete each directory and track results
    for (name, path) in paths_to_delete {
        print!("Deleting {name}... ");
        match fs::remove_dir_all(&path) {
            Ok(_) => {
                println!("OK");
                deleted += 1;
            }
            Err(e) => {
                println!("Failed ({e})");
                failed += 1;
            }
        }
    }

    println!();

    // Display final results
    if failed == 0 {
        println!(
            "Successfully deleted {} directories ({})!",
            deleted,
            format_size(total_size as f64)
        );
    } else {
        println!("Deleted {deleted} directories, {failed} failed");
    }
    Ok(())
}
