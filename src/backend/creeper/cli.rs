use crate::backend::creeper::launcher::MinecraftLauncher;
use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use dialoguer::{Confirm, Select};
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    // Custom game directory
    #[arg(short, long)]
    game_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    // Launch Minecraft with a specific version
    Launch {
        #[arg(short, long)]
        version: Option<String>,
        #[arg(long)]
        offline: bool,
    },
    // List of available Minecraft versions
    List,
    // Update version manifest
    Update,
    // Interactive mode
    Interactive,
    // Delete all launcher instances and files
    Delete,
}

pub async fn run_interactive() -> Result<()> {
    println!("{}", style("Dream Launcher [CLI mode]").bold().green());
    println!();

    let mut launcher = MinecraftLauncher::new(None).await?;
    interactive_mode(&mut launcher).await?;

    Ok(())
}

async fn launch_minecraft(
    launcher: &mut MinecraftLauncher,
    version: Option<String>,
    offline: bool,
) -> Result<()> {
    let version = match version {
        Some(v) => v,
        None => {
            if offline {
                select_offline_version(launcher).await?
            } else {
                select_version(launcher).await?
            }
        }
    };

    info!("Launching Minecraft version: {}", version);

    if offline {
        // Check if version exists locally
        let version_dir = launcher.get_game_dir().join("versions").join(&version);
        if !version_dir.exists() {
            error!(
                "Version {} not found locally. Available offline versions:",
                version
            );
            list_offline_versions(launcher).await?;
            return Err(anyhow::anyhow!("Version {} not available offline", version));
        }
    } else {
        // Check if Java is available
        if !launcher.is_java_available(&version).await? {
            let install = Confirm::new()
                .with_prompt(format!(
                    "Java runtime not found for {}. Install it?",
                    version
                ))
                .interact()?;

            if install {
                launcher.install_java(&version).await?;
            } else {
                error!("Cannot launch without Java runtime");
                return Ok(());
            }
        }

        // Download game files if needed
        match launcher.prepare_version(&version).await {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to prepare version: {}", e);
                println!("Trying to launch...");
            }
        }
    }

    // Launch the game
    launcher.launch(&version).await?;

    Ok(())
}

async fn update_manifest(launcher: &mut MinecraftLauncher) -> Result<()> {
    println!("{}", style("Updating version manifest...").bold());
    launcher.update_manifest().await?;
    println!("{}", style("Manifest updated successfully").green());
    Ok(())
}

async fn select_version(launcher: &mut MinecraftLauncher) -> Result<String> {
    let versions = launcher.get_available_versions().await?;

    // Group versions by type
    let releases: Vec<_> = versions
        .iter()
        .filter(|v| v.version_type == "release")
        .collect();
    let snapshots: Vec<_> = versions
        .iter()
        .filter(|v| v.version_type == "snapshot")
        .collect();
    let betas: Vec<_> = versions
        .iter()
        .filter(|v| v.version_type == "old_beta")
        .collect();
    let alphas: Vec<_> = versions
        .iter()
        .filter(|v| v.version_type == "old_alpha")
        .collect();

    let type_options = vec![
        format!("Releases ({})", releases.len()),
        format!("Snapshots ({})", snapshots.len()),
        format!("Beta versions ({})", betas.len()),
        format!("Alpha versions ({})", alphas.len()),
        "Show all versions".to_string(),
    ];

    let type_selection = Select::new()
        .with_prompt("Select version type")
        .items(&type_options)
        .interact()?;

    let filtered_versions = match type_selection {
        0 => releases,
        1 => snapshots,
        2 => betas,
        3 => alphas,
        4 => versions.iter().collect(),
        _ => unreachable!(),
    };

    if filtered_versions.is_empty() {
        return Err(anyhow::anyhow!("No versions available for selected type"));
    }

    let version_items: Vec<String> = filtered_versions
        .iter()
        .map(|v| format!("{} [{}]", v.id, v.version_type))
        .collect();

    let version_selection = Select::new()
        .with_prompt("Select Minecraft version")
        .items(&version_items)
        .interact()?;

    Ok(filtered_versions[version_selection].id.clone())
}

