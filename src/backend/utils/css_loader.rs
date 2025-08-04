use std::collections::HashMap;
use std::sync::OnceLock;

static CSS_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

pub struct CssLoader;

impl CssLoader {
    pub fn init() {
        let styles: [(&'static str, &'static str); 4] = [
            (
                "main",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/main.css"
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
                "chat",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/styles/chat.css"
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
        CSS_CACHE.set(cache).expect("CSS cache already initialized");
    }

    #[inline(always)]
    pub fn get(style_name: &str) -> Option<&'static str> {
        CSS_CACHE.get()?.get(style_name).copied()
    }

    #[inline(always)]
    pub fn get_main() -> &'static str {
        Self::get("main").unwrap_or("")
    }
    #[inline(always)]
    pub fn get_auth() -> &'static str {
        Self::get("auth").unwrap_or("")
    }
    #[inline(always)]
    pub fn get_chat() -> &'static str {
        Self::get("chat").unwrap_or("")
    }
    #[inline(always)]
    pub fn get_tailwind() -> &'static str {
        Self::get("tailwind").unwrap_or("")
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
        Self::combine(&["main", "tailwind"])
    }

    pub fn get_combined_auth() -> String {
        Self::combine(&["auth", "tailwind"])
    }

    /// Returns a combined string of all styles
    pub fn load_styles(styles: &[&str]) -> String {
        Self::combine(styles)
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
