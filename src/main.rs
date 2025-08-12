mod backend;
mod frontend;

use crate::backend::utils::{
    assets::ensure_assets_loaded, css_loader::ensure_css_loaded, route::Route,
};
use dioxus::{LaunchBuilder, prelude::*};
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use dioxus_router::Router;
use image::GenericImageView;
use std::sync::OnceLock;
use tao::window::Icon;
use tokio::runtime::Runtime;
use tracing_subscriber::EnvFilter;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn main() {
    // Logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("warn,hyper=warn,h2=warn"))
        .init();

    ensure_css_loaded();
    ensure_assets_loaded();

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
                .with_resizable(false)
                .with_window_icon(load_icon()),
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

fn load_icon() -> Option<Icon> {
    let icon_bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/images/other/icon_64.png"
    ));
    image::load_from_memory(icon_bytes).ok().and_then(|img| {
        let (w, h) = img.dimensions();
        Icon::from_rgba(img.into_rgba8().into_raw(), w, h).ok()
    })
}

#[component]
fn AppRoot() -> Element {
    let is_authenticated = use_signal(|| false);
    provide_context(frontend::ui::auth::auth_context::AuthState { is_authenticated });
    rsx! { Router::<Route> {} }
}
