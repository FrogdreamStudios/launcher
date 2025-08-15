//! Asset and CSS loading/caching utilities.

use std::{collections::HashMap, sync::OnceLock};

static ASSET_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
static CSS_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

macro_rules! asset_entry {
    ($name:literal, $path:literal) => {
        (
            $name,
            concat!(
                "data:image/png;base64,",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/images/",
                    $path,
                    ".png.base64"
                ))
            ),
        )
    };
}

macro_rules! css_entry {
    ($name:literal, $path:literal) => {
        (
            $name,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/styles/",
                $path
            )),
        )
    };
}

pub struct ResourceLoader;

impl ResourceLoader {
    fn get_all_assets() -> HashMap<&'static str, &'static str> {
        [
            asset_entry!("logo", "other/logo"),
            asset_entry!("home", "buttons/home"),
            asset_entry!("packs", "buttons/packs"),
            asset_entry!("settings", "buttons/settings"),
            asset_entry!("cloud", "buttons/cloud"),
            asset_entry!("plus", "buttons/plus"),
            asset_entry!("microsoft", "other/microsoft"),
            asset_entry!("play", "buttons/play"),
            asset_entry!("additional", "buttons/additional"),
            asset_entry!("change", "buttons/change"),
            asset_entry!("delete", "buttons/delete"),
            asset_entry!("folder", "buttons/folder"),
            asset_entry!("debug", "buttons/debug"),
            asset_entry!("add", "buttons/add"),
        ]
        .into_iter()
        .collect()
    }

    pub fn get_asset(name: &str) -> &'static str {
        ASSET_CACHE
            .get_or_init(Self::get_all_assets)
            .get(name)
            .copied()
            .unwrap_or("data:image/png;base64,")
    }

    pub fn get_logo() -> &'static str {
        Self::get_asset("logo")
    }
    pub fn get_home() -> &'static str {
        Self::get_asset("home")
    }
    pub fn get_packs() -> &'static str {
        Self::get_asset("packs")
    }
    pub fn get_settings() -> &'static str {
        Self::get_asset("settings")
    }
    pub fn get_cloud() -> &'static str {
        Self::get_asset("cloud")
    }
    pub fn get_plus() -> &'static str {
        Self::get_asset("plus")
    }
    pub fn get_microsoft() -> &'static str {
        Self::get_asset("microsoft")
    }
    pub fn get_play() -> &'static str {
        Self::get_asset("play")
    }
    pub fn get_additional() -> &'static str {
        Self::get_asset("additional")
    }
    pub fn get_change() -> &'static str {
        Self::get_asset("change")
    }
    pub fn get_delete() -> &'static str {
        Self::get_asset("delete")
    }
    pub fn get_folder() -> &'static str {
        Self::get_asset("folder")
    }
    pub fn get_debug() -> &'static str {
        Self::get_asset("debug")
    }
    pub fn get_add() -> &'static str {
        Self::get_asset("add")
    }

    fn get_all_styles() -> HashMap<&'static str, &'static str> {
        [
            css_entry!("base", "base.css"),
            css_entry!("animations", "animations.css"),
            css_entry!("auth", "auth.css"),
            css_entry!("tailwind", "output.css"),
            css_entry!("logo", "components/logo.css"),
            css_entry!("navigation", "components/navigation.css"),
            css_entry!("chat", "components/chat.css"),
            css_entry!("home", "components/home.css"),
            css_entry!("news", "components/news.css"),
            css_entry!("context_menu", "components/context_menu.css"),
            css_entry!("debug", "components/debug.css"),
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
