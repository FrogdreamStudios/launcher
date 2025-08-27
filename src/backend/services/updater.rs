use crate::{log_error, log_info};
use autoupdater::apis::DownloadApiTrait;
use autoupdater::apis::github::GithubApi;
use autoupdater::cargo_crate_version;
use std::cmp::Ordering;

pub async fn check_for_updates() {
    log_info!("Checking for updates...");

    let mut api = GithubApi::new("FrogdreamStudios", "launcher");
    api = api.current_version(cargo_crate_version!());

    let sort_func: Option<&fn(&str, &str) -> Ordering> = None;
    match api.get_newer(sort_func) {
        Ok(Some(download)) => {
            log_info!(
                "New version found: {}. Searching for compatible binary...",
                download.tag_name
            );

            let target_os_binary_name = match std::env::consts::OS {
                "windows" => "DreamLauncher-Windows",
                "macos" => "DreamLauncher-macOS",
                "linux" => "DreamLauncher-Linux",
                _ => "",
            };

            if target_os_binary_name.is_empty() {
                log_error!("Unsupported OS for auto-update");
                return;
            }

            let asset_to_download = download
                .assets
                .iter()
                .find(|asset| asset.name.contains(target_os_binary_name));

            if let Some(asset) = asset_to_download {
                log_info!("Downloading: {}", asset.name);
                let download_callback: Option<&fn(f32)> = None;
                match api.download(asset, download_callback) {
                    Ok(_) => log_info!("Update downloaded successfully!"),
                    Err(e) => log_error!("Failed to download update: {e:?}"),
                }
            } else {
                log_error!("No compatible binary found for this version");
            }
        }
        Ok(None) => {
            log_info!("No new updates found");
        }
        Err(e) => {
            log_error!("Failed to check for updates: {e:?}");
        }
    }
}
