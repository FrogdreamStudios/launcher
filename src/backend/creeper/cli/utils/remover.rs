use crate::backend::creeper::launcher::MinecraftLauncher;
use console::style;
use dialoguer::Confirm;
use std::fs;
use std::path::PathBuf;

pub async fn remove_instances(launcher: &MinecraftLauncher) -> anyhow::Result<()> {
    use crate::backend::utils::formater::format_size;
    use crate::backend::utils::paths::*;
    use crate::backend::utils::sizer::calculate_directory_size;

    println!("{}", style("Delete Instances").bold().red());
    println!(
        "{}",
        style("This will delete ALL files created by this launcher!").yellow()
    );

    let game_dir = launcher.get_game_dir();

    // List of directories to delete
    let mut paths_to_delete: Vec<(&str, PathBuf)> = vec![
        ("Minecraft versions", get_versions_dir(game_dir)),
        ("Game libraries", get_libraries_dir(game_dir)),
        ("Game assets", get_assets_dir(game_dir)),
        ("Game logs", get_logs_dir(game_dir)),
        ("Natives", game_dir.join("natives")),
    ];

    // Check for additional directories
    if let Ok(java_dir) = get_java_dir() {
        if java_dir.exists() {
            paths_to_delete.push(("Java runtimes", java_dir));
        }
    }
    if let Ok(cache_dir) = get_cache_dir() {
        if cache_dir.exists() {
            paths_to_delete.push(("Launcher cache", cache_dir));
        }
    }

    // Leave only existing paths
    paths_to_delete.retain(|(_, path)| path.exists());

    if paths_to_delete.is_empty() {
        println!("{}", style("No launcher files found to delete").green());
        return Ok(());
    }

    // Calculate total size of directories to delete
    let total_size: u64 = paths_to_delete
        .iter()
        .map(|(_, path)| calculate_directory_size(path).unwrap_or(0))
        .sum();

    println!("{}", style("The following paths will be deleted:").bold());
    for (name, path) in &paths_to_delete {
        println!(
            "  {} {}",
            style("•").red(),
            style(format!("{}: {}", name, path.display())).dim()
        );
    }
    println!(
        "\nTotal size: {}",
        style(format_size(total_size as f64)).bold()
    );

    if !Confirm::new()
        .with_prompt("Are you sure you want to delete all these files?")
        .default(false)
        .interact()?
    {
        println!("{}", style("Deletion cancelled").yellow());
        return Ok(());
    }

    println!("{}", style("Deleting files...").yellow());
    let (mut deleted, mut failed) = (0, 0);

    for (name, path) in paths_to_delete {
        print!("Deleting {name}... ");
        match fs::remove_dir_all(&path) {
            Ok(_) => {
                println!("{}", style("OK").green());
                deleted += 1;
            }
            Err(e) => {
                println!("{} ({})", style("Failed").red(), e);
                failed += 1;
            }
        }
    }

    println!();
    if failed == 0 {
        println!(
            "{}",
            style(format!(
                "Successfully deleted {} directories ({})!",
                deleted,
                format_size(total_size as f64)
            ))
            .green()
            .bold()
        );
    } else {
        println!(
            "{}",
            style(format!("Deleted {deleted} directories, {failed} failed")).yellow()
        );
    }
    Ok(())
}
