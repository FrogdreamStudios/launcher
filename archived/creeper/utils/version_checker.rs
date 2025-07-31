/// Parses a version string into a tuple (major, minor, patch).
pub fn parse_version_number(version: &str) -> Option<(u32, u32, u32)> {
    if version.contains('w')
        || version.contains("pre")
        || version.contains("rc") && version.starts_with("24w")
        || version.starts_with("25w")
    {
        return Some((1, 21, 0));
    }

    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
            let patch = parts
                .get(2)
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            return Some((major, minor, patch));
        }
    }

    None
}

/// Determines whether `--userProperties` should be passed based on the version.
pub fn needs_user_properties(version: &str) -> bool {
    if let Some((major, minor, _)) = parse_version_number(version) {
        if major == 1 && minor <= 8 {
            return true;
        }
    }

    if version.contains("w")
        || version.contains("pre")
        || version.contains("rc")
        || version.len() < 3
    {
        return true;
    }

    matches!(version, "1.6.4" | "1.7.2" | "1.7.10")
}

/// Determines whether legacy macOS arguments are needed.
pub fn needs_legacy_macos_args(version: &str) -> bool {
    if let Some((major, minor, _)) = parse_version_number(version) {
        if major == 1 && minor <= 12 {
            return true;
        }
    }

    if matches!(version, "1.12.2" | "1.12.1" | "1.12") {
        return true;
    }

    if version.contains("w") || version.len() < 3 || version.starts_with("1.") {
        if let Some((major, minor, _)) = parse_version_number(version) {
            if major == 1 && minor <= 12 {
                return true;
            }
        } else if version.starts_with("1.") {
            return true;
        }
    }

    false
}
