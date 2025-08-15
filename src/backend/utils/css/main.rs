//! Asset and CSS loading/caching utilities.

use base64::{Engine as _, engine::general_purpose};
use std::{collections::HashMap, fs, sync::OnceLock};

static ASSET_CACHE: OnceLock<HashMap<&'static str, String>> = OnceLock::new();
static CSS_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

const ASSETS: &[(&str, &str)] = &[
    ("logo", "assets/images/other/logo.png"),
    ("home", "assets/images/buttons/home.png"),
    ("packs", "assets/images/buttons/packs.png"),
    ("settings", "assets/images/buttons/settings.png"),
    ("cloud", "assets/images/buttons/cloud.png"),
    ("plus", "assets/images/buttons/plus.png"),
    ("microsoft", "assets/images/other/microsoft.png"),
    ("play", "assets/images/buttons/play.png"),
    ("additional", "assets/images/buttons/additional.png"),
    ("change", "assets/images/buttons/change.png"),
    ("delete", "assets/images/buttons/delete.png"),
    ("folder", "assets/images/buttons/folder.png"),
    ("debug", "assets/images/buttons/debug.png"),
    ("add", "assets/images/buttons/add.png"),
];

pub struct ResourceLoader;

impl ResourceLoader {
    fn get_all_assets() -> HashMap<&'static str, String> {
        ASSETS
            .iter()
            .map(|&(n, p)| {
                let data = fs::read(p).map_or("".into(), |b| general_purpose::STANDARD.encode(b));
                (n, format!("data:image/png;base64,{data}"))
            })
            .collect()
    }

    pub fn get_asset(name: &str) -> String {
        ASSET_CACHE
            .get_or_init(Self::get_all_assets)
            .get(name)
            .cloned()
            .unwrap_or_else(|| "data:image/png;base64,".into())
    }

    fn get_all_styles() -> HashMap<&'static str, &'static str> {
        let mut m = HashMap::new();
        macro_rules! style {
            ($n:expr, $p:expr) => {
                m.insert($n, include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $p)));
            };
        }
        style!("base", "/assets/styles/base.css");
        style!("animations", "/assets/styles/animations.css");
        style!("auth", "/assets/styles/auth.css");
        style!("tailwind", "/assets/styles/output.css");
        style!("logo", "/assets/styles/components/logo.css");
        style!("navigation", "/assets/styles/components/navigation.css");
        style!("chat", "/assets/styles/components/chat.css");
        style!("home", "/assets/styles/components/home.css");
        style!("news", "/assets/styles/components/news.css");
        style!("context_menu", "/assets/styles/components/context_menu.css");
        style!("debug", "/assets/styles/components/debug.css");
        m
    }

    pub fn get_css(name: &str) -> &'static str {
        CSS_CACHE
            .get_or_init(Self::get_all_styles)
            .get(name)
            .copied()
            .unwrap_or("")
    }

    pub fn combine_css(styles: &[&str]) -> String {
        styles
            .iter()
            .map(|&n| Self::get_css(n))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn get_combined_main_css() -> String {
        Self::combine_css(&[
            "base",
            "animations",
            "logo",
            "navigation",
            "chat",
            "home",
            "news",
            "context_menu",
            "debug",
            "tailwind",
        ])
    }
}

#[macro_export]
macro_rules! include_styles {
    ($($style:expr),*) => {
        $crate::backend::utils::resource_loader::ResourceLoader::combine_css(&[$($style),*])
    };
}
