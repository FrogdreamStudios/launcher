//! Operating system utilities.
//!
//! Detection of the current OS and architecture for proper Minecraft rule evaluation and
//! native library selection.

use std::collections::HashMap;

/// Get the current operating system name for Minecraft rules.
pub fn get_minecraft_os_name() -> &'static str {
    match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "osx", // Minecraft uses "osx" instead of "macos" (but why?)
        "linux" => "linux",
        _ => "linux", // Default to linux for unknown systems
    }
}

/// Get the current architecture for Minecraft rules.
pub fn get_minecraft_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "arm64", // Minecraft uses "arm64" instead of "aarch64"
        "x86" => "x86",
        _ => "x86_64", // Default to x86_64 for unknown architectures
    }
}

/// Get OS-specific features for rule evaluation.
pub fn get_os_features() -> HashMap<String, bool> {
    let mut features = HashMap::new();

    // Add common features that apply to all platforms
    features.insert("is_demo_user".to_string(), false);
    features.insert("has_custom_resolution".to_string(), false);

    // Set OS-specific features
    match std::env::consts::OS {
        "windows" => {
            features.insert("has_quickplay_support".to_string(), true);
        }
        "macos" => {
            features.insert("has_quickplay_support".to_string(), true);
        }
        _ => {
            features.insert("has_quickplay_support".to_string(), false);
        }
    }

    features
}

/// Get all possible native classifiers for the current platform.
pub fn get_all_native_classifiers() -> Vec<String> {
    match std::env::consts::OS {
        "windows" => match std::env::consts::ARCH {
            "x86_64" => vec!["natives-windows".to_string()],
            "x86" => vec![
                "natives-windows-x86".to_string(), // Try specific first
                "natives-windows".to_string(),     // Fall back to generic
            ],
            _ => vec!["natives-windows".to_string()],
        },
        "macos" => match std::env::consts::ARCH {
            "aarch64" => vec![
                "natives-osx-arm64".to_string(), // Apple Silicon first
                "natives-osx".to_string(),       // Intel fallback
            ],
            _ => vec!["natives-osx".to_string()],
        },
        "linux" => match std::env::consts::ARCH {
            "aarch64" => vec![
                "natives-linux-arm64".to_string(), // ARM64 first
                "natives-linux".to_string(),       // x86_64 fallback
            ],
            "x86" => vec![
                "natives-linux-x86".to_string(), // 32-bit first
                "natives-linux".to_string(),     // 64-bit fallback
            ],
            _ => vec!["natives-linux".to_string()],
        },
        _ => vec!["natives-linux".to_string()], // Default to Linux
    }
}
