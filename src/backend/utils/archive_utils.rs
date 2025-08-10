use anyhow::Result;
use std::path::Path;
use tokio::fs as async_fs;
use tracing::{debug, info};

use crate::backend::utils::file_utils::{ensure_directory, ensure_parent_directory};

/// Extracts an archive based on its file extension
pub async fn extract_archive<P: AsRef<Path>>(archive_path: P, extract_path: P) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let extract_path = extract_path.as_ref();

    ensure_directory(extract_path).await?;

    let filename = archive_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    info!("Extracting archive: {filename}");

    // Check for compound extensions first, then single extensions
    if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
        extract_tar_gz(archive_path, extract_path).await?;
    } else if filename.ends_with(".zip") {
        extract_zip(archive_path, extract_path).await?;
    } else {
        // Try to detect format by content
        extract_archive_by_content(archive_path, extract_path).await?;
    }

    info!("Archive extracted successfully to {extract_path:?}");
    Ok(())
}

/// Extracts a ZIP archive
pub async fn extract_zip<P: AsRef<Path>>(archive_path: P, extract_path: P) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let extract_path = extract_path.as_ref();

    use std::io::Read;

    let file = std::fs::File::open(archive_path)
        .map_err(|e| anyhow::anyhow!("Failed to open ZIP archive {archive_path:?}: {e}"))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| anyhow::anyhow!("Failed to read ZIP archive {archive_path:?}: {e}"))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| anyhow::anyhow!("Failed to read ZIP entry {i}: {e}"))?;

        let outpath = extract_path.join(file.mangled_name());

        if file.name().ends_with('/') {
            ensure_directory(&outpath).await?;
        } else {
            ensure_parent_directory(&outpath).await?;

            let mut outfile = async_fs::File::create(&outpath).await?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;

            use tokio::io::AsyncWriteExt;
            outfile.write_all(&buffer).await?;
            outfile.flush().await?;
        }

        debug!("Extracted: {outpath:?}");
    }

    Ok(())
}

/// Extracts a TAR.GZ archive
pub async fn extract_tar_gz<P: AsRef<Path>>(archive_path: P, extract_path: P) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let extract_path = extract_path.as_ref();

    use flate2::read::GzDecoder;
    use tar::Archive;

    let tar_gz = std::fs::File::open(archive_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let target_path = extract_path.join(&*path);

        ensure_parent_directory(&target_path).await?;
        entry.unpack(&target_path)?;
        debug!("Extracted: {target_path:?}");
    }

    Ok(())
}

/// Attempts to detect archive format by reading file header
pub async fn extract_archive_by_content<P: AsRef<Path>>(
    archive_path: P,
    extract_path: P,
) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let extract_path = extract_path.as_ref();

    // Read first few bytes to detect file type
    let mut file = async_fs::File::open(archive_path).await?;
    let mut header = [0u8; 4];

    use tokio::io::AsyncReadExt;
    if file.read_exact(&mut header).await.is_err() {
        return Err(anyhow::anyhow!("File too small to read header"));
    }

    // Check file signatures
    if header[0] == 0x50 && header[1] == 0x4B && header[2] == 0x03 && header[3] == 0x04 {
        // ZIP file signature
        debug!("Detected ZIP format");
        extract_zip(archive_path, extract_path).await?;
    } else if header[0] == 0x1F && header[1] == 0x8B {
        // GZIP signature (likely tar.gz)
        debug!("Detected GZIP format");
        extract_tar_gz(archive_path, extract_path).await?;
    } else {
        // Could be a different format or corrupted
        debug!(
            "Unknown format - header: {:02X} {:02X} {:02X} {:02X}",
            header[0], header[1], header[2], header[3]
        );
        return Err(anyhow::anyhow!(
            "Unrecognized archive format. Header: {:02X} {:02X} {:02X} {:02X}",
            header[0],
            header[1],
            header[2],
            header[3]
        ));
    }

    Ok(())
}
