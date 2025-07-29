use std::path::PathBuf;

pub struct FileManager {}

impl FileManager {
    pub fn get_app_data_dir() -> Option<PathBuf> {
        dirs::data_dir()
    }
}
