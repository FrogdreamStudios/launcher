//! Build script for the `Dream Launcher`.

use std::{env, fs, path::Path, process::Command};
use log::{info, warn, error};

fn main() {
    info!("cargo:rerun-if-changed=assets/icons/app_icon.icns");
    info!("cargo:rerun-if-changed=assets/fonts/Gilroy/Gilroy-Medium.ttf");
    info!("cargo:rerun-if-changed=assets/fonts/Gilroy/Gilroy-Bold.ttf");
    info!("cargo:rerun-if-changed=assets/styles/pages/main.css");
    info!("cargo:rerun-if-changed=package.json");
    info!("cargo:rerun-if-changed=src/");
    info!("cargo:rerun-if-changed=Cargo.toml");

    // Compile CSS using npx tailwindcss
    let css_in = "assets/styles/pages/main.css";
    let css_out = "assets/styles/other/output.css";
    let status = Command::new("npx")
        .args([
            "tailwindcss",
            "-i", css_in,
            "-o", css_out,
            "--minify",
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            info!("Tailwind CSS has been compiled successfully");
        }
        Ok(s) => {
            warn!("Error compiling Tailwind CSS: {s}");
        }
        Err(e) => {
            error!("npx/tailwindcss not found: {e}");
        }
    }

    // Embed fonts
    embed_fonts();

    // Set icon for Windows
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/images/other/icon.ico");
        res.set("ProductName", "Dream Launcher");
        res.set("FileDescription", "A powerful and lightweight Minecraft launcher");
        res.set("CompanyName", "Frogdream Studios");
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        let _ = res.compile();
    }
}

fn embed_fonts() {
    use base64::{engine::general_purpose, Engine as _};

    let fonts = [
        ("gilroy_medium", "assets/fonts/Gilroy/Gilroy-Medium.ttf"),
        ("gilroy_bold", "assets/fonts/Gilroy/Gilroy-Bold.ttf"),
    ];

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("fonts.rs");

    let mut content = String::from("// Font constants\n\n");
    content.push_str("pub fn get_fonts() -> std::collections::HashMap<&'static str, &'static str> {\n");
    content.push_str("    let mut fonts = std::collections::HashMap::new();\n");

    for (name, path) in fonts {
        match fs::read(path) {
            Ok(data) => {
                let base64_data = general_purpose::STANDARD.encode(&data);
                content.push_str(&format!(
                    "    fonts.insert(\"{name}\", \"data:font/truetype;base64,{base64_data}\");\n"
                ));
                info!("cargo:rerun-if-changed={path}");
            }
            Err(e) => {
                error!("Can't load font in {path}: {e}");
            }
        }
    }

    content.push_str("    fonts\n}\n");

    if let Err(e) = fs::write(&dest_path, content) {
        error!("Error writing fonts.rs: {e}");
    }
}
