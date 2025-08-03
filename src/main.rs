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

    // Initialize runtime once
    let _rt = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime")
    });

    let size = LogicalSize::new(1280.0, 832.0);

    // Load icon - use a smaller PNG for window icon (better performance)
    let icon = {
        let icon_bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/images/other/icon.png"
        ));
        let image = image::load_from_memory(icon_bytes).expect("Failed to load icon image");
        let (width, height) = image.dimensions();
        let rgba = image.into_rgba8().into_raw();
        Some(Icon::from_rgba(rgba, width, height).expect("Failed to create icon"))
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
    if let Some(true) = *mode.read() {
        return rsx! { AppRoot {} };
    }

    // CLI mode
    if let Some(false) = *mode.read() {
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
                    "UI in dev"
                }
                button {
                    style: "padding: 12px 32px; font-size: 1.1rem;",
                    onclick: {
                        let mut mode = mode;
                        let window = window.clone();
                        move |_| {
                            mode.set(Some(false));
                            window.set_visible(false)
                        }
                    },
                    "CLI in dev"
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