async fn interactive_mode(launcher: &mut MinecraftLauncher) -> Result<()> {
    loop {
        let options = vec![
            "Launch Minecraft",
            "List versions by type",
            "Update manifest",
            "Delete instances",
            "Exit",
        ];

        let selection = Select::new()
            .with_prompt("What would you like to do?")
            .items(&options)
            .interact()?;

        match selection {
            0 => launch_minecraft(launcher, None, false).await?,
            1 => list_versions_interactive(launcher).await?,
            2 => update_manifest(launcher).await?,
            3 => delete_instances(launcher).await?,
            4 => {
                println!("{}", style("Goodbye!").bold().green());
                break;
            }
            _ => unreachable!(),
        }

        println!();
    }

    Ok(())
}

async fn select_offline_version(launcher: &MinecraftLauncher) -> Result<String> {
    let versions_dir = launcher.get_game_dir().join("versions");
    let mut offline_versions = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&versions_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                let version_dir = entry.path();
                let jar_file = version_dir.join(format!("{}.jar", name));
                let json_file = version_dir.join(format!("{}.json", name));

                if jar_file.exists() && json_file.exists() {
                    offline_versions.push(name.to_string());
                }
            }
        }
    }

    if offline_versions.is_empty() {
        return Err(anyhow::anyhow!(
            "No offline versions found. Run the official Minecraft launcher first to download versions"
        ));
    }

    offline_versions.sort();

    let version_items: Vec<String> = offline_versions
        .iter()
        .map(|v| format!("{} [offline]", v))
        .collect();

    let version_selection = Select::new()
        .with_prompt("Select Minecraft version")
        .items(&version_items)
        .interact()?;

    Ok(offline_versions[version_selection].clone())
}

async fn list_offline_versions(launcher: &MinecraftLauncher) -> Result<()> {
    let versions_dir = launcher.get_game_dir().join("versions");
    let mut offline_versions = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&versions_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                let version_dir = entry.path();
                let jar_file = version_dir.join(format!("{}.jar", name));
                let json_file = version_dir.join(format!("{}.json", name));

                if jar_file.exists() && json_file.exists() {
                    offline_versions.push(name.to_string());
                }
            }
        }
    }

    if offline_versions.is_empty() {
        println!("No versions found");
        println!("Run the official Minecraft launcher first to download versions");
    } else {
        println!("Available versions:");
        for (i, version) in offline_versions.iter().enumerate() {
            println!("  {}. {} [offline]", i + 1, version);
        }
    }

    Ok(())
}

async fn list_versions_interactive(launcher: &MinecraftLauncher) -> Result<()> {
    let versions = launcher.get_available_versions().await?;

    let type_options = vec![
        "Releases",
        "Snapshots",
        "Beta versions",
        "Alpha versions",
        "All versions",
    ];

    let type_selection = Select::new()
        .with_prompt("Which versions to show?")
        .items(&type_options)
        .interact()?;

    let filtered_versions: Vec<_> = match type_selection {
        0 => versions
            .iter()
            .filter(|v| v.version_type == "release")
            .collect(),
        1 => versions
            .iter()
            .filter(|v| v.version_type == "snapshot")
            .collect(),
        2 => versions
            .iter()
            .filter(|v| v.version_type == "old_beta")
            .collect(),
        3 => versions
            .iter()
            .filter(|v| v.version_type == "old_alpha")
            .collect(),
        4 => versions.iter().collect(),
        _ => unreachable!(),
    };

    println!(
        "{}",
        style(format!(
            "Available {} versions:",
            type_options[type_selection]
        ))
        .bold()
    );
    println!();

    for (i, version) in filtered_versions.iter().enumerate() {
        let type_color = match version.version_type.as_str() {
            "release" => style(&version.version_type).green(),
            "snapshot" => style(&version.version_type).yellow(),
            "old_beta" => style(&version.version_type).blue(),
            "old_alpha" => style(&version.version_type).red(),
            _ => style(&version.version_type).white(),
        };

        println!(
            "  {}. {} [{}]",
            style(i + 1).dim(),
            style(&version.id).bold(),
            type_color
        );

        // Show only the first 20, then ask if user wants to see more
        if i >= 19 && filtered_versions.len() > 20 {
            let show_more = Confirm::new()
                .with_prompt(format!(
                    "Show remaining {} versions?",
                    filtered_versions.len() - 20
                ))
                .interact()?;

            if !show_more {
                break;
            }
        }
    }

    Ok(())
}

