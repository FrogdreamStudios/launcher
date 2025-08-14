//! Windows resource utilities.

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Interface for configuring Windows resources
/// such as application icons and version information that will be embedded
/// into the final executable.
pub struct WindowsResource {
    icon: Option<String>,
    fields: Vec<(String, String)>,
}

impl WindowsResource {
    /// Create a new Windows resource builder.
    pub fn new() -> Self {
        Self {
            icon: None,
            fields: Vec::new(),
        }
    }

    /// Set the application icon from an .ico file.
    pub fn set_icon(&mut self, path: &str) -> &mut Self {
        self.icon = Some(path.to_string());
        self
    }

    /// Set a version information field.
    pub fn set(&mut self, key: &str, value: &str) -> &mut Self {
        self.fields.push((key.to_string(), value.to_string()));
        self
    }

    /// Compile the Windows resources and link them into the executable.
    pub fn compile(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(not(target_os = "windows"))]
        return Ok(());

        #[cfg(target_os = "windows")]
        {
            let out_dir = env::var("OUT_DIR")?;
            let rc_path = format!("{}/app.rc", out_dir);

            // Generate RC content
            let mut content = String::new();

            if let Some(ref icon) = self.icon {
                content.push_str(&format!("1 ICON \"{}\"\n", icon));
            }

            if !self.fields.is_empty() {
                let version = self
                    .fields
                    .iter()
                    .find(|(k, _)| k == "ProductVersion")
                    .map(|(_, v)| v.as_str())
                    .unwrap_or("1.0.0");

                let parts: Vec<&str> = version.split('.').collect();
                let v1 = parts.get(0).unwrap_or(&"1");
                let v2 = parts.get(1).unwrap_or(&"0");
                let v3 = parts.get(2).unwrap_or(&"0");

                content.push_str(&format!(
                    "1 VERSIONINFO\nFILEVERSION {v1},{v2},{v3},0\nPRODUCTVERSION {v1},{v2},{v3},0\n"
                ));
                content.push_str("BEGIN\n  BLOCK \"StringFileInfo\"\n  BEGIN\n    BLOCK \"040904b0\"\n    BEGIN\n");

                for (key, value) in &self.fields {
                    content.push_str(&format!("      VALUE \"{key}\", \"{value}\"\n"));
                }

                content.push_str("    END\n  END\n  BLOCK \"VarFileInfo\"\n  BEGIN\n    VALUE \"Translation\", 0x409, 1200\n  END\nEND\n");
            }

            fs::write(&rc_path, content)?;

            // Try to compile with RC
            let res_path = format!("{out_dir}/app.res");
            if Command::new("rc")
                .args(["/fo", &res_path, &rc_path])
                .status()
                .is_ok()
            {
                println!("cargo:rustc-link-search=native={out_dir}");
                println!("cargo:rustc-link-arg=app.res");
            }
        }

        Ok(())
    }
}
