use std::env;

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/images/other/icon.png");
    println!("cargo:rerun-if-changed=assets/icons/app_icon.icns");

    println!("cargo:rerun-if-changed=assets/styles/main.css");
    println!("cargo:rerun-if-changed=assets/styles/auth.css");
    println!("cargo:rerun-if-changed=assets/styles/chat.css");

    // Build Tailwind CSS
    let tailwind_input = "assets/styles/main.css";
    let tailwind_output = "assets/styles/output.css";

    let status = Command::new("npx")
        .arg("tailwindcss")
        .arg("-i")
        .arg(tailwind_input)
        .arg("-o")
        .arg(tailwind_output)
        .arg("--minify")
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("cargo:warning=Tailwind CSS built successfully.");
        }
        Ok(s) => {
            println!("cargo:warning=Tailwind CSS build failed with status: {}", s);
        }
        Err(e) => {
            println!("cargo:warning=Failed to run Tailwind CSS build: {}", e);
        }
    }

    let profile = env::var("PROFILE").unwrap_or_default();

    if cfg!(target_os = "macos") && profile == "release" {
        println!(
            "cargo:warning=For macOS releases, use the generated .dmg file instead of the raw executable"
        );
        println!(
            "cargo:warning=Run: create-dmg --volname \"Dream Launcher\" --app-drop-link 600 185 \"Dream Launcher.dmg\" \"target/release/Dream Launcher.app\""
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
        res.set("FileDescription", "A powerful Minecraft launcher");
        res.set("CompanyName", "Frogdream Studios");
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        let _ = res.compile();
    }
}
