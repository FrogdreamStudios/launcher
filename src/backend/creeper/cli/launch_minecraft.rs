use crate::backend::creeper::launcher::MinecraftLauncher;
use console::style;
use dialoguer::Confirm;
use tracing::{error, info};

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

    info!("Launching Minecraft version: {}", version);

    if offline {
        // Check if a version exists locally
        let version_dir = launcher.get_game_dir().join("versions").join(&version);
        if !version_dir.exists() {
            error!(
                "Version {} not found locally. Available offline versions:",
                version
            );
            crate::backend::creeper::cli::selector::list_offline_versions(launcher).await?;
            return Err(anyhow::anyhow!("Version {} not available offline", version));
        }
    } else {
        // Check if Java is available
        if !launcher.is_java_available(&version).await? {
            if Confirm::new()
                .with_prompt(format!(
                    "Java runtime not found for {}. Install it?",
                    version
                ))
                .interact()?
            {
                launcher.install_java(&version).await?;
            } else {
                error!("Cannot launch without Java runtime");
                return Ok(());
            }
        }
        if let Err(e) = launcher.prepare_version(&version).await {
            error!("Failed to prepare version: {}", e);
            println!("Trying to launch...");
        }
    }
    launcher.launch(&version).await
}

pub async fn update_manifest(launcher: &mut MinecraftLauncher) -> anyhow::Result<()> {
    println!("{}", style("Updating version manifest...").bold());
    launcher.update_manifest().await?;
    println!("{}", style("Manifest updated successfully").green());
    Ok(())
}
