//! Asset loading and caching utilities.
//!
//! Load and cache base64-encoded images that are embedded at compile time.
//! All assets are loaded at once and cached for fast access throughout the application.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Global cache for storing loaded assets as base64 data URLs.
static ASSET_CACHE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

/// Macro to generate asset entries with base64 data.
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

/// Macro to generate getter methods for assets.
macro_rules! asset_getter {
    ($fn_name:ident, $asset_name:literal, $doc:literal) => {
        #[doc = $doc]
        #[inline(always)]
        pub fn $fn_name() -> &'static str {
            Self::get($asset_name)
        }
    };
}

/// Asset loader that manages embedded image assets.
///
/// All images are converted to base64 at compile time and embedded
/// into the binary for fast loading without file system access.
pub struct AssetLoader;

impl AssetLoader {
    /// Initializes the asset cache with all embedded images.
    pub fn init() {
        let assets = [
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
        ];

        if ASSET_CACHE.set(assets.into_iter().collect()).is_err() {
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
            .unwrap_or("data:image/png;base64,")
    }

    asset_getter!(get_logo, "logo", "Gets the launcher's logo image.");
    asset_getter!(get_home, "home", "Gets the home button icon.");
    asset_getter!(get_packs, "packs", "Gets the mod packs button icon.");
    asset_getter!(get_settings, "settings", "Gets the settings button icon.");
    asset_getter!(get_cloud, "cloud", "Gets the cloud storage button icon.");
    asset_getter!(get_plus, "plus", "Gets the plus button icon.");
    asset_getter!(get_microsoft, "microsoft", "Gets the Microsoft login icon.");
    asset_getter!(get_play, "play", "Gets the play button icon.");
    asset_getter!(
        get_additional,
        "additional",
        "Gets the additional options button icon."
    );
    asset_getter!(get_change, "change", "Gets the edit button icon.");
    asset_getter!(get_delete, "delete", "Gets the delete button icon.");
    asset_getter!(get_folder, "folder", "Gets the folder icon.");
    asset_getter!(get_debug, "debug", "Gets the debug icon.");
    asset_getter!(get_add, "add", "Gets the add icon.");
}

/// Ensures that assets are loaded into a cache.
///
/// Checks if assets are already loaded and initializes
/// them if they haven't been loaded yet.
pub fn ensure_assets_loaded() {
    if ASSET_CACHE.get().is_none() {
        AssetLoader::init();
    }
}
