pub fn calculate_directory_size(path: &std::path::Path) -> anyhow::Result<u64, std::io::Error> {
    if path.is_file() {
        Ok(path.metadata()?.len())
    } else if path.is_dir() {
        std::fs::read_dir(path)?
            .map(|e| calculate_directory_size(&e?.path()))
            .sum()
    } else {
        Ok(0)
    }
}
