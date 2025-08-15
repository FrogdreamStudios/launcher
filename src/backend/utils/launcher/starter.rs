//! Minecraft command building utilities.
//!
//! Tools and utilities for constructing Java commands to launch Minecraft.
//! JVM arguments, game arguments, classpath construction, variable substitution, platform-specific
//! configurations (such as Rosetta support on macOS), and compatibility settings for different
//! Minecraft versions.

use crate::utils::Result;
use crate::{log_error, log_info, log_warn, simple_error};
use std::{path::PathBuf, process::Command};

use crate::backend::launcher::launcher::MinecraftLauncher;
use crate::backend::launcher::models::{ArgumentValue, ArgumentValueInner, VersionDetails};

use crate::backend::utils::launcher::paths::{get_classpath_separator, get_natives_dir};
use crate::backend::utils::system::os::{
    get_minecraft_arch, get_minecraft_os_name, get_os_features,
};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Configuration structure for building Minecraft commands.
pub struct CommandConfig {
    pub java_path: PathBuf,
    pub game_dir: PathBuf,
    pub version_details: VersionDetails,
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
    pub version_type: String,
    pub assets_dir: PathBuf,
    pub libraries: Vec<PathBuf>,
    pub main_jar: PathBuf,
    pub java_major_version: u8,
    pub use_rosetta: bool,
}

/// Main command builder for Minecraft launch commands.
pub struct MinecraftCommand {
    java_path: PathBuf,
    game_dir: PathBuf,
    version_details: VersionDetails,
    username: String,
    uuid: String,
    access_token: String,
    user_type: String,
    version_type: String,
    assets_dir: PathBuf,
    libraries: Vec<PathBuf>,
    natives_dir: PathBuf,
    main_jar: PathBuf,
    java_major_version: u8,
    use_rosetta: bool,
}

impl MinecraftCommand {
    /// Initializes the command builder with the provided config and computes the natives' directory.
    pub fn new(config: CommandConfig) -> Self {
        let natives_dir = get_natives_dir(&config.game_dir, &config.version_details.id);

        Self {
            java_path: config.java_path,
            game_dir: config.game_dir,
            version_details: config.version_details,
            username: config.username,
            uuid: config.uuid,
            access_token: config.access_token,
            user_type: config.user_type,
            version_type: config.version_type,
            assets_dir: config.assets_dir,
            libraries: config.libraries,
            main_jar: config.main_jar,
            natives_dir,
            java_major_version: config.java_major_version,
            use_rosetta: config.use_rosetta,
        }
    }

    /// Builds the complete Java command to launch Minecraft.
    pub fn build(&self) -> Result<Command> {
        let mut cmd = if self.use_rosetta && cfg!(target_os = "macos") {
            let script_content = format!(
                r#"#!/bin/bash
            export LSUIElement=0
            export NSHighResolutionCapable=true
            export JAVA_STARTED_ON_FIRST_THREAD_1=1
            export OBJC_DISABLE_INITIALIZE_FORK_SAFETY=YES

            arch -x86_64 "{}" "$@" &
            JAVA_PID=$!

            sleep 3
            osascript -e 'tell application "System Events" to set frontmost of every process whose name contains "java" to true' 2>/dev/null || true

            wait $JAVA_PID
            "#,
                self.java_path.display()
            );

            let script_path = std::env::temp_dir().join("minecraft_launcher.sh");
            std::fs::write(&script_path, script_content)?;

            #[cfg(unix)]
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;

            Command::new(&script_path)
        } else {
            Command::new(&self.java_path)
        };

        if cfg!(target_os = "macos") {
            cmd.env("JAVA_STARTED_ON_FIRST_THREAD_1", "1");
            cmd.env("OBJC_DISABLE_INITIALIZE_FORK_SAFETY", "YES");

            if self.use_rosetta {
                cmd.env("LSUIElement", "0");
                cmd.env("NSHighResolutionCapable", "true");
                cmd.env(
                    "JAVA_TOOL_OPTIONS",
                    "-Djava.awt.headless=false -Dapple.awt.application.name=Minecraft",
                );
            }
        }

        self.add_jvm_arguments(&mut cmd)?;

        cmd.arg(&self.version_details.main_class);

        if self.version_details.main_class == "net.minecraft.launchwrapper.Launch"
            && self.is_version_at_least("1.3")
        {
            cmd.arg("--tweakClass");
            cmd.arg("net.minecraft.client.tweaker.LegacyTweaker");
        }

        self.add_game_arguments(&mut cmd)?;

        cmd.current_dir(&self.game_dir);

        Ok(cmd)
    }

