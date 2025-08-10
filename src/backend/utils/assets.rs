//! Asset loading and caching utilities.
//!
//! Load and cache base64-encoded images that are embedded at compile time.
//! All assets are loaded at once and cached for fast access throughout the application.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Global cache for storing loaded assets as base64 data URLs.
static ASSET_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

/// Asset loader that manages embedded image assets.
///
/// All images are converted to base64 at compile time and embedded
/// into the binary for fast loading without file system access.
pub struct AssetLoader;

impl AssetLoader {
    /// Initializes the asset cache with all embedded images.
    pub fn init() {
        // Array of all embedded assets with their names and base64 data
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

        // Initialize the global cache (only works once)
        if ASSET_CACHE.set(cache).is_err() {
            tracing::warn!("Asset cache was already initialized");
        }
    }

    /// Gets an asset by name as a base64 data URL.
    #[inline(always)]
    pub fn get(asset_name: &str) -> &'static str {
        ASSET_CACHE
            .get()
            .and_then(|cache| cache.get(asset_name))
            .copied()
            .unwrap_or("data:image/png;base64,") // Return empty data URL if not found
    }

    /// Gets the application logo image.
    #[inline(always)]
    pub fn get_logo() -> &'static str {
        Self::get("logo")
    }
    /// Gets the home button icon.
    #[inline(always)]
    pub fn get_home() -> &'static str {
        Self::get("home")
    }
    /// Gets the mod packs button icon.
    #[inline(always)]
    pub fn get_packs() -> &'static str {
        Self::get("packs")
    }
    /// Gets the settings button icon.
    #[inline(always)]
    pub fn get_settings() -> &'static str {
        Self::get("settings")
    }
    /// Gets the cloud storage button icon.
    #[inline(always)]
    pub fn get_cloud() -> &'static str {
        Self::get("cloud")
    }
    /// Gets the plus/add button icon.
    #[inline(always)]
    pub fn get_plus() -> &'static str {
        Self::get("plus")
    }
    /// Gets the Microsoft login icon.
    #[inline(always)]
    pub fn get_microsoft() -> &'static str {
        Self::get("microsoft")
    }
    /// Gets the play button icon.
    #[inline(always)]
    pub fn get_play() -> &'static str {
        Self::get("play")
    }
    /// Gets the additional options button icon.
    #[inline(always)]
    pub fn get_additional() -> &'static str {
        Self::get("additional")
    }
    /// Gets the change/edit button icon.
    #[inline(always)]
    pub fn get_change() -> &'static str {
        Self::get("change")
    }
    /// Gets the delete button icon.
    #[inline(always)]
    pub fn get_delete() -> &'static str {
        Self::get("delete")
    }
    /// Gets the folder icon.
    #[inline(always)]
    pub fn get_folder() -> &'static str {
        Self::get("folder")
    }
}

/// Ensures that assets are loaded into a cache.
///
/// Checks if assets are already loaded and initializes
/// them if they haven't been loaded yet.
///
/// By the way, it's safe to call it multiple times.
pub fn ensure_assets_loaded() {
    if ASSET_CACHE.get().is_none() {
        AssetLoader::init();
    }
}
