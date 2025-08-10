use crate::backend::creeper::launcher::MinecraftLauncher;
use std::io::{self, Write};
use tracing::{error, info, warn};

pub async fn launch_minecraft(
    launcher: &mut MinecraftLauncher,
    version: Option<String>,
    offline: bool,
) -> anyhow::Result<()> {
    let version = match version {
        Some(v) => v,
        None => {
            if offline {
                crate::backend::creeper::cli::selector::select_offline_version(launcher).await?
            } else {
                crate::backend::creeper::cli::selector::select_version(launcher).await?
            }
        }
    };

    info!("Launching Minecraft version: {version}");

    if offline {
        // Check if a version exists locally
        let version_dir = launcher.get_game_dir().join("versions").join(&version);
        if !version_dir.exists() {
            error!("Version {version} not found locally. Available offline versions:");
            crate::backend::creeper::cli::selector::list_offline_versions(launcher).await?;
            return Err(anyhow::anyhow!("Version {version} not available offline"));
        }

        println!("Version {version} found locally, skipping download checks...");
    } else {
        // Online mode - check if version exists online first
        println!("Checking version {version} availability online...");

        // Try to prepare version (download if needed)
        match launcher.prepare_version(&version).await {
            Ok(_) => {
                println!("Version {version} prepared successfully");
            }
            Err(e) => {
                error!("Failed to prepare version: {e}");

                // Check if we have it offline as fallback
                let version_dir = launcher.get_game_dir().join("versions").join(&version);
                if version_dir.exists() {
                    println!("Found version offline, attempting to use local version...");
                } else {
                    return Err(anyhow::anyhow!(
                        "Version {version} not available online or offline"
                    ));
                }
            }
        }

        // Check if Java is available
        if !launcher.is_java_available(&version).await? {
            print!(
                "Java runtime not found for {}. Install it? (y/n): ",
                version
            );
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            let install =
                input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes";

            if install {
                match launcher.install_java(&version).await {
                    Ok(_) => println!("Java installed successfully"),
                    Err(e) => {
                        warn!("Failed to install Java: {e}");
                        println!("Continuing without Java installation - may cause launch issues");
                    }
                }
            } else {
                println!("Continuing without Java installation - may cause launch issues");
            }
        }
    }
    launcher.launch(&version).await
}

pub async fn update_manifest(launcher: &mut MinecraftLauncher) -> anyhow::Result<()> {
    println!("Updating version manifest...");
    launcher.update_manifest().await?;
    println!("Manifest updated successfully");
    Ok(())
}