    /// Adds JVM arguments to the command.
    fn add_jvm_arguments(&self, cmd: &mut Command) -> Result<()> {
        cmd.arg("-cp");
        cmd.arg(self.build_classpath());

        cmd.arg("-Dminecraft.launcher.brand=DreamLauncher");
        cmd.arg("-Dminecraft.launcher.version=1.0.0-beta.1");

        self.add_default_jvm_args(cmd);

        if let Some(arguments) = &self.version_details.arguments {
            for arg in &arguments.jvm {
                self.process_argument(cmd, arg, true);
            }
        }

        Ok(())
    }

    /// Adds game arguments to the command.
    fn add_game_arguments(&self, cmd: &mut Command) -> Result<()> {
        if let Some(arguments) = &self.version_details.arguments {
            for arg in &arguments.game {
                self.process_argument(cmd, arg, false);
            }
        } else if let Some(minecraft_arguments) = &self.version_details.minecraft_arguments {
            let args = self.substitute_legacy_arguments(minecraft_arguments);
            let mut current_arg = String::new();
            let mut in_quotes = false;
            for ch in args.chars() {
                match ch {
                    '"' => in_quotes = !in_quotes,
                    ' ' if !in_quotes => {
                        if !current_arg.is_empty() {
                            cmd.arg(&current_arg);
                            current_arg.clear();
                        }
                    }
                    _ => current_arg.push(ch),
                }
            }
            if !current_arg.is_empty() {
                cmd.arg(&current_arg);
            }
        }

        Ok(())
    }

