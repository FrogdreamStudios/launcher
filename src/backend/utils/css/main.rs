//! Asset and CSS loading/caching utilities.

use base64::{Engine as _, engine::general_purpose};
use std::{collections::HashMap, sync::OnceLock};

static ASSET_CACHE: OnceLock<HashMap<&'static str, String>> = OnceLock::new();
static CSS_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

macro_rules! embed_asset {
    ($name:expr, $path:expr) => {
        (
            $name,
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)),
        )
    };
}

const ASSETS: &[(&str, &[u8])] = &[
    embed_asset!("logo", "assets/images/other/logo.png"),
    embed_asset!("home", "assets/images/buttons/home.png"),
    embed_asset!("packs", "assets/images/buttons/packs.png"),
    embed_asset!("settings", "assets/images/buttons/settings.png"),
    embed_asset!("cloud", "assets/images/buttons/cloud.png"),
    embed_asset!("plus", "assets/images/buttons/plus.png"),
    embed_asset!("microsoft", "assets/images/other/microsoft.png"),
    embed_asset!("play", "assets/images/buttons/play.png"),
    embed_asset!("additional", "assets/images/buttons/additional.png"),
    embed_asset!("change", "assets/images/buttons/change.png"),
    embed_asset!("delete", "assets/images/buttons/delete.png"),
    embed_asset!("folder", "assets/images/buttons/folder.png"),
    embed_asset!("debug", "assets/images/buttons/debug.png"),
    embed_asset!("add", "assets/images/buttons/add.png"),
    embed_asset!("open", "assets/images/buttons/open.png"),
    embed_asset!("minecraft_icon", "assets/images/other/minecraft.png"),
    embed_asset!(
        "minecraft_wiki_icon",
        "assets/images/other/minecraft_wiki.png"
    ),
    embed_asset!(
        "planet_minecraft_icon",
        "assets/images/other/planet_minecraft.png"
    ),
    embed_asset!("curseforge_icon", "assets/images/other/curseforge.png"),
    embed_asset!("namemc_icon", "assets/images/other/namemc.png"),
];

pub struct ResourceLoader;

impl ResourceLoader {
    fn get_all_assets() -> HashMap<&'static str, String> {
        ASSETS
            .iter()
            .map(|&(n, bytes)| {
                let data = general_purpose::STANDARD.encode(bytes);
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
        style!("settings", "/assets/styles/components/settings.css");
        style!("progress", "/assets/styles/components/progress.css");
        style!("new", "/assets/styles/components/new.css");
        style!("browser", "/assets/styles/components/browser.css");
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
            "settings",
            "progress",
            "new",
            "browser",
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
