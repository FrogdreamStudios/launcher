//! System directories utilities.

use std::path::PathBuf;

/// Get the user's home directory.
pub fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE")
            .or_else(|_| {
                let homedrive = std::env::var("HOMEDRIVE").unwrap_or_default();
                let homepath = std::env::var("HOMEPATH").unwrap_or_default();
                if homedrive.is_empty() || homepath.is_empty() {
                    Err(std::env::VarError::NotPresent)
                } else {
                    Ok(format!("{}{}", homedrive, homepath))
                }
            })
            .ok()
            .map(PathBuf::from)
    }

    #[cfg(unix)]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

/// Get the user's data directory (`AppData` on Windows, ~/.local/share on Unix).
pub fn data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    }

    #[cfg(target_os = "macos")]
    {
        home_dir().map(|home| home.join("Library").join("Application Support"))
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var("XDG_DATA_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join(".local").join("share")))
    }
}
