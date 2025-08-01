use crate::backend::creeper::launcher::MinecraftLauncher;
use console::style;
use dialoguer::Confirm;

pub async fn remove_instances(launcher: &MinecraftLauncher) -> anyhow::Result<()> {
    use crate::backend::utils::paths::*;
    use std::fs;

    println!("{}", style("Delete Instances").bold().red());
    println!(
        "{}",
        style("This will delete ALL files created by this launcher!").yellow()
    );
    let game_dir = launcher.get_game_dir();

    let mut paths_to_delete = vec![];
    let mut total_size = 0u64;

    let dirs = [
        ("Minecraft versions", get_versions_dir(game_dir)),
        ("Game libraries", get_libraries_dir(game_dir)),
        ("Game assets", get_assets_dir(game_dir)),
        ("Game logs", get_logs_dir(game_dir)),
        ("Natives", game_dir.join("natives")),
    ];

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
    for (name, path) in dirs {
        if path.exists() {
            paths_to_delete.push((name, path));
        }
    }

    for (_, path) in &paths_to_delete {
        if let Ok(size) = crate::backend::utils::sizer::calculate_directory_size(path) {
            total_size += size;
        }
    }

    if paths_to_delete.is_empty() {
        println!("{}", style("No launcher files found to delete").green());
        return Ok(());
    }

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
        style(crate::backend::utils::formater::format_size(total_size)).bold()
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
                crate::backend::utils::formater::format_size(total_size)
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
