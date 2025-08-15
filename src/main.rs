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
    let size = LogicalSize::new(1280.0, 832.0);

    let config = Config::default()
        .with_window(
            WindowBuilder::new()
                .with_title("Dream Launcher")
                .with_inner_size(size)
                .with_min_inner_size(size)
                .with_resizable(false),
        )
        .with_menu(None);

    LaunchBuilder::new().with_cfg(config).launch(AppRoot);
}

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

#[component]
fn AppRoot() -> Element {
    let is_authenticated = use_signal(|| false);
    provide_context(frontend::pages::auth::AuthState { is_authenticated });
    rsx! { Router::<Route> {} }
}
