use std::collections::HashMap;

/// Get the current operating system name for Minecraft rules.
pub fn get_minecraft_os_name() -> &'static str {
    match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "osx",
        "linux" => "linux",
        _ => "linux",
    }
}

/// Get the current architecture for Minecraft rules.
pub fn get_minecraft_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        "x86" => "x86",
        _ => "x86_64",
    }
}

/// Get OS-specific features for rule evaluation.
pub fn get_os_features() -> HashMap<String, bool> {
    let mut features = HashMap::new();

    // Add common features
    features.insert("is_demo_user".to_string(), false);
    features.insert("has_custom_resolution".to_string(), false);

    // OS-specific features
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

/// Get all possible native classifiers for the current platform (for fallback)..
pub fn get_all_native_classifiers() -> Vec<String> {
    match std::env::consts::OS {
        "windows" => match std::env::consts::ARCH {
            "x86_64" => vec!["natives-windows".to_string()],
            "x86" => vec![
                "natives-windows-x86".to_string(),
                "natives-windows".to_string(),
            ],
            _ => vec!["natives-windows".to_string()],
        },
        "macos" => match std::env::consts::ARCH {
            "aarch64" => vec!["natives-osx-arm64".to_string(), "natives-osx".to_string()],
            _ => vec!["natives-osx".to_string()],
        },
        "linux" => match std::env::consts::ARCH {
            "aarch64" => vec![
                "natives-linux-arm64".to_string(),
                "natives-linux".to_string(),
            ],
            "x86" => vec!["natives-linux-x86".to_string(), "natives-linux".to_string()],
            _ => vec!["natives-linux".to_string()],
        },
        _ => vec!["natives-linux".to_string()],
    }
}
