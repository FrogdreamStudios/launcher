//! CSS loading and caching utilities.
//!
//! Load and cache CSS stylesheets that are embedded at compile time.
//! Styles can be loaded individually or combined for different parts of the application.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Global cache for storing loaded CSS styles.
static CSS_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

/// Macro to generate CSS entry with embedded content.
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

/// Macro to generate CSS getter methods.
macro_rules! css_getter {
    ($fn_name:ident, $style_name:literal, $doc:literal) => {
        #[doc = $doc]
        #[inline(always)]
        pub fn $fn_name() -> &'static str {
            Self::get($style_name).unwrap_or("")
        }
    };
}

/// CSS loader that manages embedded stylesheets.
///
/// All CSS files are embedded at compile time and cached for fast access.
pub struct CssLoader;

impl CssLoader {
    /// Initializes the CSS cache with all embedded stylesheets.
    pub fn init() {
        let styles = [
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
        ];

        let _ = CSS_CACHE.set(styles.into_iter().collect());
    }

    /// Returns a CSS style by name.
    #[inline(always)]
    pub fn get(style_name: &str) -> Option<&'static str> {
        CSS_CACHE.get()?.get(style_name).copied()
    }

    /// Combines multiple styles into a single string.
    pub fn combine(styles: &[&str]) -> String {
        styles
            .iter()
            .filter_map(|&name| Self::get(name))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Gets all main application styles combined into one string.
    pub fn get_combined_main() -> String {
        Self::combine(&[
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

    /// Gets authentication page styles combined into one string.
    pub fn get_combined_auth() -> String {
        Self::combine(&["auth", "tailwind"])
    }

    css_getter!(get_chat, "chat", "Returns chat component CSS.");
}

/// Macro to include multiple styles at compile time.
#[macro_export]
macro_rules! include_styles {
    ($($style:expr),*) => {
        $crate::backend::utils::css_loader::CssLoader::combine(&[$($style),*])
    };
}

/// Ensures that CSS styles are loaded into a cache.
pub fn ensure_css_loaded() {
    if CSS_CACHE.get().is_none() {
        CssLoader::init();
    }
}
