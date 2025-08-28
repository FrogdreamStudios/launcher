//! Platform information for the launcher.

use std::collections::HashMap;

use crate::backend::utils::system::os::{
    get_all_native_classifiers, get_minecraft_arch, get_minecraft_os_name, get_os_features,
};

/// Platform information cached for downloads.
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub os_name: &'static str,
    pub os_arch: &'static str,
    pub os_features: HashMap<String, bool>,
    pub native_classifiers: Vec<String>,
}

impl PlatformInfo {
    pub fn new() -> Self {
        Self {
            os_name: get_minecraft_os_name(),
            os_arch: get_minecraft_arch(),
            os_features: get_os_features(),
            native_classifiers: get_all_native_classifiers(),
        }
    }
}

impl Default for PlatformInfo {
    fn default() -> Self {
        Self::new()
    }
}
