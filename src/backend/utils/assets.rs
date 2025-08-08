use std::collections::HashMap;
use std::sync::OnceLock;

static ASSET_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

pub struct AssetLoader;

impl AssetLoader {
    pub fn init() {
        let assets: [(&'static str, &'static str); 12] = [
            (
                "logo",
                concat!(
                    "data:image/png;base64,",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/assets/images/other/logo.png.base64"
                    ))
                ),
            ),
            (
                "home",
                concat!(
                    "data:image/png;base64,",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/assets/images/buttons/home.png.base64"
                    ))
                ),
            ),
            (
                "packs",
                concat!(
                    "data:image/png;base64,",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/assets/images/buttons/packs.png.base64"
                    ))
                ),
            ),
            (
                "settings",
                concat!(
                    "data:image/png;base64,",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/assets/images/buttons/settings.png.base64"
                    ))
                ),
            ),
            (
                "cloud",
                concat!(
                    "data:image/png;base64,",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/assets/images/buttons/cloud.png.base64"
                    ))
                ),
            ),
            (
                "plus",
                concat!(
                    "data:image/png;base64,",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/assets/images/buttons/plus.png.base64"
                    ))
                ),
            ),
            (
                "microsoft",
                concat!(
                    "data:image/png;base64,",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/assets/images/other/microsoft.png.base64"
                    ))
                ),
            ),
            (
                "play",
                concat!(
                    "data:image/png;base64,",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/assets/images/buttons/play.png.base64"
                    ))
                ),
            ),
            (
                "additional",
                concat!(
                    "data:image/png;base64,",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/assets/images/buttons/additional.png.base64"
                    ))
                ),
            ),
            (
                "change",
                concat!(
                    "data:image/png;base64,",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/assets/images/buttons/change.png.base64"
                    ))
                ),
            ),
            (
                "delete",
                concat!(
                    "data:image/png;base64,",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/assets/images/buttons/delete.png.base64"
                    ))
                ),
            ),
            (
                "folder",
                concat!(
                    "data:image/png;base64,",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/assets/images/buttons/folder.png.base64"
                    ))
                ),
            ),
        ];

        let cache: HashMap<_, _> = assets.into_iter().collect();

        if ASSET_CACHE.set(cache).is_err() {
            tracing::warn!("Asset cache was already initialized");
        }
    }

    #[inline(always)]
    pub fn get(asset_name: &str) -> &'static str {
        ASSET_CACHE
            .get()
            .and_then(|cache| cache.get(asset_name))
            .copied()
            .unwrap_or("data:image/png;base64,")
    }

    #[inline(always)]
    pub fn get_logo() -> &'static str {
        Self::get("logo")
    }
    #[inline(always)]
    pub fn get_home() -> &'static str {
        Self::get("home")
    }
    #[inline(always)]
    pub fn get_packs() -> &'static str {
        Self::get("packs")
    }
    #[inline(always)]
    pub fn get_settings() -> &'static str {
        Self::get("settings")
    }
    #[inline(always)]
    pub fn get_cloud() -> &'static str {
        Self::get("cloud")
    }
    #[inline(always)]
    pub fn get_plus() -> &'static str {
        Self::get("plus")
    }
    #[inline(always)]
    pub fn get_microsoft() -> &'static str {
        Self::get("microsoft")
    }
    #[inline(always)]
    pub fn get_play() -> &'static str {
        Self::get("play")
    }
    #[inline(always)]
    pub fn get_additional() -> &'static str {
        Self::get("additional")
    }
    #[inline(always)]
    pub fn get_change() -> &'static str {
        Self::get("change")
    }
    #[inline(always)]
    pub fn get_delete() -> &'static str {
        Self::get("delete")
    }
    #[inline(always)]
    pub fn get_folder() -> &'static str {
        Self::get("folder")
    }
}

pub fn ensure_assets_loaded() {
    if ASSET_CACHE.get().is_none() {
        AssetLoader::init();
    }
}
