mod backend;
mod frontend;
use crate::backend::utils::assets::ensure_assets_loaded;
use crate::backend::utils::css_loader::ensure_css_loaded;
use crate::backend::utils::route::Route;
use dioxus::LaunchBuilder;
use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder, use_window};
use dioxus_router::Router;
use image::GenericImageView;
use std::sync::OnceLock;
use tao::window::Icon;
use tokio::runtime::Runtime;
use tracing_subscriber::EnvFilter;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn main() {
    // Logging setup
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("warn,hyper=warn,h2=warn"))
        .init();

    ensure_css_loaded();
    ensure_assets_loaded();

    // Icon for macOS
    #[cfg(target_os = "macos")]
    {
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

    // Initialize runtime once
    let _rt = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap_or_else(|_| {
                eprintln!("Failed to create tokio runtime, exiting");
                std::process::exit(1);
            })
    });

    let size = LogicalSize::new(1280.0, 832.0);

    // Load icon
    let icon = {
        let icon_bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/images/other/icon_64.png"
        ));
        match image::load_from_memory(icon_bytes) {
            Ok(image) => {
                let (width, height) = image.dimensions();
                let rgba = image.into_rgba8().into_raw();
                match Icon::from_rgba(rgba, width, height) {
                    Ok(icon) => Some(icon),
                    Err(_) => {
                        eprintln!("Failed to create icon from image data");
                        None
                    }
                }
            }
            Err(_) => {
                eprintln!("Failed to load icon image from embedded data");
                None
            }
        }
    };

    let config = Config::default()
        .with_window(
            WindowBuilder::new()
                .with_title("Dream Launcher")
                .with_inner_size(size)
                .with_min_inner_size(size)
                .with_resizable(false)
                .with_window_icon(icon),
        )
        .with_menu(None);

    LaunchBuilder::new().with_cfg(config).launch(ModeSelector);
}

#[component]
fn ModeSelector() -> Element {
    let mut mode = use_signal(|| None::<bool>);
    let window = use_window();

    // UI mode
    if *mode.read() == Some(true) {
        return rsx! { AppRoot {} };
    }

    // CLI mode
    if *mode.read() == Some(false) {
        use_future(move || {
            let window = window.clone();
            async move {
                if let Err(e) = backend::creeper::cli::main::run_interactive().await {
                    eprintln!("CLI error: {e}");
                }
                window.close();
            }
        });

        return rsx! {};
    }

    rsx! {
        div {
            style: "display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100vh;",
            h2 { "What do you want to launch?" }
            div {
                style: "display: flex; gap: 24px; margin-top: 24px;",
                button {
                    style: "padding: 12px 32px; font-size: 1.1rem;",
                    onclick: move |_| mode.set(Some(true)),
                    "Launch UI Mode"
                }
                button {
                    style: "padding: 12px 32px; font-size: 1.1rem;",
                    onclick: {
                        let mut mode = mode;
                        let window = window;
                        move |_| {
                            mode.set(Some(false));
                            window.set_visible(false);
                        }
                    },
                    "Launch CLI Mode"
                }
            }
        }
    }
}

#[component]
fn AppRoot() -> Element {
    let is_authenticated = use_signal(|| false);
    provide_context(frontend::ui::auth::auth_context::AuthState { is_authenticated });
    rsx! { Router::<Route> {} }
}
