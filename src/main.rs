//! Entry point of the application.

pub mod backend;
pub mod frontend;

use std::sync::OnceLock;

use dioxus::{LaunchBuilder, prelude::*};
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use dioxus_router::Router;

use crate::backend::Archon;
use crate::backend::utils::application::Route;
use log::{error, info};
use std::sync::Arc;
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static ARCHON: OnceLock<Arc<Archon>> = OnceLock::new();

/// Main function for starting the application.
fn main() {
    // Logging
    env_logger::init();

    // Tokio runtime
    let runtime = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap_or_else(|_| {
                error!("Failed to create tokio runtime, exiting");
                std::process::exit(1);
            })
    });

    // Initialize Archon
    let archon = runtime.block_on(async {
        match Archon::new().await {
            Ok(archon) => {
                info!("Archon initialized successfully");
                Arc::new(archon)
            }
            Err(e) => {
                error!("Failed to create Archon: {e}");
                std::process::exit(1);
            }
        }
    });

    if ARCHON.set(archon.clone()).is_err() {
        error!("Failed to set global Archon instance, exiting...");
        std::process::exit(1);
    }

    // Run the updater in a separate thread
    runtime.spawn(async {
        backend::services::updater::check_for_updates().await;
    });

    // Initialize the launcher in a separate thread
    runtime.spawn(async {
        let _ = frontend::services::launcher::init_launcher().await;

        // Refresh version manifest after initialization
        if let Err(e) = frontend::services::launcher::refresh_version_manifest().await {
            error!("Failed to refresh version manifest: {e}");
        }
    });

    // Setup graceful shutdown handler
    let archon_shutdown = archon.clone();
    runtime.spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!("Failed to listen for Ctrl + C: {e}");
            return;
        }
        info!("Received shutdown signal, shutting down Archon...");
        if let Err(e) = archon_shutdown.shutdown().await {
            error!("Error during shutdown: {e}");
        }
        std::process::exit(0);
    });

    // Dioxus
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
                    format!(
                        "--user-data-dir={} --memory-pressure-off --max_old_space_size=512 --optimize-for-size --no-sandbox --disable-dev-shm-usage --disable-gpu --disable-software-rasterizer --disable-background-timer-throttling --disable-backgrounding-occluded-windows --disable-renderer-backgrounding",
                        user_data_dir.display()
                    ),
                );
            }
        }
    }

    LaunchBuilder::new().with_cfg(config).launch(AppRoot);
}

/// Get the global Archon instance.
pub fn get_archon() -> Option<Arc<Archon>> {
    ARCHON.get().cloned()
}

/// Root component of the application.
#[component]
fn AppRoot() -> Element {
    let is_authenticated = use_signal(|| false);
    let current_user = use_signal(|| None);
    let auth_state = frontend::services::context::AuthState {
        is_authenticated,
        current_user,
    };

    // Load saved user data on component mount
    use_effect(move || {
        let auth_state = auth_state;
        spawn(async move {
            let mut auth_state_local = auth_state;
            auth_state_local.load_saved_user().await;
        });
    });

    provide_context(auth_state);
    rsx! { Router::<Route> {} }
}
