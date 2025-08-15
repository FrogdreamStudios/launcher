//! Operating system utilities.

use std::collections::HashMap;

/// Get the current operating system name for Minecraft rules.
pub fn get_minecraft_os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "osx",
        "windows" | "linux" => std::env::consts::OS,
        _ => "linux",
    }
}

/// Get the current architecture for Minecraft rules.
pub fn get_minecraft_arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" | "x86" => std::env::consts::ARCH,
        _ => "x86_64",
    }
}

/// Get OS-specific features for rule evaluation.
pub fn get_os_features() -> HashMap<String, bool> {
    let mut features = HashMap::from([
        ("is_demo_user".to_string(), false),
        ("has_custom_resolution".to_string(), false),
        (
            "has_quickplay_support".to_string(),
            matches!(std::env::consts::OS, "windows" | "macos"),
        ),
    ]);
    features
}

/// Get all possible native classifiers for the current platform.
pub fn get_all_native_classifiers() -> Vec<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("windows", "x86") => vec!["natives-windows-x86".into(), "natives-windows".into()],
        ("windows", _) => vec!["natives-windows".into()],
        ("macos", "aarch64") => vec!["natives-osx-arm64".into(), "natives-osx".into()],
        ("macos", _) => vec!["natives-osx".into()],
        ("linux", "aarch64") => vec!["natives-linux-arm64".into(), "natives-linux".into()],
        ("linux", "x86") => vec!["natives-linux-x86".into(), "natives-linux".into()],
        ("linux", _) => vec!["natives-linux".into()],
        _ => vec!["natives-linux".into()],
    }
}