    /// Processes a single argument, handling strings and conditionals.
    fn process_argument(&self, cmd: &mut Command, arg: &ArgumentValue, is_jvm: bool) {
        match arg {
            ArgumentValue::String(s) => {
                let substituted = self.substitute_variables(s, is_jvm);
                if !substituted.trim().is_empty() {
                    cmd.arg(substituted);
                }
            }
            ArgumentValue::Conditional { rules, value } => {
                if Self::evaluate_rules(rules) {
                    match value {
                        ArgumentValueInner::String(s) => {
                            let substituted = self.substitute_variables(s, is_jvm);
                            if !substituted.trim().is_empty() {
                                cmd.arg(substituted);
                            }
                        }
                        ArgumentValueInner::Array(array) => {
                            for s in array {
                                let substituted = self.substitute_variables(s, is_jvm);
                                if !substituted.trim().is_empty() {
                                    cmd.arg(substituted);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Checks if all rules match based on OS name, architecture, and features.
    fn evaluate_rules(rules: &[crate::backend::launcher::models::Rule]) -> bool {
        let os_name = get_minecraft_os_name();
        let os_arch = get_minecraft_arch();
        let features = get_os_features();

        rules
            .iter()
            .all(|rule| rule.matches(os_name, os_arch, &features))
    }

    /// Substitutes variables in a string with actual values.
    fn substitute_variables(&self, input: &str, is_jvm: bool) -> String {
        let mut result = input.to_string();

        result = result.replace("${game_directory}", &self.get_game_directory(false));
        result = result.replace("${assets_root}", &self.get_assets_root(false));
        result = result.replace("${assets_index_name}", &self.version_details.assets);
        result = result.replace("${game_assets}", &self.get_game_assets(false));
        result = result.replace("${auth_player_name}", &self.username);
        result = result.replace("${auth_uuid}", &self.uuid);
        result = result.replace("${auth_access_token}", &self.access_token);
        result = result.replace("${auth_session}", &self.access_token);
        result = result.replace("${user_type}", &self.user_type);
        result = result.replace("${user_properties}", "{}");
        result = result.replace("${version_name}", &self.version_details.id);
        result = result.replace("${version_type}", &self.version_type);
        result = result.replace("${launcher_name}", "DreamLauncher");
        result = result.replace("${launcher_version}", "1.0.0-beta.1");

        if is_jvm {
            result = result.replace(
                "${natives_directory}",
                &self.natives_dir.display().to_string(),
            );
            result = result.replace("${classpath}", &self.build_classpath());
        }

        result
    }

    /// Substitutes variables in legacy argument strings.
    fn substitute_legacy_arguments(&self, args: &str) -> String {
        let mut result = args.to_string();

        result = result.replace("${auth_player_name}", &self.username);
        result = result.replace("${version_name}", &self.version_details.id);
        result = result.replace("${game_directory}", &self.get_game_directory(true));
        result = result.replace("${assets_root}", &self.get_assets_root(true));
        result = result.replace("${assets_index_name}", &self.version_details.assets);
        result = result.replace("${game_assets}", &self.get_game_assets(true));
        result = result.replace("${auth_uuid}", &self.uuid);
        result = result.replace("${auth_access_token}", &self.access_token);
        result = result.replace("${auth_session}", &self.access_token);
        result = result.replace("${user_type}", &self.user_type);
        result = result.replace("${version_type}", &self.version_type);
        result = result.replace("${user_properties}", "{}");

        result
    }

    /// Builds the classpath string.
    fn build_classpath(&self) -> String {
        let separator = get_classpath_separator();
        let mut classpath: Vec<_> = self
            .libraries
            .iter()
            .map(|lib| lib.display().to_string())
            .collect();
        classpath.push(self.main_jar.display().to_string());
        classpath.join(separator)
    }

    /// Adds default JVM arguments based on version and platform.
    fn add_default_jvm_args(&self, cmd: &mut Command) {
        let is_modern_mc = self.is_version_at_least("1.17");
        let is_legacy = self.version_details.arguments.is_none();
        let is_java_8 = self.java_major_version == 8;
        let natives_empty = self
            .natives_dir
            .read_dir()
            .map_or(true, |entries| entries.count() == 0);

        if is_modern_mc {
            cmd.arg("-Xms128M");
            cmd.arg("-Xmx4096M");
        } else {
            cmd.arg("-Xms512M");
            cmd.arg("-Xmx2G");
        }

        cmd.arg("-XX:+UseG1GC");
        cmd.arg("-XX:MaxGCPauseMillis=200");
        cmd.arg("-XX:G1HeapRegionSize=16M");
        cmd.arg("-XX:+DisableExplicitGC");
        cmd.arg("-XX:+ParallelRefProcEnabled");

        match std::env::consts::OS {
            "macos" => {
                cmd.arg("-Djava.awt.headless=false");
                cmd.arg("-Dapple.awt.application.name=Minecraft");

                if is_legacy {
                    if !self.use_rosetta {
                        cmd.arg("-XstartOnFirstThread");
                    }
                    cmd.arg("-Dcom.apple.mrj.application.apple.menu.about.name=Minecraft");
                    cmd.arg("-Dapple.laf.useScreenMenuBar=false");
                    cmd.arg("-Dcom.apple.macos.useScreenMenuBar=false");

                    if self.use_rosetta {
                        cmd.arg("-Djava.awt.Window.locationByPlatform=false");
                        cmd.arg("-Dapple.awt.graphics.EnableQ2DX=false");
                        cmd.arg("-Dapple.awt.graphics.UseQuartz=true");
                        cmd.arg("-Dcom.apple.mrj.application.growbox.intrudes=false");
                        cmd.arg("-Dcom.apple.mrj.application.live-resize=true");
                        cmd.arg("-Djava.awt.Window.alwaysOnTop=false");
                        cmd.arg("-Dapple.awt.application.appearance=system");
                    } else {
                        cmd.arg("-Djava.awt.Window.locationByPlatform=true");
                    }

                    cmd.arg("-Dorg.lwjgl.util.Debug=false");
                    cmd.arg("-Dorg.lwjgl.util.NoChecks=true");
                    cmd.arg("-Dorg.lwjgl.system.macosx.bundleLookup=false");
                    cmd.arg("-Dorg.lwjgl.system.macosx.loadLibrary=system");

                    if self.use_rosetta {
                        cmd.arg("-Dorg.lwjgl.opengl.Display.allowSoftwareOpenGL=true");
                        cmd.arg("-Dorg.lwjgl.system.macosx.windowClass=NSWindow");
                        cmd.arg("-Dorg.lwjgl.system.macosx.forceWindowToFront=true");
                        cmd.arg("-Dorg.lwjgl.system.macosx.activateIgnoringOtherApps=true");
                        cmd.arg("-Dorg.lwjgl.system.macosx.windowLevel=0");
                        cmd.arg("-Dorg.lwjgl.opengl.Window.undecorated=false");
                    }
                } else if !self.use_rosetta {
                    cmd.arg("-XstartOnFirstThread");
                }

                if std::env::consts::ARCH == "aarch64" {
                    if is_legacy {
                        cmd.arg("-Dorg.lwjgl.system.macosx.bundleLookup=false");
                        cmd.arg("-Dorg.lwjgl.system.macosx.loadLibrary=system");
                    } else if !is_java_8 {
                        cmd.arg("--add-opens");
                        cmd.arg("java.base/java.nio=ALL-UNNAMED");
                        cmd.arg("--add-opens");
                        cmd.arg("java.base/sun.nio.ch=ALL-UNNAMED");
                    }
                }

                let library_path = if natives_empty {
                    "/System/Library/Frameworks".to_string()
                } else {
                    self.natives_dir.display().to_string()
                };
                cmd.arg(format!("-Djava.library.path={library_path}"));
            }
            "windows" => {
                cmd.arg("-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump");
                cmd.arg("-Dos.name=Windows 10");
                cmd.arg("-Dos.version=10.0");
                cmd.arg("-Dorg.lwjgl.opengl.Window.undecorated=false");
            }
            _ => {
                cmd.arg("-Dorg.lwjgl.opengl.libname=libGL.so.1");
                cmd.arg("-Dorg.lwjgl.system.SharedLibraryExtractPath=/tmp/lwjgl");
            }
        }

        cmd.arg("-Dfml.ignoreInvalidMinecraftCertificates=true");
        cmd.arg("-Dfml.ignorePatchDiscrepancies=true");
        cmd.arg("-Dlog4j2.formatMsgNoLookups=true");
        cmd.arg("-Djava.rmi.server.useCodebaseOnly=true");
        cmd.arg("-Dcom.sun.jndi.rmi.object.trustURLCodebase=false");
        cmd.arg("-Dcom.sun.jndi.cosnaming.object.trustURLCodebase=false");

        if !natives_empty {
            cmd.arg(format!(
                "-Dorg.lwjgl.librarypath={}",
                self.natives_dir.display()
            ));
        } else {
            cmd.arg("-Dorg.lwjgl.system.allocator=system");
        }

        if is_modern_mc && !is_java_8 {
            cmd.arg("--add-exports");
            cmd.arg("java.base/sun.security.util=ALL-UNNAMED");
            cmd.arg("--add-exports");
            cmd.arg("java.base/sun.security.x509=ALL-UNNAMED");
        }
    }

    fn get_game_directory(&self, quote: bool) -> String {
        let path = self.game_dir.display().to_string();
        if quote && path.contains(' ') {
            format!("\"{path}\"")
        } else {
            path
        }
    }

    fn get_assets_root(&self, quote: bool) -> String {
        let path = self.assets_dir.display().to_string();
        if quote && path.contains(' ') {
            format!("\"{path}\"")
        } else {
            path
        }
    }

    fn get_game_assets(&self, quote: bool) -> String {
        let assets = &self.version_details.assets;
        let virtual_path = self.assets_dir.join("virtual").join(assets);
        let path = if assets == "pre-1.6" || assets == "legacy" {
            if virtual_path.exists() {
                virtual_path.display().to_string()
            } else {
                self.assets_dir.display().to_string()
            }
        } else if self.is_version_at_least("1.7") {
            virtual_path.display().to_string()
        } else {
            self.assets_dir.display().to_string()
        };
        if quote && path.contains(' ') {
            format!("\"{path}\"")
        } else {
            path
        }
    }

    fn is_version_at_least(&self, target: &str) -> bool {
        let self_parts: Vec<i32> = self
            .version_details
            .id
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        let target_parts: Vec<i32> = target.split('.').filter_map(|s| s.parse().ok()).collect();
        let max_len = self_parts.len().max(target_parts.len());
        let mut s = self_parts;
        let mut t = target_parts;
        s.resize_with(max_len, || 0);
        t.resize_with(max_len, || 0);
        for i in 0..max_len {
            if s[i] > t[i] {
                return true;
            }
            if s[i] < t[i] {
                return false;
            }
        }
        true
    }
}

/// Builder pattern for creating `MinecraftCommand` instances.
pub struct CommandBuilder {
    java_path: Option<PathBuf>,
    game_dir: Option<PathBuf>,
    version_details: Option<VersionDetails>,
    username: Option<String>,
    uuid: Option<String>,
    access_token: Option<String>,
    user_type: String,
    version_type: String,
    assets_dir: Option<PathBuf>,
    libraries: Vec<PathBuf>,
    main_jar: Option<PathBuf>,
    java_major_version: Option<u8>,
    use_rosetta: bool,
}

impl CommandBuilder {
    /// Creates a new `CommandBuilder` with default values.
    pub fn new() -> Self {
        Self {
            java_path: None,
            game_dir: None,
            version_details: None,
            username: None,
            uuid: None,
            access_token: None,
            user_type: "mojang".to_string(),
            version_type: "release".to_string(),
            assets_dir: None,
            libraries: Vec::new(),
            main_jar: None,
            java_major_version: None,
            use_rosetta: false,
        }
    }

    pub fn java_path(mut self, path: PathBuf) -> Self {
        self.java_path = Some(path);
        self
    }

    pub fn game_dir(mut self, dir: PathBuf) -> Self {
        self.game_dir = Some(dir);
        self
    }

    pub fn version_details(mut self, details: VersionDetails) -> Self {
        self.version_details = Some(details);
        self
    }

    pub fn username(mut self, name: String) -> Self {
        self.username = Some(name);
        self
    }

    pub fn uuid(mut self, id: String) -> Self {
        self.uuid = Some(id);
        self
    }

    pub fn access_token(mut self, token: String) -> Self {
        self.access_token = Some(token);
        self
    }

    pub fn user_type(mut self, user_type: String) -> Self {
        self.user_type = user_type;
        self
    }

    pub fn version_type(mut self, version_type: String) -> Self {
        self.version_type = version_type;
        self
    }

    pub fn assets_dir(mut self, dir: PathBuf) -> Self {
        self.assets_dir = Some(dir);
        self
    }

    pub fn libraries(mut self, libs: Vec<PathBuf>) -> Self {
        self.libraries = libs;
        self
    }

    pub fn main_jar(mut self, main_jar: PathBuf) -> Self {
        self.main_jar = Some(main_jar);
        self
    }

    pub const fn java_major_version(mut self, java_major_version: u8) -> Self {
        self.java_major_version = Some(java_major_version);
        self
    }

    pub const fn use_rosetta(mut self, use_rosetta: bool) -> Self {
        self.use_rosetta = use_rosetta;
        self
    }

    pub fn build(self) -> Result<MinecraftCommand> {
        let config = CommandConfig {
            java_path: self
                .java_path
                .ok_or_else(|| simple_error!("Java path not set"))?,
            game_dir: self
                .game_dir
                .ok_or_else(|| simple_error!("Game directory not set"))?,
            version_details: self
                .version_details
                .ok_or_else(|| simple_error!("Version details not set"))?,
            username: self.username.unwrap_or_else(|| "Player".to_string()),
            uuid: self
                .uuid
                .unwrap_or_else(|| "00000000-0000-0000-0000-000000000000".to_string()),
            access_token: self.access_token.unwrap_or_else(|| "null".to_string()),
            user_type: self.user_type,
            version_type: self.version_type,
            assets_dir: self
                .assets_dir
                .ok_or_else(|| simple_error!("Assets directory not set"))?,
            libraries: self.libraries,
            main_jar: self
                .main_jar
                .ok_or_else(|| simple_error!("Main jar not set"))?,
            java_major_version: self.java_major_version.unwrap_or(8),
            use_rosetta: self.use_rosetta,
        };
        Ok(MinecraftCommand::new(config))
    }
}

impl Default for CommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Launch Minecraft with the specified version and instance
pub async fn launch_minecraft(version: String, instance_id: u32) -> Result<()> {
    log_info!("Starting Minecraft launch for version: {version}");

    let mut launcher = MinecraftLauncher::new(None, Some(instance_id)).await?;
    log_info!("Launcher created successfully");

    // Check if a version exists locally
    let game_dir = launcher.get_game_dir();
    let version_dir = game_dir.join("versions").join(&version);
    let jar_file = version_dir.join(format!("{version}.jar"));
    let json_file = version_dir.join(format!("{version}.json"));

    let version_exists = jar_file.exists() && json_file.exists();

    if !version_exists {
        log_info!("Version {version} not found locally, attempting to install...");

        // Update manifest first
        match launcher.update_manifest().await {
            Ok(_) => log_info!("Manifest updated successfully"),
            Err(e) => {
                log_warn!("Failed to update manifest: {e}, continuing anyway...")
            }
        }

        // Install/prepare the version
        launcher.prepare_version(&version).await.map_err(|e| {
            log_error!("Failed to install version {version}: {e}");
            e
        })?;

        log_info!("Version {version} installed successfully");
    }

    // Check Java availability
    let java_available = launcher.is_java_available(&version);

    if !java_available {
        log_info!("Java not available for version {version}, installing...");

        launcher.install_java(&version).await.map_err(|e| {
            log_error!("Failed to install Java for version {version}: {e}");
            e
        })?;

        log_info!("Java installed successfully for version {version}");
    }

    log_info!("Starting Minecraft {version}...");

    // Launch Minecraft
    launcher.launch(&version).await.map_err(|e| {
        log_error!("Failed to launch Minecraft {version}: {e}");
        e
    })?;

    log_info!("Minecraft {version} launched successfully");
    Ok(())
}
