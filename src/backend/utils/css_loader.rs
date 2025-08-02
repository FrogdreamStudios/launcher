use std::collections::HashMap;
use std::sync::OnceLock;

static CSS_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

pub struct CssLoader;

impl CssLoader {
    pub fn init() {
        let mut cache = HashMap::new();

        cache.insert(
            "main",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/styles/main.css"
            )),
        );
        cache.insert(
            "auth",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/styles/auth.css"
            )),
        );
        cache.insert(
            "chat",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/styles/chat.css"
            )),
        );
        cache.insert(
            "tailwind",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/styles/output.css"
            )),
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

    pub fn get_tailwind() -> &'static str {
        Self::get("tailwind").unwrap_or("")
    }

    pub fn get_combined_main() -> String {
        format!("{}\n{}", Self::get_main(), Self::get_tailwind())
    }

    pub fn get_combined_auth() -> String {
        format!("{}\n{}", Self::get_auth(), Self::get_tailwind())
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
