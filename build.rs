use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/images/other/icon.png");
    println!("cargo:rerun-if-changed=assets/icons/app_icon.icns");

    let profile = env::var("PROFILE").unwrap_or_default();

    if cfg!(target_os = "macos") && profile == "release" {
        println!(
            "cargo:warning=For macOS releases, use the generated .dmg file instead of the raw executable"
        );
        println!(
            "cargo:warning=Run: create-dmg --volname \"Dream Launcher\" --app-drop-link 600 185 \"Dream Launcher.dmg\" \"target/release/Dream Launcher.app\""
        );

        let executable_name = "DreamLauncher";
        let executable_path = format!("target/{}/{}", profile, executable_name);
        let icon_path = "assets/icons/app_icon.icns";

        if let Ok(_) = Command::new("which").arg("fileicon").output() {
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
