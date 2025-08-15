//! Asset and CSS loading/caching utilities.

use base64::Engine as _;
use base64::engine::general_purpose;
use std::{collections::HashMap, fs, sync::OnceLock};

static ASSET_CACHE: OnceLock<HashMap<&'static str, String>> = OnceLock::new();
static CSS_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

pub struct ResourceLoader;

impl ResourceLoader {
    fn get_asset_path(name: &str) -> Option<&'static str> {
        match name {
            "logo" => Some("assets/images/other/logo.png"),
            "home" => Some("assets/images/buttons/home.png"),
            "packs" => Some("assets/images/buttons/packs.png"),
            "settings" => Some("assets/images/buttons/settings.png"),
            "cloud" => Some("assets/images/buttons/cloud.png"),
            "plus" => Some("assets/images/buttons/plus.png"),
            "microsoft" => Some("assets/images/other/microsoft.png"),
            "play" => Some("assets/images/buttons/play.png"),
            "additional" => Some("assets/images/buttons/additional.png"),
            "change" => Some("assets/images/buttons/change.png"),
            "delete" => Some("assets/images/buttons/delete.png"),
            "folder" => Some("assets/images/buttons/folder.png"),
            "debug" => Some("assets/images/buttons/debug.png"),
            "add" => Some("assets/images/buttons/add.png"),
            _ => None,
        }
    }

    fn get_all_assets() -> HashMap<&'static str, String> {
        let mut map = HashMap::new();
        for &name in &[
            "logo",
            "home",
            "packs",
            "settings",
            "cloud",
            "plus",
            "microsoft",
            "play",
            "additional",
            "change",
            "delete",
            "folder",
            "debug",
            "add",
        ] {
            map.insert(name, Self::load_asset(name));
        }
        map
    }

    fn load_asset(name: &str) -> String {
        if let Some(path) = Self::get_asset_path(name) {
            match fs::read(path) {
                Ok(bytes) => format!(
                    "data:image/png;base64,{}",
                    general_purpose::STANDARD.encode(bytes)
                ),
                Err(_) => "data:image/png;base64,".to_string(),
            }
        } else {
            "data:image/png;base64,".to_string()
        }
    }

    pub fn get_asset(name: &str) -> String {
        ASSET_CACHE
            .get_or_init(Self::get_all_assets)
            .get(name)
            .cloned()
            .unwrap_or_else(|| "data:image/png;base64,".to_string())
    }

    pub fn get_logo() -> String {
        Self::get_asset("logo")
    }
    pub fn get_home() -> String {
        Self::get_asset("home")
    }
    pub fn get_packs() -> String {
        Self::get_asset("packs")
    }
    pub fn get_settings() -> String {
        Self::get_asset("settings")
    }
    pub fn get_cloud() -> String {
        Self::get_asset("cloud")
    }
    pub fn get_plus() -> String {
        Self::get_asset("plus")
    }
    pub fn get_microsoft() -> String {
        Self::get_asset("microsoft")
    }
    pub fn get_play() -> String {
        Self::get_asset("play")
    }
    pub fn get_additional() -> String {
        Self::get_asset("additional")
    }
    pub fn get_change() -> String {
        Self::get_asset("change")
    }
    pub fn get_delete() -> String {
        Self::get_asset("delete")
    }
    pub fn get_folder() -> String {
        Self::get_asset("folder")
    }
    pub fn get_debug() -> String {
        Self::get_asset("debug")
    }
    pub fn get_add() -> String {
        Self::get_asset("add")
    }

    fn get_all_styles() -> HashMap<&'static str, &'static str> {
        [
            (
                "base",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/base.css"
                )),
            ),
            (
                "animations",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/animations.css"
                )),
            ),
            (
                "auth",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/auth.css"
                )),
            ),
            (
                "tailwind",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/output.css"
                )),
            ),
            (
                "logo",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/components/logo.css"
                )),
            ),
            (
                "navigation",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/components/navigation.css"
                )),
            ),
            (
                "chat",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/components/chat.css"
                )),
            ),
            (
                "home",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/components/home.css"
                )),
            ),
            (
                "news",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/components/news.css"
                )),
            ),
            (
                "context_menu",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/components/context_menu.css"
                )),
            ),
            (
                "debug",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/components/debug.css"
                )),
            ),
        ]
        .into_iter()
        .collect()
    }

    pub fn get_css(name: &str) -> &'static str {
        CSS_CACHE
            .get_or_init(Self::get_all_styles)
            .get(name)
            .copied()
            .unwrap_or("")
    }

    pub fn get_chat_css() -> &'static str {
        Self::get_css("chat")
    }
    pub fn get_base_css() -> &'static str {
        Self::get_css("base")
    }
    pub fn get_animations_css() -> &'static str {
        Self::get_css("animations")
    }
    pub fn get_auth_css() -> &'static str {
        Self::get_css("auth")
    }
    pub fn get_tailwind_css() -> &'static str {
        Self::get_css("tailwind")
    }
    pub fn get_logo_css() -> &'static str {
        Self::get_css("logo")
    }
    pub fn get_navigation_css() -> &'static str {
        Self::get_css("navigation")
    }
    pub fn get_home_css() -> &'static str {
        Self::get_css("home")
    }
    pub fn get_news_css() -> &'static str {
        Self::get_css("news")
    }
    pub fn get_context_menu_css() -> &'static str {
        Self::get_css("context_menu")
    }
    pub fn get_debug_css() -> &'static str {
        Self::get_css("debug")
    }

    pub fn combine_css(styles: &[&str]) -> String {
        styles
            .iter()
            .map(|&name| Self::get_css(name))
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

    pub fn get_combined_auth_css() -> String {
        Self::combine_css(&["auth", "tailwind"])
    }
}

#[macro_export]
macro_rules! include_styles {
    ($($style:expr),*) => {
        $crate::backend::utils::resource_loader::ResourceLoader::combine_css(&[$($style),*])
    };
}
