//! Executable finder.

use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum WhichError {
    NotFound,
    IoError(std::io::Error),
}

impl std::fmt::Display for WhichError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Executable not found"),
            Self::IoError(e) => write!(f, "IO main: {e}"),
        }
    }
}

impl std::error::Error for WhichError {}

impl From<std::io::Error> for WhichError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error)
    }
}

pub fn which<P: AsRef<Path>>(binary_name: P) -> Result<PathBuf, WhichError> {
    let binary_name = binary_name.as_ref();

    let path_env = std::env::var("PATH").map_err(|_| WhichError::NotFound)?;
    let separator = if cfg!(windows) { ';' } else { ':' };
    let _extension = if cfg!(windows) { ".exe" } else { "" };

    for dir in path_env.split(separator) {
        let mut candidate = PathBuf::from(dir).join(binary_name);

        if cfg!(windows) && !binary_name.to_string_lossy().ends_with(".exe") {
            candidate.set_extension("exe");
        }

        if candidate.exists() && is_executable(&candidate)? {
            return Ok(candidate);
        }
    }

    Err(WhichError::NotFound)
}

fn is_executable(path: &Path) -> Result<bool, std::io::Error> {
    let metadata = std::fs::metadata(path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Ok(metadata.is_file() && (metadata.permissions().mode() & 0o111) != 0)
    }

    #[cfg(windows)]
    {
        Ok(metadata.is_file())
    }
}
