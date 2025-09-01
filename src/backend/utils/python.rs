//! Python file embedding.

use std::process::Command;
use std::io::Write;
use tempfile::NamedTempFile;
use anyhow::Result;
use log::info;

/// Macro to embed Python files at compile time.
macro_rules! embed_python {
    ($name:expr, $path:expr) => {
        ($name, include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)))
    };
}

/// Embedded Python files as strings.
static EMBEDDED_PYTHON_FILES: &[(&str, &str)] = &[
    embed_python!("launcher.py", "python/launcher.py"),
    embed_python!("requirements.txt", "python/requirements.txt"),
];

/// Get embedded Python file content as string.
pub fn get_embedded_python_file(filename: &str) -> Option<&'static str> {
    EMBEDDED_PYTHON_FILES
        .iter()
        .find(|(name, _)| *name == filename)
        .map(|(_, content)| *content)
}

/// Check if Python is available in the system.
pub fn check_python_availability() -> Result<String> {
    let python_commands = if cfg!(target_os = "windows") {
        vec!["python", "python3", "py"]
    } else {
        vec!["python3", "python"]
    };
    
    for cmd in python_commands {
        if let Ok(output) = Command::new(cmd)
            .arg("--version")
            .output()
        {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                info!("Found Python: {cmd} - {version}");
                return Ok(cmd.to_string());
            }
        }
    }
    
    Err(anyhow::anyhow!("Python not found in system PATH. Please install Python from https://www.python.org/ or Microsoft Store"))
}

/// Install Python dependencies from embedded requirements.txt.
pub fn install_python_dependencies() -> Result<()> {
    let python_cmd = check_python_availability()?;
    
    let requirements_content = get_embedded_python_file("requirements.txt")
        .ok_or_else(|| anyhow::anyhow!("requirements.txt not found in embedded files"))?;
    
    // Create the temporary requirements.txt file
    let mut temp_requirements = NamedTempFile::new()?;
    temp_requirements.write_all(requirements_content.as_bytes())?;
    temp_requirements.flush()?;
    
    info!("Installing Python dependencies using: {python_cmd}");
    
    let output = Command::new(&python_cmd)
        .args(["-m", "pip", "install", "-r", temp_requirements.path().to_str().unwrap()])
        .output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to install Python dependencies: {error}"));
    }
    
    info!("Python dependencies installed successfully");
    Ok(())
}

/// Create a temporary file with the launcher script content.
pub fn get_launcher_script_path() -> Result<NamedTempFile> {
    let launcher_content = get_embedded_python_file("launcher.py")
        .ok_or_else(|| anyhow::anyhow!("launcher.py not found in embedded files"))?;
    
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(launcher_content.as_bytes())?;
    temp_file.flush()?;
    
    info!("Created temporary launcher script: {:?}", temp_file.path());
    Ok(temp_file)
}
