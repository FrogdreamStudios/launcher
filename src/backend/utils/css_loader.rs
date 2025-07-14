use std::collections::HashMap;
use std::sync::OnceLock;

static CSS_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

pub struct CssLoader;

impl CssLoader {
    pub fn init() {
        let mut cache = HashMap::new();

        cache.insert(
            "main",
            include_str!("../../../public/assets/styles/main.css"),
        );
        cache.insert(
            "auth",
            include_str!("../../../public/assets/styles/auth.css"),
        );
        cache.insert(
            "chat",
            include_str!("../../../public/assets/styles/chat.css"),
        );

        CSS_CACHE.set(cache).expect("CSS cache already initialized");
    }

    pub fn get(style_name: &str) -> Option<&'static str> {
        CSS_CACHE.get()?.get(style_name).copied()
    }

    pub fn get_main() -> &'static str {
        Self::get("main").unwrap_or("")
    }

    pub fn get_auth() -> &'static str {
        Self::get("auth").unwrap_or("")
    }

    pub fn get_chat() -> &'static str {
        Self::get("chat").unwrap_or("")
    }

    #[allow(dead_code)]
    pub fn load_styles(styles: &[&str]) -> String {
        styles
            .iter()
            .filter_map(|&style| Self::get(style))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[macro_export]
macro_rules! include_styles {
    ($($style:expr),*) => {
        {
            use $crate::utils::css_loader::CssLoader;
            CssLoader::load_styles(&[$($style),*])
        }
    };
}

pub fn ensure_css_loaded() {
    if CSS_CACHE.get().is_none() {
        CssLoader::init();
    }
}
