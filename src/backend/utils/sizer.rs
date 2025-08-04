pub fn calculate_directory_size(path: &std::path::Path) -> Result<u64, std::io::Error> {
    if path.is_file() {
        return Ok(path.metadata()?.len());
    }
    if path.is_dir() {
        return std::fs::read_dir(path)?
            .map(|entry| calculate_directory_size(&entry?.path()))
            .try_fold(0, |acc, size| Ok(acc + size?));
    }
    Ok(0)
}
