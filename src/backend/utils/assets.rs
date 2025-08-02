use std::collections::HashMap;
use std::sync::OnceLock;

static ASSET_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

pub struct AssetLoader;

impl AssetLoader {
    pub fn init() {
        let mut cache = HashMap::new();

        cache.insert(
            "logo",
            concat!(
                "data:image/png;base64,",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/images/other/logo.png.base64"
                ))
            ),
        );

        cache.insert(
            "home",
            concat!(
                "data:image/png;base64,",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/images/buttons/home.png.base64"
                ))
            ),
        );

        cache.insert(
            "packs",
            concat!(
                "data:image/png;base64,",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/images/buttons/packs.png.base64"
                ))
            ),
        );

        cache.insert(
            "settings",
            concat!(
                "data:image/png;base64,",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/images/buttons/settings.png.base64"
                ))
            ),
        );

        cache.insert(
            "cloud",
            concat!(
                "data:image/png;base64,",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/images/buttons/cloud.png.base64"
                ))
            ),
        );

        cache.insert(
            "plus",
            concat!(
                "data:image/png;base64,",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/images/buttons/plus.png.base64"
                ))
            ),
        );

        cache.insert(
            "microsoft",
            concat!(
                "data:image/png;base64,",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/images/other/microsoft.png.base64"
                ))
            ),
        );

        ASSET_CACHE
            .set(cache)
            .expect("Asset cache already initialized");
    }

    pub fn get(asset_name: &str) -> &'static str {
        ASSET_CACHE
            .get()
            .and_then(|cache| cache.get(asset_name))
            .copied()
            .unwrap_or("data:image/png;base64,")
    }

    pub fn get_logo() -> &'static str {
        Self::get("logo")
    }

    pub fn get_home() -> &'static str {
        Self::get("home")
    }

    pub fn get_packs() -> &'static str {
        Self::get("packs")
    }

    pub fn get_settings() -> &'static str {
        Self::get("settings")
    }

    pub fn get_cloud() -> &'static str {
        Self::get("cloud")
    }

    pub fn get_plus() -> &'static str {
        Self::get("plus")
    }

    pub fn get_microsoft() -> &'static str {
        Self::get("microsoft")
    }
}

pub fn ensure_assets_loaded() {
    if ASSET_CACHE.get().is_none() {
        AssetLoader::init();
    }
}
