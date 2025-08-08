use std::collections::HashMap;
use std::sync::OnceLock;

static CSS_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

pub struct CssLoader;

impl CssLoader {
    pub fn init() {
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
        let _ = CSS_CACHE.set(cache);
    }

    #[inline(always)]
    pub fn get(style_name: &str) -> Option<&'static str> {
        CSS_CACHE.get()?.get(style_name).copied()
    }
    #[inline(always)]
    pub fn get_chat() -> &'static str {
        Self::get("chat").unwrap_or("")
    }

    /// Combines multiple styles into a single string
    pub fn combine(styles: &[&str]) -> String {
        styles
            .iter()
            .map(|&name| Self::get(name).unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n")
    }

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

    pub fn get_combined_auth() -> String {
        Self::combine(&["auth", "tailwind"])
    }
}

#[macro_export]
macro_rules! include_styles {
    ($($style:expr),*) => {
        {
            $crate::utils::css_loader::CssLoader::load_styles(&[$($style),*])
        }
    };
}

pub fn ensure_css_loaded() {
    if CSS_CACHE.get().is_none() {
        CssLoader::init();
    }
}
