use crate::backend::creeper::cli::launch_minecraft::launch_minecraft;
use crate::backend::creeper::cli::launch_minecraft::update_manifest;
use crate::backend::creeper::launcher::MinecraftLauncher;
use console::style;
use dialoguer::{Confirm, Select};

pub async fn interactive_mode(launcher: &mut MinecraftLauncher) -> anyhow::Result<()> {
    let options = [
        "Launch Minecraft",
        "List versions by type",
        "Update manifest",
        "Delete instances",
        "Exit",
    ];
    loop {
        match Select::new()
            .with_prompt("What would you like to do?")
            .items(&options)
            .interact()?
        {
            0 => launch_minecraft(launcher, None, false).await?,
            1 => list_versions_interactive(launcher).await?,
            2 => update_manifest(launcher).await?,
            3 => crate::backend::creeper::cli::utils::remover::remove_instances(launcher).await?,
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

    let type_selection = Select::new()
        .with_prompt("Select version type")
        .items(&type_options)
        .interact()?;
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
    let version_selection = Select::new()
        .with_prompt("Select Minecraft version")
        .items(&version_items)
        .interact()?;
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
            let jar = version_path.join(format!("{name}.jar"));
            let json = version_path.join(format!("{name}.json"));
            if jar.exists() && json.exists() {
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

    let version_items: Vec<_> = offline_versions
        .iter()
        .map(|v| format!("{v} [offline]"))
        .collect();

    let version_selection = Select::new()
        .with_prompt("Select Minecraft version")
        .items(&version_items)
        .interact()?;

    Ok(offline_versions[version_selection].clone())
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
    let type_selection = Select::new()
        .with_prompt("Which versions to show?")
        .items(&type_options)
        .interact()?;

    let filtered: Vec<_> = if type_selection < 4 {
        versions
            .iter()
            .filter(|v| v.version_type == types[type_selection].0)
            .collect()
    } else {
        versions.iter().collect()
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

    for (i, version) in filtered.iter().enumerate() {
        let color = match version.version_type.as_str() {
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
            color
        );

        if i >= 19
            && filtered.len() > 20
            && !Confirm::new()
                .with_prompt(format!("Show remaining {} versions?", filtered.len() - 20))
                .interact()?
        {
            break;
        }
    }
    Ok(())
}
