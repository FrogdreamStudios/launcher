//! Entry point of the application.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backend;
mod frontend;
mod utils;

use std::sync::OnceLock;

use dioxus::{LaunchBuilder, prelude::*};
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use dioxus_router::Router;

use crate::backend::utils::app::main::Route;
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Main function for starting the application.
fn main() {
    // Logging
    utils::logging::init_from_env();

    // Set icon on macOS
    #[cfg(target_os = "macos")]
    set_macos_icon();

    // Tokio runtime
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap_or_else(|_| {
                eprintln!("Failed to create tokio runtime, exiting");
                std::process::exit(1);
            })
    });

    // Dioxus
    // Original size of the application is 1280x832, but we will change it in the future
    // to 1280x832
    // Added 24px height to account for the custom titlebar
    let size = LogicalSize::new(1056.0, 709.0);

    let config = Config::default()
        .with_window(
            WindowBuilder::new()
                .with_title("Dream Launcher")
                .with_inner_size(size)
                .with_min_inner_size(size)
                .with_max_inner_size(size)
                .with_resizable(false)
                .with_decorations(false)
                .with_transparent(true),
        )
        .with_menu(None);

    // Configure WebView2 user data folder on Windows
    #[cfg(target_os = "windows")]
    {
        if let Some(home_dir) = std::env::var("USERPROFILE")
            .ok()
            .or_else(|| std::env::var("HOME").ok())
            .map(std::path::PathBuf::from)
        {
            let user_data_dir = home_dir.join(".dream-launcher");

            // Create the directory if it doesn't exist
            let _ = std::fs::create_dir_all(&user_data_dir);

            // Set environment variables for WebView2 (safe in single-threaded startup)
            unsafe {
                std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &user_data_dir);
                std::env::set_var(
                    "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
                    format!("--user-data-dir={}", user_data_dir.display()),
                );
            }
        }
    }

    LaunchBuilder::new().with_cfg(config).launch(AppRoot);
}

/// Set icon on macOS.
#[cfg(target_os = "macos")]
fn set_macos_icon() {
    if let Ok(exe_path) = std::env::current_exe() {
        let icon_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons/app_icon.icns");
        if icon_path.exists() {
            let _ = std::process::Command::new("fileicon")
                .arg("set")
                .arg(&exe_path)
                .arg(&icon_path)
                .output();
        }
    }
}

/// Root component of the application.
/// This component initializes the application state and provides context for authentication.
/// If a user is authenticated, go to the main page.
#[component]
fn AppRoot() -> Element {
    let is_authenticated = use_signal(|| false);
    provide_context(frontend::pages::auth::AuthState { is_authenticated });
    rsx! { Router::<Route> {} }
}
