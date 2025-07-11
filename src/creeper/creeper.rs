use crate::creeper::java::java_config::JavaConfig;
use crate::creeper::java::java_downloader::JavaManager;
use crate::creeper::utils::file_manager::FileSystem;
use crate::creeper::vanilla::downloader::Downloader;
use crate::creeper::vanilla::models::{VersionDetails, VersionManifest};
use std::error::Error;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::try_join;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    println!("What do you want to launch?");
    println!("[1] Vanilla Minecraft");
    println!("[2] Fabric Minecraft");
    println!("[3] Forge Minecraft");
    println!("[4] Nothing, exit");

    let downloader = Downloader::new();
    let java_manager = JavaManager::new();

    // Set the path to the .minecraft directory
    let minecraft_dir = Path::new(".minecraft");
    let fs = FileSystem::new();

    loop {
        print!("\nEnter command: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        match input.trim() {
            "1" => {
                println!("What version of Minecraft do you want to launch? (e.g., 1.21.7)");
                let mut version_input = String::new();
                io::stdin().read_line(&mut version_input)?;
                let version = version_input.trim();
                if version.is_empty() {
                    println!("No version specified, using default 1.21.7");
                    let version = "1.21.7";
                    if let Err(e) =
                        start_minecraft(&downloader, &java_manager, &fs, &minecraft_dir, version)
                            .await
                    {
                        eprintln!("Failed to start Minecraft: {}", e);
                    }
                } else {
                    println!("Using version {}", version);
                    if let Err(e) =
                        start_minecraft(&downloader, &java_manager, &fs, &minecraft_dir, version)
                            .await
                    {
                        eprintln!("Failed to start Minecraft: {}", e);
                    }
                }
            }
            "2" => {
                println!("Fabric Minecraft is not implemented yet.");
                break;
            }
            "3" => {
                println!("Forge Minecraft is not implemented yet.");
                break;
            }
            "4" => break,
            cmd => println!("Unknown command: {}", cmd),
        }
    }
    Ok(())
}

async fn start_minecraft(
    downloader: &Downloader,
    java_manager: &JavaManager,
    fs: &FileSystem,
    minecraft_dir: &Path,
    version: &str,
) -> Result<String, Box<dyn Error>> {
    // Start the timer to measure launch time
    let start_time = Instant::now();

    // Check for Java
    println!("Checking Java compatibility for Minecraft {}...", version);
    let java_executable = match java_manager.get_java_executable(Some(version)).await {
        Ok(path) => {
            println!("Java ready: {}", path.display());
            path
        }
        Err(e) => {
            eprintln!("Failed to get Java: {}", e);
            return Err(e);
        }
    };

    let manifest_url = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

    println!("Fetching version manifest from {}", manifest_url);
    let manifest: VersionManifest = downloader.get_json(manifest_url).await?;
    println!("Version manifest fetched");

    let version_info = manifest
        .versions
        .iter()
        .find(|v| v.id == version)
        .ok_or(format!("Version {} not found", version))?;
    println!("Found version {}", version);

    println!("Fetching version details from {}", version_info.url);
    let version_details: VersionDetails = downloader.get_json(&version_info.url).await?;
    println!("Version details fetched");

    let versions_dir = minecraft_dir.join("versions");
    let version_dir = versions_dir.join(version);
    let libraries_dir = minecraft_dir.join("libraries");
    let client_jar_path = version_dir.join(format!("{}.jar", version));

    let client_jar_fut = if !fs.exists(&client_jar_path) {
        Some(downloader.download_file_if_not_exists(
            &version_details.downloads.client.url,
            &client_jar_path,
            None,
            None,
        ))
    } else {
        println!("Client already exists at {}", client_jar_path.display());
        None
    };

    let libs_fut =
        downloader.download_libraries(&version_details.libraries, &libraries_dir, &version_dir);
    let assets_fut = downloader.download_assets(&version_details.asset_index, minecraft_dir);

    if let Some(fut) = client_jar_fut {
        try_join!(fut, libs_fut, assets_fut)?;
        println!("Client downloaded");
    } else {
        try_join!(libs_fut, assets_fut)?;
    }

    println!("Building classpath...");
    let classpath = fs.build_classpath(&libraries_dir, &client_jar_path)?;

    // Use Java
    let java_version = Command::new(&java_executable)
        .arg("-version")
        .output()
        .await?;
    println!(
        "Using Java: {:?}",
        String::from_utf8_lossy(&java_version.stderr)
    );

    println!("Starting Minecraft...");

    // Version of Minecraft
    let java_config = JavaConfig::new(version);

    // Custom Java
    let mut command = java_config.build_command_with_executable(
        &java_executable,
        &classpath,
        &version_details.main_class,
        minecraft_dir,
        &version_details.asset_index.id,
    );

    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let mut child: Child = command.spawn()?;
    println!("Java command: {:?}", command);

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut sound_engine_started = false;
    while !sound_engine_started {
        tokio::select! {
            line = stdout_reader.next_line() => {
                if let Some(line) = line? {
                    println!("{}", line);
                    if line.contains("Sound engine started") {
                        sound_engine_started = true;
                        let elapsed_time = start_time.elapsed();
                        println!(
                            "Time to launch Minecraft: {:.2} seconds",
                            elapsed_time.as_secs_f64()
                        );
                    }
                }
            }
            line = stderr_reader.next_line() => {
                if let Some(line) = line? {
                    println!("{}", line);
                    if line.contains("Sound engine started") {
                        sound_engine_started = true;
                        let elapsed_time = start_time.elapsed();
                        println!(
                            "Time to launch Minecraft: {:.2} seconds",
                            elapsed_time.as_secs_f64()
                        );
                    }
                }
            }
            status = child.wait() => {
                let status = status?;
                println!("Minecraft exited with code: {}", status.code().unwrap_or(-1));
                break;
            }
        }
    }

    if !sound_engine_started {
        println!("Sound engine started not detected before process exit");
    }

    Ok("Minecraft started successfully".to_string())
}