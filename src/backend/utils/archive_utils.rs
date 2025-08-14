//! Archive extraction utilities.
//!
//! Extract different types of archives like ZIP and TAR.GZ files.
//! It can detect archive types by extension or by reading file headers.

use std::path::Path;

use crate::utils::Result;
use crate::{log_debug, log_info, simple_error};
use tokio::fs as async_fs;

use crate::backend::utils::file_utils::ensure_directory;

/// Extracts an archive based on its file extension.
///
/// Supports ZIP and TAR.GZ formats. If extension detection fails,
/// it tries to detect the format by reading file headers.
pub async fn extract_archive<P: AsRef<Path>>(archive_path: P, extract_path: P) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let extract_path = extract_path.as_ref();

    // Make sure the extraction directory exists
    ensure_directory(extract_path).await?;

    let filename = archive_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    log_info!("Extracting archive: {filename}");

    // Check for compound extensions first, then single extensions
    if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
        extract_tar_gz(archive_path, extract_path).await?;
    } else if filename.ends_with(".zip") {
        extract_zip(archive_path, extract_path).await?;
    } else {
        // Try to detect a format by reading file headers
        extract_archive_by_content(archive_path, extract_path).await?;
    }

    log_info!("Archive extracted successfully to {extract_path:?}");
    Ok(())
}

/// Extracts a ZIP archive to the specified directory.
pub async fn extract_zip<P: AsRef<Path>>(archive_path: P, extract_path: P) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let extract_path = extract_path.as_ref();

    // Use our custom archive extraction utility
    crate::utils::extract_zip(archive_path, extract_path).await?;
    log_info!("Successfully extracted ZIP archive: {:?}", archive_path);

    Ok(())
}

/// Extracts a TAR.GZ (compressed tar) archive.
pub async fn extract_tar_gz<P: AsRef<Path>>(archive_path: P, extract_path: P) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let extract_path = extract_path.as_ref();

    // Use our custom archive extraction utility
    crate::utils::extract_tar_gz(archive_path, extract_path).await?;
    log_info!("Successfully extracted TAR.GZ archive: {:?}", archive_path);

    Ok(())
}

/// Attempts to detect archive formats by reading file header.
///
/// This function reads the first few bytes of a file to identify
/// whether it's a ZIP or GZIP file based on magic numbers.
pub async fn extract_archive_by_content<P: AsRef<Path>>(
    archive_path: P,
    extract_path: P,
) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let extract_path = extract_path.as_ref();

    // Read the first 4 bytes to detect the file type
    let mut file = async_fs::File::open(archive_path).await?;
    let mut header = [0u8; 4];

    use tokio::io::AsyncReadExt;
    if file.read_exact(&mut header).await.is_err() {
        return Err(simple_error!("File too small to read header"));
    }

    // Check magic numbers (file signatures)
    if header[0] == 0x50 && header[1] == 0x4B && header[2] == 0x03 && header[3] == 0x04 {
        // ZIP file magic number: PK...
        log_debug!("Detected ZIP format");
        extract_zip(archive_path, extract_path).await?;
    } else if header[0] == 0x1F && header[1] == 0x8B {
        // GZIP magic number (probably tar.gz)
        log_debug!("Detected GZIP format");
        extract_tar_gz(archive_path, extract_path).await?;
    } else {
        // Unknown format or corrupted file
        log_debug!(
            "Unknown format. Header: {:02X} {:02X} {:02X} {:02X}",
            header[0],
            header[1],
            header[2],
            header[3]
        );
        return Err(simple_error!(
            "Unrecognized archive format. Header: {:02X} {:02X} {:02X} {:02X}",
            header[0],
            header[1],
            header[2],
            header[3]
        ));
    }

    Ok(())
}