async fn delete_instances(launcher: &MinecraftLauncher) -> Result<()> {
    use crate::backend::utils::paths::*;
    use std::fs;

    println!("{}", style("Delete Instances").bold().red());
    println!(
        "{}",
        style("This will delete ALL files created by this launcher!").yellow()
    );
    println!();

    let game_dir = launcher.get_game_dir();
    let mut paths_to_delete = Vec::new();
    let mut total_size = 0u64;

    // Collect all paths that will be deleted
    let directories_to_check = vec![
        ("Minecraft versions", get_versions_dir(game_dir)),
        ("Game libraries", get_libraries_dir(game_dir)),
        ("Game assets", get_assets_dir(game_dir)),
        ("Game logs", get_logs_dir(game_dir)),
        ("Natives", game_dir.join("natives")),
    ];

    // Check launcher-specific directories
    if let Ok(java_dir) = get_java_dir() {
        if java_dir.exists() {
            paths_to_delete.push(("Java runtimes", java_dir.clone()));
        }
    }

    if let Ok(cache_dir) = get_cache_dir() {
        if cache_dir.exists() {
            paths_to_delete.push(("Launcher cache", cache_dir.clone()));
        }
    }

    // Check game directories
    for (name, path) in directories_to_check {
        if path.exists() {
            paths_to_delete.push((name, path));
        }
    }

    // Calculate total size
    for (_, path) in &paths_to_delete {
        if let Ok(size) = calculate_directory_size(path) {
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
    println!();
    println!("Total size: {}", style(format_size(total_size)).bold());
    println!();

    let confirm = Confirm::new()
        .with_prompt("Are you sure you want to delete all these files?")
        .default(false)
        .interact()?;

    if !confirm {
        println!("{}", style("Deletion cancelled").yellow());
        return Ok(());
    }

    println!("{}", style("Deleting files...").yellow());
    let mut deleted_count = 0;
    let mut failed_count = 0;

    for (name, path) in paths_to_delete {
        print!("Deleting {}... ", name);
        match fs::remove_dir_all(&path) {
            Ok(_) => {
                println!("{}", style("OK").green());
                deleted_count += 1;
            }
            Err(e) => {
                println!("{} ({})", style("Failed").red(), e);
                failed_count += 1;
            }
        }
    }

    println!();
    if failed_count == 0 {
        println!(
            "{}",
            style(format!(
                "Successfully deleted {} directories ({})!",
                deleted_count,
                format_size(total_size)
            ))
            .green()
            .bold()
        );
    } else {
        println!(
            "{}",
            style(format!(
                "Deleted {deleted_count} directories, {failed_count} failed"
            ))
            .yellow()
        );
    }

    Ok(())
}

fn calculate_directory_size(path: &std::path::Path) -> Result<u64, std::io::Error> {
    let mut size = 0;
    if path.is_file() {
        size += path.metadata()?.len();
    } else if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            size += calculate_directory_size(&entry.path())?;
        }
    }
    Ok(size)
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if size < 10.0 && unit_index > 0 {
        format!("{:.1} {}", size, UNITS[unit_index])
    } else {
        format!("{:.0} {}", size, UNITS[unit_index])
    }
}
