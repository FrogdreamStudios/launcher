use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let package_json = Path::new(&manifest_dir).join("package.json");
    if package_json.exists() {
        let status = Command::new("npm")
            .arg("run")
            .arg("build:css")
            .current_dir(&manifest_dir)
            .status()
            .expect("failed to run npm build:css");
        if !status.success() {
            panic!("npm run build:css failed");
        }
    } else {
        println!("cargo:warning=package.json not found, skipping CSS build");
    }
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=tailwind.config.js");
    println!("cargo:rerun-if-changed=public/assets/styles/tailwind.css");
}
