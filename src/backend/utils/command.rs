use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

use crate::backend::creeper::model::{ArgumentValue, ArgumentValueInner, VersionDetails};
use crate::backend::utils::os::{get_minecraft_arch, get_minecraft_os_name, get_os_features};
use crate::backend::utils::paths::{get_classpath_separator, get_natives_dir};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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
    pub fn new(
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
        main_jar: PathBuf,
        java_major_version: u8,
        use_rosetta: bool,
    ) -> Self {
        let natives_dir = get_natives_dir(&game_dir, &version_details.id);

        Self {
            java_path,
            game_dir,
            version_details,
            username,
            uuid,
            access_token,
            user_type,
            version_type,
            assets_dir,
            libraries,
            natives_dir,
            main_jar,
            java_major_version,
            use_rosetta,
        }
    }

    pub fn build(&self) -> Result<Command> {
        let mut cmd = if self.use_rosetta && cfg!(target_os = "macos") {
            // Create simple wrapper script for Rosetta with window activation
            let script_content = format!(
                r#"#!/bin/bash
export LSUIElement=0
export NSHighResolutionCapable=true
export JAVA_STARTED_ON_FIRST_THREAD_1=1
export OBJC_DISABLE_INITIALIZE_FORK_SAFETY=YES

# Launch Java with Rosetta
arch -x86_64 "{}" "$@" &
JAVA_PID=$!

# Wait for window creation and activate it
sleep 3
osascript -e 'tell application "System Events" to set frontmost of every process whose name contains "java" to true' 2>/dev/null || true

# Wait for process to complete
wait $JAVA_PID
"#,
                self.java_path.display()
            );

            let script_path = std::env::temp_dir().join("minecraft_launcher.sh");
            std::fs::write(&script_path, script_content)?;

            #[cfg(unix)]
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;

            let cmd = Command::new(&script_path);
            cmd
        } else {
            Command::new(&self.java_path)
        };

        // Set environment variables for GUI display on macOS
        if cfg!(target_os = "macos") {
            cmd.env("JAVA_STARTED_ON_FIRST_THREAD_1", "1");
            cmd.env("OBJC_DISABLE_INITIALIZE_FORK_SAFETY", "YES");

            if self.use_rosetta {
                // Additional Rosetta environment variables
                cmd.env("LSUIElement", "0");
                cmd.env("NSHighResolutionCapable", "true");
                cmd.env(
                    "JAVA_TOOL_OPTIONS",
                    "-Djava.awt.headless=false -Dapple.awt.application.name=Minecraft",
                );
            }
        }

        // Add JVM arguments
        self.add_jvm_arguments(&mut cmd)?;

        // Add main class
        cmd.arg(&self.version_details.main_class);

        // For LaunchWrapper versions (very old MC), add additional arguments
        if self.version_details.main_class == "net.minecraft.launchwrapper.Launch" {
            // Only add tweaker class for versions that have it (1.3+)
            // Very old versions (1.0-1.2.x) don't have LegacyTweaker
            let version_parts: Vec<&str> = self.version_details.id.split('.').collect();
            let needs_tweaker = if version_parts.len() >= 2 {
                if let (Ok(major), Ok(minor)) = (
                    version_parts[0].parse::<i32>(),
                    version_parts[1].parse::<i32>(),
                ) {
                    major > 1 || (major == 1 && minor >= 3)
                } else {
                    false // If we can't parse, don't add tweaker
                }
            } else {
                false
            };

            if needs_tweaker {
                cmd.arg("--tweakClass");
                cmd.arg("net.minecraft.client.tweaker.LegacyTweaker");
            }
        }

        // Add game arguments
        self.add_game_arguments(&mut cmd)?;

        // Set working directory
        cmd.current_dir(&self.game_dir);

        Ok(cmd)
    }

    fn add_jvm_arguments(&self, cmd: &mut Command) -> Result<()> {
        // Add classpath
        let classpath = self.build_classpath();
        cmd.arg("-cp");
        cmd.arg(classpath);

        // Natives directory will be set in add_default_jvm_args to avoid duplicates

        // Add launcher name and version
        cmd.arg("-Dminecraft.launcher.brand=DreamLauncher");
        cmd.arg("-Dminecraft.launcher.version=1.0.0-beta.1");

        // Add default JVM arguments
        self.add_default_jvm_args(cmd);

        // Add version-specific JVM arguments
        if let Some(arguments) = &self.version_details.arguments {
            for arg in &arguments.jvm {
                self.process_argument(cmd, arg, true)?;
            }
        }

        Ok(())
    }

    fn add_game_arguments(&self, cmd: &mut Command) -> Result<()> {
        if let Some(arguments) = &self.version_details.arguments {
            // Modern argument format (1.13+)
            for arg in &arguments.game {
                self.process_argument(cmd, arg, false)?;
            }
        } else if let Some(minecraft_arguments) = &self.version_details.minecraft_arguments {
            // Legacy argument format (pre-1.13)
            let args = self.substitute_legacy_arguments(minecraft_arguments);

            // Parse arguments while preserving quoted strings
            let mut current_arg = String::new();
            let mut in_quotes = false;
            let mut chars = args.chars().peekable();

            while let Some(ch) = chars.next() {
                match ch {
                    '"' => {
                        in_quotes = !in_quotes;
                    }
                    ' ' if !in_quotes => {
                        if !current_arg.is_empty() {
                            cmd.arg(&current_arg);
                            current_arg.clear();
                        }
                    }
                    _ => {
                        current_arg.push(ch);
                    }
                }
            }

            // Add the last argument if any
            if !current_arg.is_empty() {
                cmd.arg(&current_arg);
            }
        }

        Ok(())
    }

    fn process_argument(&self, cmd: &mut Command, arg: &ArgumentValue, is_jvm: bool) -> Result<()> {
        match arg {
            ArgumentValue::String(s) => {
                let substituted = self.substitute_variables(s, is_jvm);
                if !substituted.trim().is_empty() {
                    cmd.arg(substituted);
                }
            }
            ArgumentValue::Conditional { rules, value } => {
                if self.evaluate_rules(rules) {
                    match value {
                        ArgumentValueInner::String(s) => {
                            let substituted = self.substitute_variables(s, is_jvm);
                            if !substituted.trim().is_empty() {
                                cmd.arg(substituted);
                            }
                        }
                        ArgumentValueInner::Array(arr) => {
                            for s in arr {
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

        Ok(())
    }

    fn evaluate_rules(&self, rules: &[crate::backend::creeper::model::Rule]) -> bool {
        let os_name = get_minecraft_os_name();
        let os_arch = get_minecraft_arch();
        let features = get_os_features();

        for rule in rules {
            if !rule.matches(os_name, os_arch, &features) {
                return false;
            }
        }

        true
    }

    fn substitute_variables(&self, input: &str, is_jvm: bool) -> String {
        let mut result = input.to_string();

        // Game directory - DO NOT quote as it breaks path resolution
        let game_dir_str = self.game_dir.display().to_string();
        result = result.replace("${game_directory}", &game_dir_str);

        // Assets - DO NOT quote assets directory as it breaks path resolution
        let assets_dir_str = self.assets_dir.display().to_string();
        result = result.replace("${assets_root}", &assets_dir_str);
        result = result.replace("${assets_index_name}", &self.version_details.assets);

        // Handle game_assets based on version and asset type
        let game_assets_str = if self.version_details.assets == "pre-1.6"
            || self.version_details.assets == "legacy"
        {
            // For very old versions, try virtual assets first
            let legacy_assets_path = self
                .assets_dir
                .join("virtual")
                .join(&self.version_details.assets);
            if legacy_assets_path.exists() {
                legacy_assets_path.display().to_string()
            } else {
                // Fallback to regular assets directory
                assets_dir_str.clone()
            }
        } else if self.version_details.id >= "1.7".to_string() {
            // For 1.7+ versions, use virtual assets
            let virtual_assets_path = self
                .assets_dir
                .join("virtual")
                .join(&self.version_details.assets);
            virtual_assets_path.display().to_string()
        } else {
            // For other versions, use regular assets directory
            assets_dir_str.clone()
        };
        result = result.replace("${game_assets}", &game_assets_str);

        // User info
        result = result.replace("${auth_player_name}", &self.username);
        result = result.replace("${auth_uuid}", &self.uuid);
        result = result.replace("${auth_access_token}", &self.access_token);
        result = result.replace("${user_type}", &self.user_type);

        // Legacy auth session support (for very old versions like 1.0-1.5.x)
        result = result.replace("${auth_session}", &self.access_token);

        // User properties - Minecraft expects this as a JSON object
        // For offline mode, we provide an empty JSON object
        result = result.replace("${user_properties}", "{}");

        // Version info
        result = result.replace("${version_name}", &self.version_details.id);
        result = result.replace("${version_type}", &self.version_type);

        // Launcher info
        result = result.replace("${launcher_name}", "DreamLauncher");
        result = result.replace("${launcher_version}", "1.0.0-beta.1");

        // Natives directory (JVM only) - DO NOT quote
        if is_jvm {
            let natives_dir_str = self.natives_dir.display().to_string();
            result = result.replace("${natives_directory}", &natives_dir_str);
        }

        // Classpath (JVM only)
        if is_jvm {
            result = result.replace("${classpath}", &self.build_classpath());
        }

        result
    }

    fn substitute_legacy_arguments(&self, args: &str) -> String {
        let mut result = args.to_string();

        result = result.replace("${auth_player_name}", &self.username);
        result = result.replace("${version_name}", &self.version_details.id);

        // Quote paths with spaces for legacy arguments
        let game_dir_str = self.game_dir.display().to_string();
        let game_dir_quoted = if game_dir_str.contains(' ') {
            format!("\"{game_dir_str}\"")
        } else {
            game_dir_str
        };
        result = result.replace("${game_directory}", &game_dir_quoted);

        let assets_dir_str = self.assets_dir.display().to_string();
        let assets_dir_quoted = if assets_dir_str.contains(' ') {
            format!("\"{assets_dir_str}\"")
        } else {
            assets_dir_str
        };
        result = result.replace("${assets_root}", &assets_dir_quoted);

        result = result.replace("${assets_index_name}", &self.version_details.assets);
        result = result.replace("${auth_uuid}", &self.uuid);
        result = result.replace("${auth_access_token}", &self.access_token);
        result = result.replace("${user_type}", &self.user_type);
        result = result.replace("${version_type}", &self.version_type);

        // Legacy auth session support (for very old versions like 1.0-1.5.x)
        result = result.replace("${auth_session}", &self.access_token);

        // Handle game_assets for old versions (pre-1.7 used different asset structure)
        let game_assets_str = if self.version_details.assets == "pre-1.6"
            || self.version_details.assets == "legacy"
        {
            // For very old versions, game_assets points to assets/virtual/legacy or similar
            let legacy_assets_path = self
                .assets_dir
                .join("virtual")
                .join(&self.version_details.assets);
            if legacy_assets_path.exists() {
                legacy_assets_path.display().to_string()
            } else {
                // Fallback to regular assets directory
                self.assets_dir.display().to_string()
            }
        } else {
            // For newer versions, use regular assets directory
            self.assets_dir.display().to_string()
        };

        // Quote game_assets path if it contains spaces
        let game_assets_quoted = if game_assets_str.contains(' ') {
            format!("\"{game_assets_str}\"")
        } else {
            game_assets_str
        };
        result = result.replace("${game_assets}", &game_assets_quoted);

        // User properties - Minecraft expects this as a JSON object
        // For offline mode, we provide an empty JSON object
        result = result.replace("${user_properties}", "{}");

        result
    }

    fn build_classpath(&self) -> String {
        let separator = get_classpath_separator();
        let mut classpath = Vec::new();

        // Add libraries
        for lib in &self.libraries {
            classpath.push(lib.display().to_string());
        }

        // Add main jar
        classpath.push(self.main_jar.display().to_string());

        classpath.join(separator)
    }

    fn add_default_jvm_args(&self, cmd: &mut Command) {
        // Determine optimal memory settings based on version
        let is_modern_version = self.version_details.id >= "1.17".to_string();

        if is_modern_version {
            // Modern versions (1.17+) need more memory
            cmd.arg("-Xms1G");
            cmd.arg("-Xmx4G");
        } else {
            // Older versions work fine with less memory
            cmd.arg("-Xms512M");
            cmd.arg("-Xmx2G");
        }

        // Conservative GC settings for better compatibility
        cmd.arg("-XX:+UseG1GC");
        cmd.arg("-XX:MaxGCPauseMillis=200");
        cmd.arg("-XX:G1HeapRegionSize=16M");

        // Essential stability settings
        cmd.arg("-XX:+DisableExplicitGC");
        cmd.arg("-XX:+ParallelRefProcEnabled");

        // Check if natives directory is empty (ARM64 compatibility)
        let natives_empty = std::fs::read_dir(&self.natives_dir)
            .map(|entries| entries.count() == 0)
            .unwrap_or(true);

        // Platform-specific arguments
        match std::env::consts::OS {
            "macos" => {
                // Check if this is a legacy Minecraft version (pre-1.13)
                let is_legacy_version = {
                    let version_parts: Vec<&str> = self.version_details.id.split('.').collect();
                    if version_parts.len() >= 2 {
                        if let (Ok(major), Ok(minor)) = (
                            version_parts[0].parse::<i32>(),
                            version_parts[1].parse::<i32>(),
                        ) {
                            major == 1 && minor < 13
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                };

                // Only add -XstartOnFirstThread for non-Rosetta or modern versions
                // Legacy versions with Rosetta have window display issues with this flag
                if !self.use_rosetta || !is_legacy_version {
                    cmd.arg("-XstartOnFirstThread");
                }

                // Use actual Java version being used
                let is_java_8 = self.java_major_version == 8;

                if is_legacy_version {
                    // Legacy version approach - force window visibility
                    cmd.arg("-Dapple.awt.application.name=Minecraft");
                    cmd.arg("-Dcom.apple.mrj.application.apple.menu.about.name=Minecraft");

                    // Critical AWT settings for window display
                    cmd.arg("-Djava.awt.headless=false");
                    cmd.arg("-Dapple.laf.useScreenMenuBar=false");
                    cmd.arg("-Dcom.apple.macos.useScreenMenuBar=false");

                    if self.use_rosetta {
                        // Rosetta 2 specific fixes - force window creation and activation
                        cmd.arg("-Dcom.apple.awt.application.name=Minecraft");
                        cmd.arg("-Djava.awt.Window.locationByPlatform=false");
                        cmd.arg("-Dapple.awt.graphics.EnableQ2DX=false");
                        cmd.arg("-Dapple.awt.graphics.UseQuartz=true");
                        cmd.arg("-Dcom.apple.mrj.application.growbox.intrudes=false");
                        cmd.arg("-Dcom.apple.mrj.application.live-resize=true");

                        // Force window to be visible and on top initially
                        cmd.arg("-Djava.awt.Window.alwaysOnTop=false");
                        cmd.arg("-Dapple.awt.application.appearance=system");
                    } else {
                        cmd.arg("-Djava.awt.Window.locationByPlatform=true");
                    }

                    // Critical: Set natives directory for LWJGL
                    if !natives_empty {
                        cmd.arg(format!(
                            "-Djava.library.path={}",
                            self.natives_dir.display()
                        ));
                    } else {
                        cmd.arg("-Djava.library.path=/System/Library/Frameworks");
                    }

                    // Essential LWJGL settings for legacy versions
                    cmd.arg("-Dorg.lwjgl.util.Debug=false");
                    cmd.arg("-Dorg.lwjgl.util.NoChecks=true");
                    cmd.arg("-Dorg.lwjgl.system.macosx.bundleLookup=false");
                    cmd.arg("-Dorg.lwjgl.system.macosx.loadLibrary=system");

                    // Enhanced LWJGL window settings for macOS
                    if self.use_rosetta {
                        cmd.arg("-Dorg.lwjgl.opengl.Display.allowSoftwareOpenGL=true");
                        cmd.arg("-Dorg.lwjgl.system.macosx.windowClass=NSWindow");
                        cmd.arg("-Dorg.lwjgl.system.macosx.forceWindowToFront=true");
                        cmd.arg("-Dorg.lwjgl.system.macosx.activateIgnoringOtherApps=true");

                        // Try to force window creation on main display
                        cmd.arg("-Dorg.lwjgl.system.macosx.windowLevel=0");
                        cmd.arg("-Dorg.lwjgl.opengl.Window.undecorated=false");
                    }
                } else {
                    // Modern version approach - comprehensive fixes
                    cmd.arg("-Djava.awt.headless=false");
                    cmd.arg("-Dapple.awt.application.name=Minecraft");
                    cmd.arg("-Dapple.laf.useScreenMenuBar=false");

                    // Comprehensive LWJGL fixes for macOS
                    cmd.arg("-Dorg.lwjgl.opengl.Display.allowSoftwareOpenGL=true");
                    cmd.arg("-Dorg.lwjgl.system.allocator=system");
                    cmd.arg("-Dorg.lwjgl.system.stackSize=4096");
                    cmd.arg("-Dorg.lwjgl.util.Debug=false");
                    cmd.arg("-Dorg.lwjgl.util.DebugFunctions=false");
                    cmd.arg("-Dorg.lwjgl.util.DebugLoader=false");
                    cmd.arg("-Dorg.lwjgl.util.DebugStack=false");
                    cmd.arg("-Dorg.lwjgl.util.NoChecks=true");
                    cmd.arg("-Dorg.lwjgl.system.jemalloc=false");

                    // OpenGL dispatch fixes
                    cmd.arg("-Dorg.lwjgl.opengl.maxVersion=4.1");
                    cmd.arg("-Dorg.lwjgl.opengl.explicitInit=true");
                    cmd.arg("-Dorg.lwjgl.system.SharedLibraryExtractDirectory=/tmp/lwjgl");

                    // Memory and string handling fixes for dispatch.c
                    cmd.arg("-Dorg.lwjgl.system.ExplicitInit=true");
                    cmd.arg("-Dorg.lwjgl.system.CheckFunctionAddress=false");

                    // Force system OpenGL on macOS
                    cmd.arg(
                        "-Dorg.lwjgl.opengl.libname=/System/Library/Frameworks/OpenGL.framework/OpenGL",
                    );

                    // Additional macOS compatibility
                    cmd.arg("-Dcom.apple.mrj.application.apple.menu.about.name=Minecraft");
                    cmd.arg("-Dcom.apple.macos.useScreenMenuBar=false");
                }

                // ARM64 specific fixes (apply to all versions)
                if std::env::consts::ARCH == "aarch64" {
                    if !is_legacy_version && !is_java_8 {
                        // Disable problematic LWJGL features on ARM64
                        cmd.arg("-Dorg.lwjgl.system.macosx.bundleLookup=false");
                        cmd.arg("-Dorg.lwjgl.system.macosx.loadLibrary=system");

                        // Java module system fixes for ARM64 (Only for Java 9+)
                        cmd.arg("--add-opens");
                        cmd.arg("java.base/java.nio=ALL-UNNAMED");
                        cmd.arg("--add-opens");
                        cmd.arg("java.base/sun.nio.ch=ALL-UNNAMED");
                        cmd.arg("--add-opens");
                        cmd.arg("java.base/java.lang=ALL-UNNAMED");
                        cmd.arg("--add-opens");
                        cmd.arg("java.base/java.lang.reflect=ALL-UNNAMED");
                        cmd.arg("--add-opens");
                        cmd.arg("java.base/java.util=ALL-UNNAMED");
                        cmd.arg("--add-opens");
                        cmd.arg("java.base/sun.security.util=ALL-UNNAMED");
                    } else if is_java_8 {
                        // Java 8 specific ARM64 fixes without module system args
                        cmd.arg("-Dorg.lwjgl.system.macosx.bundleLookup=false");
                        cmd.arg("-Dorg.lwjgl.system.macosx.loadLibrary=system");
                    }

                    // Force system library loading
                    if natives_empty {
                        if is_legacy_version {
                            cmd.arg("-Djava.library.path=/System/Library/Frameworks");
                        } else {
                            cmd.arg("-Djava.library.path=/usr/lib:/System/Library/Frameworks");
                        }
                    } else {
                        // Always set natives path for ARM64
                        cmd.arg(format!(
                            "-Djava.library.path={}",
                            self.natives_dir.display()
                        ));
                    }
                }
            }
            "windows" => {
                cmd.arg("-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump");
                cmd.arg("-Dos.name=Windows 10");
                cmd.arg("-Dos.version=10.0");
                // Windows OpenGL compatibility
                cmd.arg("-Dorg.lwjgl.opengl.Window.undecorated=false");
            }
            _ => {
                // Linux OpenGL and compatibility settings
                cmd.arg("-Dorg.lwjgl.opengl.libname=libGL.so.1");
                cmd.arg("-Dorg.lwjgl.system.SharedLibraryExtractPath=/tmp/lwjgl");
            }
        }

        // Critical compatibility settings for modern Minecraft
        cmd.arg("-Dfml.ignoreInvalidMinecraftCertificates=true");
        cmd.arg("-Dfml.ignorePatchDiscrepancies=true");
        cmd.arg("-Dlog4j2.formatMsgNoLookups=true"); // Log4j security fix
        cmd.arg("-Djava.rmi.server.useCodebaseOnly=true");
        cmd.arg("-Dcom.sun.jndi.rmi.object.trustURLCodebase=false");
        cmd.arg("-Dcom.sun.jndi.cosnaming.object.trustURLCodebase=false");

        // LWJGL library path is already set above in platform-specific sections
        if !natives_empty {
            // Additional LWJGL system properties
            cmd.arg(format!(
                "-Dorg.lwjgl.librarypath={}",
                self.natives_dir.display()
            ));
        } else {
            // If no natives, use system libraries
            cmd.arg("-Dorg.lwjgl.system.allocator=system");
        }

        // Additional modern Java compatibility (only for Java 9+ and Minecraft 1.17+)
        let is_modern_mc = self.version_details.id >= "1.17".to_string();
        let is_using_java_8 = self.java_major_version == 8;

        if is_modern_mc && !is_using_java_8 {
            cmd.arg("--add-exports");
            cmd.arg("java.base/sun.security.util=ALL-UNNAMED");
            cmd.arg("--add-exports");
            cmd.arg("java.base/sun.security.x509=ALL-UNNAMED");
        }
    }
}

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

    pub fn java_major_version(mut self, java_major_version: u8) -> Self {
        self.java_major_version = Some(java_major_version);
        self
    }

    pub fn use_rosetta(mut self, use_rosetta: bool) -> Self {
        self.use_rosetta = use_rosetta;
        self
    }

    pub fn build(self) -> Result<MinecraftCommand> {
        Ok(MinecraftCommand::new(
            self.java_path
                .ok_or_else(|| anyhow::anyhow!("Java path not set"))?,
            self.game_dir
                .ok_or_else(|| anyhow::anyhow!("Game directory not set"))?,
            self.version_details
                .ok_or_else(|| anyhow::anyhow!("Version details not set"))?,
            self.username.unwrap_or_else(|| "Player".to_string()),
            self.uuid
                .unwrap_or_else(|| "00000000-0000-0000-0000-000000000000".to_string()),
            self.access_token.unwrap_or_else(|| "null".to_string()),
            self.user_type,
            self.version_type,
            self.assets_dir
                .ok_or_else(|| anyhow::anyhow!("Assets directory not set"))?,
            self.libraries,
            self.main_jar
                .ok_or_else(|| anyhow::anyhow!("Main jar not set"))?,
            self.java_major_version.unwrap_or(8),
            self.use_rosetta,
        ))
    }
}

impl Default for CommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}
