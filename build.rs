//! Build script for the `Dream Launcher`.
//!
//! This script handles CSS compilation, font embedding, and asset processing
//! during the build process using Tailwind CSS and npm/npx.

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn is_command_available(command: &str) -> bool {
    let check_command = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };

    let command_to_check = if cfg!(target_os = "windows") {
        format!("{command}.cmd")
    } else {
        command.to_owned()
    };

    Command::new(check_command)
        .arg(&command_to_check)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run_npm_command(args: &[&str]) -> std::io::Result<std::process::ExitStatus> {
    if cfg!(target_os = "windows") {
        Command::new("cmd.exe")
            .args(["/C", "npm"])
            .args(args)
            .status()
    } else {
        Command::new("npm").args(args).status()
    }
}

fn run_npx_command(args: &[&str]) -> std::io::Result<std::process::ExitStatus> {
    if cfg!(target_os = "windows") {
        let mut cmd_args = vec!["/C", "npx"];
        cmd_args.extend(args);
        Command::new("cmd.exe").args(&cmd_args).status()
    } else {
        Command::new("npx").args(args).status()
    }
}

fn build_with_npx_fallback() {
    if !is_command_available("npx") {
        println!("cargo:warning=npx not found either, skipping CSS build");
        return;
    }

    let tailwind_input = "assets/styles/pages/main.css";
    let tailwind_output = "assets/styles/other/output.css";

    let fallback_status = run_npx_command(&[
        "tailwindcss",
        "-i",
        tailwind_input,
        "-o",
        tailwind_output,
        "--minify",
    ]);

    match fallback_status {
        Ok(s) if s.success() => {
            println!("cargo:warning=Tailwind CSS built successfully with npx");
        }
        Ok(s) => {
            println!("cargo:warning=npx Tailwind CSS build failed with status: {s}");
        }
        Err(e) => {
            println!("cargo:warning=Failed to run npx Tailwind CSS build: {e}");
        }
    }
}

fn generate_font_constants() {
    let fonts = vec![
        ("gilroy_medium", "assets/fonts/Gilroy/Gilroy-Medium.ttf"),
        ("gilroy_bold", "assets/fonts/Gilroy/Gilroy-Bold.ttf"),
    ];

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("fonts.rs");

    let mut content = String::from("// Auto-generated font constants\n\n");
    content.push_str(
        "pub fn get_fonts() -> std::collections::HashMap<&'static str, &'static str> {\n",
    );
    content.push_str("    let mut fonts = std::collections::HashMap::new();\n");

    for (name, path) in &fonts {
        if let Ok(data) = fs::read(path) {
            let base64_data = base64_encode(&data);
            content.push_str(&format!(
                "    fonts.insert(\"{name}\", \"data:font/truetype;base64,{base64_data}\");\n"
            ));
            println!("cargo:rerun-if-changed={path}");
        } else {
            println!("cargo:warning=Failed to read font: {path}");
        }
    }

    content.push_str("    fonts\n");
    content.push_str("}\n");

    fs::write(&dest_path, content).unwrap();
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, &byte) in chunk.iter().enumerate() {
            buf[i] = byte;
        }

        let b = (u32::from(buf[0]) << 16) | (u32::from(buf[1]) << 8) | u32::from(buf[2]);

        result.push(CHARS[((b >> 18) & 63) as usize] as char);
        result.push(CHARS[((b >> 12) & 63) as usize] as char);
        result.push(if chunk.len() > 1 {
            CHARS[((b >> 6) & 63) as usize] as char
        } else {
            '='
        });
        result.push(if chunk.len() > 2 {
            CHARS[(b & 63) as usize] as char
        } else {
            '='
        });
    }

    result
}

fn main() {
    println!("cargo:rerun-if-changed=assets/icons/app_icon.icns");

    // Generate font constants
    generate_font_constants();

    println!("cargo:rerun-if-changed=assets/styles/main.css");
    println!("cargo:rerun-if-changed=assets/styles/auth.css");
    println!("cargo:rerun-if-changed=assets/styles/chat.css");
    println!("cargo:rerun-if-changed=assets/styles/tailwind.css");
    println!("cargo:rerun-if-changed=package.json");

    // Add rerun triggers for all Rust source files
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Check if npm is available and try npm build first
    if is_command_available("npm") {
        println!("cargo:warning=npm found, attempting npm build...");

        // Install npm dependencies if needed
        let npm_install_status = run_npm_command(&["install"]);

        match npm_install_status {
            Ok(s) if s.success() => {
                println!("cargo:warning=npm install completed successfully");
            }
            Ok(s) => {
                println!("cargo:warning=npm install failed with status: {s}");
            }
            Err(e) => {
                println!("cargo:warning=Failed to run npm install: {e}");
            }
        }

        // Build CSS using npm script
        let npm_build_status = run_npm_command(&["run", "build:css"]);

        match npm_build_status {
            Ok(s) if s.success() => {
                println!("cargo:warning=npm build:css completed successfully");
            }
            Ok(s) => {
                println!("cargo:warning=npm build:css failed with status: {s}");
                build_with_npx_fallback();
            }
            Err(e) => {
                println!("cargo:warning=Failed to run npm build:css: {e}");
                build_with_npx_fallback();
            }
        }
    } else {
        println!("cargo:warning=npm not found, using npx fallback...");
        build_with_npx_fallback();
    }

    let profile = env::var("PROFILE").unwrap_or_default();

    if cfg!(target_os = "macos") && profile == "release" {
        println!(
            "cargo:warning=For macOS releases, use the generated .dmg file instead of the raw executable"
        );

        let executable_name = env!("CARGO_PKG_NAME");
        let executable_path = format!("target/{profile}/{executable_name}");
        let icon_path = "assets/icons/app_icon.icns";

        if Command::new("which").arg("fileicon").output().is_ok() {
            let _ = Command::new("fileicon")
                .arg("set")
                .arg(&executable_path)
                .arg(icon_path)
                .output();
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/images/other/icon.ico");
        res.set("ProductName", "Dream Launcher");
        res.set(
            "FileDescription",
            "A powerful and lightweight Minecraft launcher that will be perfect for every player",
        );
        res.set("CompanyName", "Frogdream Studios");
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        let _ = res.compile();
    }
}
