//! Version selection and interactive CLI utilities.

use crate::backend::creeper::cli::launch_minecraft::launch_minecraft;
use crate::backend::creeper::cli::launch_minecraft::update_manifest;
use crate::backend::creeper::launcher::MinecraftLauncher;
use crate::backend::utils::file_utils::is_minecraft_version_complete;
use std::io::{self, Write};

/// Runs the interactive CLI mode with main menu options.
pub async fn interactive_mode(launcher: &mut MinecraftLauncher) -> anyhow::Result<()> {
    let options = [
        "Launch Minecraft",
        "List versions by type",
        "Update manifest",
        "Delete instances",
        "Exit",
    ];

    loop {
        println!("\nWhat would you like to do?");
        for (i, option) in options.iter().enumerate() {
            println!("  {}. {}", i + 1, option);
        }

        print!("Enter your choice (1-5): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let choice = input.trim().parse::<usize>().unwrap_or(0);

        match choice {
            1 => launch_minecraft(launcher, None, false).await?,
            2 => list_versions_interactive(launcher).await?,
            3 => update_manifest(launcher).await?,
            4 => crate::backend::creeper::cli::utils::remover::remove_instances(launcher).await?,
            5 => {
                println!("Goodbye!");
                break;
            }
            _ => println!("Invalid selection. Please enter 1-5."),
        }
    }
    Ok(())
}

/// Provides interactive version selection with type filtering.
pub async fn select_version(launcher: &MinecraftLauncher) -> anyhow::Result<String> {
    let versions = launcher.get_available_versions().await?;
    let types = [
        ("release", "Releases"),
        ("snapshot", "Snapshots"),
        ("old_beta", "Beta versions"),
        ("old_alpha", "Alpha versions"),
    ];
    let type_options: Vec<_> = types
        .iter()
        .map(|(_, name)| {
            format!(
                "{} ({})",
                name,
                versions.iter().filter(|v| v.version_type == *name).count()
            )
        })
        .collect();
    let type_options = [type_options, vec!["Show all versions".to_string()]].concat();

    println!("\nSelect version type:");
    for (i, option) in type_options.iter().enumerate() {
        println!("  {}. {}", i + 1, option);
    }
    print!("Enter your choice: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let type_selection = input.trim().parse::<usize>().unwrap_or(1).saturating_sub(1);
    let filtered: Vec<_> = if type_selection < 4 {
        versions
            .iter()
            .filter(|v| v.version_type == types[type_selection].0)
            .collect()
    } else {
        versions.iter().collect()
    };

    if filtered.is_empty() {
        return Err(anyhow::anyhow!("No versions available for selected type"));
    }

    let version_items: Vec<_> = filtered
        .iter()
        .map(|v| format!("{} [{}]", v.id, v.version_type))
        .collect();
    println!("\nSelect Minecraft version:");
    for (i, item) in version_items.iter().enumerate() {
        println!("  {}. {}", i + 1, item);
        if i >= 19 && version_items.len() > 20 {
            println!("  ... and {} more", version_items.len() - 20);
            break;
        }
    }
    print!("Enter your choice: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let version_selection = input.trim().parse::<usize>().unwrap_or(1).saturating_sub(1);
    Ok(filtered[version_selection].id.clone())
}

async fn get_offline_versions(launcher: &MinecraftLauncher) -> Vec<String> {
    let versions_dir = launcher.get_game_dir().join("versions");
    std::fs::read_dir(&versions_dir)
        .ok()
        .into_iter()
        .flat_map(|e| e.flatten())
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_string();
            let version_path = entry.path();
            if is_minecraft_version_complete(&version_path, &name) {
                Some(name)
            } else {
                None
            }
        })
        .collect()
}

pub async fn list_offline_versions(launcher: &MinecraftLauncher) -> anyhow::Result<()> {
    let offline_versions = get_offline_versions(launcher).await;

    if offline_versions.is_empty() {
        println!(
            "No versions found\nRun the official Minecraft launcher first to download versions"
        );
    } else {
        println!("Available versions:");
        for (i, version) in offline_versions.iter().enumerate() {
            println!("  {}. {} [offline]", i + 1, version);
        }
    }
    Ok(())
}

pub async fn select_offline_version(launcher: &MinecraftLauncher) -> anyhow::Result<String> {
    let offline_versions = get_offline_versions(launcher).await;

    if offline_versions.is_empty() {
        return Err(anyhow::anyhow!(
            "No offline versions found. Run the official Minecraft launcher first to download versions"
        ));
    }

    println!("\nSelect offline version:");
    for (i, version) in offline_versions.iter().enumerate() {
        println!("  {}. {} [offline]", i + 1, version);
    }

    print!("Enter your choice: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let choice = input.trim().parse::<usize>().unwrap_or(1);
    if choice < 1 || choice > offline_versions.len() {
        return Err(anyhow::anyhow!("Invalid selection"));
    }

    Ok(offline_versions[choice - 1].clone())
}

pub async fn list_versions_interactive(launcher: &MinecraftLauncher) -> anyhow::Result<()> {
    let versions = launcher.get_available_versions().await?;
    let types = [
        ("release", "Releases"),
        ("snapshot", "Snapshots"),
        ("old_beta", "Beta versions"),
        ("old_alpha", "Alpha versions"),
    ];
    let type_options: Vec<_> = types
        .iter()
        .map(|(_, name)| *name)
        .chain(std::iter::once("All versions"))
        .collect();
    println!("\nWhich versions to show?");
    for (i, option) in type_options.iter().enumerate() {
        println!("  {}. {}", i + 1, option);
    }
    print!("Enter your choice: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let type_selection = input.trim().parse::<usize>().unwrap_or(1).saturating_sub(1);

    let filtered: Vec<_> = if type_selection < 4 {
        versions
            .iter()
            .filter(|v| v.version_type == types[type_selection].0)
            .collect()
    } else {
        versions.iter().collect()
    };

    println!("Available {} versions:", type_options[type_selection]);
    println!();

    for (i, version) in filtered.iter().enumerate() {
        let version_type = &version.version_type;
        println!("  {}. {} [{}]", i + 1, &version.id, version_type);

        if i >= 19 && filtered.len() > 20 {
            print!("Show remaining {} versions? (y/n): ", filtered.len() - 20);
            io::stdout().flush().unwrap_or(());

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                let show_more =
                    input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes";
                if !show_more {
                    break;
                }
            } else {
                break;
            }
        }
    }
    Ok(())
}
