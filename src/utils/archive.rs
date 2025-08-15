//! Minimal archive extraction using system commands.

use crate::utils::{Error, Result};
use std::path::Path;
use tokio::process::Command;

/// Extract a ZIP archive using the system unzip command.
pub async fn extract_zip<P: AsRef<Path>>(archive_path: P, extract_path: P) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let extract_path = extract_path.as_ref();

    tokio::fs::create_dir_all(extract_path).await?;

    let success = if cfg!(target_os = "windows") {
        // Try PowerShell first, then 7z
        run_command(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    archive_path.display(),
                    extract_path.display()
                ),
            ],
        )
        .await
        .is_ok()
            || run_command(
                "7z",
                &[
                    "x",
                    &archive_path.to_string_lossy(),
                    &format!("-o{}", extract_path.display()),
                    "-y",
                ],
            )
            .await
            .is_ok()
    } else {
        run_command(
            "unzip",
            &[
                "-q",
                "-o",
                &archive_path.to_string_lossy(),
                "-d",
                &extract_path.to_string_lossy(),
            ],
        )
        .await
        .is_ok()
    };

    if success {
        Ok(())
    } else {
        Err(Error::new(format!(
            "Failed to extract ZIP: {}",
            archive_path.display()
        )))
    }
}

/// Extract a TAR.GZ archive using the system tar command.
pub async fn extract_tar_gz<P: AsRef<Path>>(archive_path: P, extract_path: P) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let extract_path = extract_path.as_ref();

    tokio::fs::create_dir_all(extract_path).await?;

    let success = run_command(
        "tar",
        &[
            "-xzf",
            &archive_path.to_string_lossy(),
            "-C",
            &extract_path.to_string_lossy(),
        ],
    )
    .await
    .is_ok()
        || (cfg!(target_os = "windows")
            && run_command(
                "7z",
                &[
                    "x",
                    &archive_path.to_string_lossy(),
                    &format!("-o{}", extract_path.display()),
                    "-y",
                ],
            )
            .await
            .is_ok());

    if success {
        Ok(())
    } else {
        Err(Error::new(format!(
            "Failed to extract TAR.GZ: {}",
            archive_path.display()
        )))
    }
}

async fn run_command(cmd: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(cmd).args(args).output().await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::new(format!("Command {} failed", cmd)))
    }
}
