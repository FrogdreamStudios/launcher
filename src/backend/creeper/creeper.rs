use crate::backend::creeper::java::java_config::JavaConfig;
use crate::backend::creeper::java::java_downloader::JavaManager;
use crate::backend::creeper::utils::file_manager::FileSystem;
use crate::backend::creeper::vanilla::downloader::Downloader;
use crate::backend::creeper::vanilla::models::{VersionDetails, VersionManifest};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::try_join;

#[derive(Serialize, Deserialize, Debug)]
struct QuickLaunchCache {
    version: String,
    java_executable: PathBuf,
    classpath: String,
    main_class: String,
    asset_index_id: String,
    created_at: u64,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    tokio::runtime::Runtime::new()?.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn Error>> {
    println!("What do you want to launch?");
    println!("[1] Vanilla Minecraft");
    println!("[2] Fabric Minecraft");
    println!("[3] Forge Minecraft");
    println!("[4] Clear quick launch cache");
    println!("[5] Nothing");

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
            "4" => {
                clear_quick_launch_cache(&minecraft_dir).await;
            }
            "5" => break,
            cmd => println!("Unknown command: {}", cmd),
        }
    }
    Ok(())
}

async fn fetch_version_manifest(
    downloader: &Downloader,
    _fs: &FileSystem,
) -> Result<VersionManifest, Box<dyn Error>> {
    let cache_path = Path::new(".cache/version_manifest.json");
    if cache_path.exists() {
        let content = fs::read_to_string(&cache_path).await?;
        let manifest: VersionManifest = serde_json::from_str(&content)?;
        println!("Loaded version manifest from cache");
        Ok(manifest)
    } else {
        let manifest_url = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
        println!("Fetching version manifest from {}", manifest_url);
        let manifest: VersionManifest = downloader.get_json(manifest_url).await?;
        println!("Version manifest fetched, caching it...");
        fs::create_dir_all(".cache").await.ok();
        fs::write(&cache_path, serde_json::to_string(&manifest)?).await?;
        Ok(manifest)
    }
}

async fn try_quick_launch(minecraft_dir: &Path, version: &str) -> Option<QuickLaunchCache> {
    let quick_launch_path = minecraft_dir.join("quick_launch.json");
    if quick_launch_path.exists() {
        if let Ok(content) = fs::read_to_string(&quick_launch_path).await {
            if let Ok(cache) = serde_json::from_str::<QuickLaunchCache>(&content) {
                if cache.version == version {
                    if cache.java_executable.exists() {
                        return Some(cache);
                    }
                }
            }
        }
    }
    None
}

async fn save_quick_launch_cache(
    minecraft_dir: &Path,
    version: &str,
    java_executable: &Path,
    classpath: &str,
    main_class: &str,
    asset_index_id: &str,
) -> Result<(), Box<dyn Error>> {
    let cache = QuickLaunchCache {
        version: version.to_string(),
        java_executable: java_executable.to_path_buf(),
        classpath: classpath.to_string(),
        main_class: main_class.to_string(),
        asset_index_id: asset_index_id.to_string(),
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    let quick_launch_path = minecraft_dir.join("quick_launch.json");
    let content = serde_json::to_string_pretty(&cache)?;
    fs::write(&quick_launch_path, content).await?;
    Ok(())
}

// TODO: debug function, delete it later
async fn clear_quick_launch_cache(minecraft_dir: &Path) {
    let quick_launch_path = minecraft_dir.join("quick_launch.json");
    if quick_launch_path.exists() {
        match fs::remove_file(&quick_launch_path).await {
            Ok(_) => println!("Quick launch cache cleared successfully"),
            Err(e) => eprintln!("Failed to clear quick launch cache: {}", e),
        }
    } else {
        println!("No quick launch cache found");
    }
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

    // Check if quick launch is possible
    if let Some(cache) = try_quick_launch(minecraft_dir, version).await {
        println!("Using quick launch cache for version {}", version);
        println!("Starting Minecraft...");

        let java_config = JavaConfig::new(version);
        let mut command = java_config.build_command_with_executable(
            &cache.java_executable,
            &cache.classpath,
            &cache.main_class,
            minecraft_dir,
            &cache.asset_index_id,
        );

        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let mut child: Child = command.spawn()?;

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

        return Ok("Minecraft started successfully".to_string());
    }

    // If a quick launch is not possible, proceed with the full launch procedure
    println!("Full launch procedure: downloading and checking files...");
    // Create unnecessary directories asynchronously
    fs::create_dir_all(&minecraft_dir.join("versions"))
        .await
        .ok();
    fs::create_dir_all(&minecraft_dir.join("libraries"))
        .await
        .ok();
    fs::create_dir_all(".cache").await.ok();

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

    // Cache manifest, to not download it every time
    let manifest = fetch_version_manifest(downloader, fs).await?;

    let version_info = manifest
        .versions
        .iter()
        .find(|v| v.id == version)
        .ok_or(format!("Version {} not found", version))?;
    println!("Found version {}", version);

    // Cache version details
    let version_cache_path = format!(".cache/version_{}.json", version);
    let version_details: VersionDetails = if fs::metadata(&version_cache_path).await.is_ok() {
        let content = fs::read_to_string(&version_cache_path).await?;
        serde_json::from_str(&content)?
    } else {
        let details: VersionDetails = downloader.get_json(&version_info.url).await?;
        fs::write(&version_cache_path, serde_json::to_string(&details)?).await?;
        details
    };
    println!("Version details fetched");

    let versions_dir = minecraft_dir.join("versions");
    let version_dir = versions_dir.join(version);
    let libraries_dir = minecraft_dir.join("libraries");
    let client_jar_path = version_dir.join(format!("{}.jar", version));

    let client_jar_needed = !fs.exists(&client_jar_path);

    let client_jar_fut = if client_jar_needed {
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

    let downloader_clone1 = downloader.clone();
    let downloader_clone2 = downloader.clone();

    let libs_fut = async {
        downloader_clone1
            .download_libraries(&version_details.libraries, &libraries_dir, &version_dir)
            .await
    };
    let assets_fut = async {
        downloader_clone2
            .download_assets(&version_details.asset_index, minecraft_dir)
            .await
    };

    let _ = match client_jar_fut {
        Some(fut) => try_join!(fut, libs_fut, assets_fut)?,
        None => try_join!(async { Ok(()) }, libs_fut, assets_fut)?,
    };

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

                        // Save cache for quick launch on a successful start
                        println!("Saving quick launch cache for future runs...");
                        if let Err(e) = save_quick_launch_cache(
                            minecraft_dir,
                            version,
                            &java_executable,
                            &classpath,
                            &version_details.main_class,
                            &version_details.asset_index.id,
                        ).await {
                            eprintln!("Failed to save quick launch cache: {}", e);
                        } else {
                            println!("Quick launch cache saved");
                        }
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

                        // Save cache for quick launch on a successful start
                        println!("Saving quick launch cache for future runs...");
                        if let Err(e) = save_quick_launch_cache(
                            minecraft_dir,
                            version,
                            &java_executable,
                            &classpath,
                            &version_details.main_class,
                            &version_details.asset_index.id,
                        ).await {
                            eprintln!("Failed to save quick launch cache: {}", e);
                        } else {
                            println!("Quick launch cache saved");
                        }
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
