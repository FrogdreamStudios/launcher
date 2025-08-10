//! CSS loading and caching utilities.
//!
//! Load and cache CSS stylesheets that are embedded at compile time.
//! Styles can be loaded individually or combined for different parts of the application.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Global cache for storing loaded CSS styles.
static CSS_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

/// CSS loader that manages embedded stylesheets.
///
/// All CSS files are embedded at compile time and cached for fast access.
pub struct CssLoader;

impl CssLoader {
    /// Initializes the CSS cache with all embedded stylesheets.
    ///
    /// This loads all CSS files into memory for fast access.
    /// Should be called once at application startup.
    pub fn init() {
        // Array of all embedded CSS files with their names and content
        let styles: [(&'static str, &'static str); 10] = [
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
                "auth",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/auth.css"
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
                "tailwind",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/output.css"
                )),
            ),
        ];

        let cache: HashMap<_, _> = styles.into_iter().collect();
        // Initialize the global cache (only works once)
        let _ = CSS_CACHE.set(cache);
    }

    /// Gets a CSS style by name.
    #[inline(always)]
    pub fn get(style_name: &str) -> Option<&'static str> {
        CSS_CACHE.get()?.get(style_name).copied()
    }
    /// Gets the chat component CSS styles.
    #[inline(always)]
    pub fn get_chat() -> &'static str {
        Self::get("chat").unwrap_or("")
    }

    /// Combines multiple styles into a single string.
    pub fn combine(styles: &[&str]) -> String {
        styles
            .iter()
            .map(|&name| Self::get(name).unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Gets all main application styles combined into one string.
    ///
    /// Includes base styles, animations, components, and Tailwind CSS.
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
            "tailwind",
        ])
    }

    /// Gets authentication page styles combined into one string.
    ///
    /// Includes auth-specific styles and Tailwind CSS.
    pub fn get_combined_auth() -> String {
        Self::combine(&["auth", "tailwind"])
    }
}

/// Macro to include multiple styles at compile time.
///
/// This macro provides a convenient way to load multiple CSS styles
/// in a single call.
#[macro_export]
macro_rules! include_styles {
    ($($style:expr),*) => {
        {
            $crate::utils::css_loader::CssLoader::load_styles(&[$($style),*])
        }
    };
}

/// Ensures that CSS styles are loaded into a cache.
///
/// This function checks if styles are already loaded and initializes
/// them if they haven't been loaded yet. Safe to call multiple times.
pub fn ensure_css_loaded() {
    if CSS_CACHE.get().is_none() {
        CssLoader::init();
    }
}
