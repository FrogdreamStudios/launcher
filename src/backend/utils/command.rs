//! Minecraft command building utilities.
//!
//! Tools and utilities for constructing Java commands to launch Minecraft.
//! JVM arguments, game arguments, classpath construction, variable substitution, platform-specific
//! configurations (such as Rosetta support on macOS), and compatibility settings for different
//! Minecraft versions (from legacy pre-1.6 to modern 1.17+).
//!
//! #### Example:
//! ```rust
//! use anyhow::Result;
//! use std::path::PathBuf;
//! use crate::backend::creeper::models::VersionDetails;
//! use crate::backend::command::CommandBuilder;
//!
//! fn main() -> Result<()> {
//!     let builder = CommandBuilder::new()
//!         .java_path(PathBuf::from("/path/to/java"))
//!         .game_dir(PathBuf::from("/path/to/.minecraft"))
//!         .version_details(VersionDetails { /* ... */ })
//!         .username("Player".to_string())
//!         .uuid("uuid-here".to_string())
//!         .access_token("token-here".to_string())
//!         .assets_dir(PathBuf::from("/path/to/assets"))
//!         .libraries(vec![/* library paths */])
//!         .main_jar(PathBuf::from("/path/to/minecraft.jar"))
//!         .java_major_version(17)
//!         .use_rosetta(false);
//!
//!     let mc_command = builder.build()?;
//!     let command = mc_command.build()?;
//!     command.spawn()?;
//!     Ok(())
//! }
//! ```

use anyhow::Result;
use std::{path::PathBuf, process::Command};

use crate::backend::{
    creeper::models::{ArgumentValue, ArgumentValueInner, VersionDetails},
    utils::{
        os::{get_minecraft_arch, get_minecraft_os_name, get_os_features},
        paths::{get_classpath_separator, get_natives_dir},
    },
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Configuration structure for building Minecraft commands.
pub struct CommandConfig {
    pub java_path: PathBuf, // The absolute path to the Java executable
    pub game_dir: PathBuf,  // The Minecraft game directory
    pub version_details: VersionDetails, // Detailed information about the Minecraft version being launched
    pub username: String, // The player's nickname (defaults to "Player" in offline mode)
    pub uuid: String,     // The player's UUID (defaults to a zero UUID in offline mode)
    pub access_token: String, // Authentication token for online mode (defaults to "null" in offline mode)
    pub user_type: String,    // Type of user account (e.g., "mojang" or "msa" for Microsoft)
    pub version_type: String, // Type of version (e.g., "release", "snapshot")
    pub assets_dir: PathBuf,  // Directory for Minecraft assets
    pub libraries: Vec<PathBuf>, // List of library JAR paths to include in the classpath
    pub main_jar: PathBuf,    // Path to the main Minecraft JAR file
    pub java_major_version: u8, // Major version of Java being used for compatibility adjustments
    pub use_rosetta: bool, // Flag to enable Rosetta 2 emulation on macOS ARM64 for x86_64 compatibility
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
    /// Initializes the command builder with the provided config and computes the natives directory.
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

            Command::new(&script_path)
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

        // Set the working directory
        cmd.current_dir(&self.game_dir);

        Ok(cmd)
    }

    /// Adds JVM arguments to the command.
    fn add_jvm_arguments(&self, cmd: &mut Command) -> Result<()> {
        // Add classpath
        let classpath = self.build_classpath();
        cmd.arg("-cp");
        cmd.arg(classpath);

        // Add launcher name and version
        cmd.arg("-Dminecraft.launcher.brand=DreamLauncher");
        cmd.arg("-Dminecraft.launcher.version=1.0.0-beta.1");

        // Add default JVM arguments
        self.add_default_jvm_args(cmd);

        // Add version-specific JVM arguments
        if let Some(arguments) = &self.version_details.arguments {
            for arg in &arguments.jvm {
                self.process_argument(cmd, arg, true);
            }
        }

        Ok(())
    }

    /// Adds game arguments to the command.
    ///
    /// Handles both modern (post-1.13 with structured arguments) and legacy (pre-1.13 with string-based arguments) formats.
    /// For legacy, it substitutes variables and parses the string while preserving quoted parts.
    fn add_game_arguments(&self, cmd: &mut Command) -> Result<()> {
        if let Some(arguments) = &self.version_details.arguments {
            // Modern argument format (1.13+)
            for arg in &arguments.game {
                self.process_argument(cmd, arg, false);
            }
        } else if let Some(minecraft_arguments) = &self.version_details.minecraft_arguments {
            // Legacy argument format (pre-1.13)
            let args = self.substitute_legacy_arguments(minecraft_arguments);

            // Parse arguments while preserving quoted strings
            let mut current_arg = String::new();
            let mut in_quotes = false;
            let chars = args.chars().peekable();

            for ch in chars {
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

    /// Processes a single argument, handling strings and conditionals.
    ///
    /// Substitutes variables and adds to the command if the argument is a string or if conditional rules evaluate to true.
    fn process_argument(&self, cmd: &mut Command, arg: &ArgumentValue, is_jvm: bool) {
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
    fn evaluate_rules(&self, rules: &[crate::backend::creeper::models::Rule]) -> bool {
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

    /// Substitutes variables in a string with actual values.
    fn substitute_variables(&self, input: &str, is_jvm: bool) -> String {
        let mut result = input.to_string();

        // Game directory.
        // WARNING: do NOT quote this path
        let game_dir_str = self.game_dir.display().to_string();
        result = result.replace("${game_directory}", &game_dir_str);

        // Assets
        // WARNING: do NOT quote this path
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
                assets_dir_str
            }
        } else if self.version_details.id.as_str() >= "1.7" {
            // For 1.7+ versions, use virtual assets
            let virtual_assets_path = self
                .assets_dir
                .join("virtual")
                .join(&self.version_details.assets);
            virtual_assets_path.display().to_string()
        } else {
            // For other versions, use regular assets directory
            assets_dir_str
        };
        result = result.replace("${game_assets}", &game_assets_str);

        // User info
        result = result.replace("${auth_player_name}", &self.username);
        result = result.replace("${auth_uuid}", &self.uuid);
        result = result.replace("${auth_access_token}", &self.access_token);
        result = result.replace("${user_type}", &self.user_type);

        // Legacy auth session support (for very old versions like 1.0-1.5.x)
        result = result.replace("${auth_session}", &self.access_token);

        // User properties. Minecraft expects this as a JSON object
        // For offline mode, we provide an empty JSON object
        result = result.replace("${user_properties}", "{}");

        // Version info
        result = result.replace("${version_name}", &self.version_details.id);
        result = result.replace("${version_type}", &self.version_type);

        // Launcher info
        result = result.replace("${launcher_name}", "DreamLauncher");
        result = result.replace("${launcher_version}", "1.0.0-beta.1");

        // Natives directory (JVM only)
        // WARNING: do NOT quote this path
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

    /// Substitutes variables in legacy argument strings.
    /// Similar to `substitute_variables`, but specifically for legacy arguments.
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

        // User properties. Minecraft expects this as a JSON object
        // For offline mode, we provide an empty JSON object
        result = result.replace("${user_properties}", "{}");

        result
    }

    /// Combines library paths and the main JAR, joined by the platform-specific separator.
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

    /// Adds default JVM arguments based on version and platform.
    fn add_default_jvm_args(&self, cmd: &mut Command) {
        // Determine optimal memory settings based on version
        let is_modern_version = self.version_details.id.as_str() >= "1.17";

        if is_modern_version {
            // Modern versions (1.17+) need more memory, so we will use these settings for our betas
            cmd.arg("-Xms128M");
            cmd.arg("-Xmx4096M");
        } else {
            // Older versions (pre-1.17) need less memory, so we will use these settings
            cmd.arg("-Xms512M");
            cmd.arg("-Xmx2G");
        }

        // Conservative GC settings
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

                // Always add -XstartOnFirstThread for 1.13+ versions
                // For legacy versions, only add if not using Rosetta (to avoid window display issues)
                if !is_legacy_version || !self.use_rosetta {
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
                    // Modern version approach (1.13+) - minimal flags for better compatibility
                    cmd.arg("-Djava.awt.headless=false");
                    cmd.arg("-Dapple.awt.application.name=Minecraft");

                    // Only essential LWJGL settings for modern versions
                    if !natives_empty {
                        cmd.arg(format!(
                            "-Djava.library.path={}",
                            self.natives_dir.display()
                        ));
                    }
                }

                // ARM64 specific fixes (apply to all versions)
                if std::env::consts::ARCH == "aarch64" {
                    if is_legacy_version {
                        // Legacy versions need special handling on ARM64
                        cmd.arg("-Dorg.lwjgl.system.macosx.bundleLookup=false");
                        cmd.arg("-Dorg.lwjgl.system.macosx.loadLibrary=system");

                        // Force system library loading for legacy versions
                        if natives_empty {
                            cmd.arg("-Djava.library.path=/System/Library/Frameworks");
                        } else {
                            cmd.arg(format!(
                                "-Djava.library.path={}",
                                self.natives_dir.display()
                            ));
                        }
                    } else {
                        // Versions (1.13+) - minimal ARM64 fixes, let Minecraft handle the rest
                        if !is_java_8 {
                            // Only essential module system fixes for modern versions with Java 9+
                            cmd.arg("--add-opens");
                            cmd.arg("java.base/java.nio=ALL-UNNAMED");
                            cmd.arg("--add-opens");
                            cmd.arg("java.base/sun.nio.ch=ALL-UNNAMED");
                        }
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
        let is_modern_mc = self.version_details.id.as_str() >= "1.17";
        let is_using_java_8 = self.java_major_version == 8;

        if is_modern_mc && !is_using_java_8 {
            cmd.arg("--add-exports");
            cmd.arg("java.base/sun.security.util=ALL-UNNAMED");
            cmd.arg("--add-exports");
            cmd.arg("java.base/sun.security.x509=ALL-UNNAMED");
        }
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

    /// Sets the Java executable path.
    pub fn java_path(mut self, path: PathBuf) -> Self {
        self.java_path = Some(path);
        self
    }

    /// Sets the game directory.
    pub fn game_dir(mut self, dir: PathBuf) -> Self {
        self.game_dir = Some(dir);
        self
    }

    /// Sets the version details.
    pub fn version_details(mut self, details: VersionDetails) -> Self {
        self.version_details = Some(details);
        self
    }

    /// Sets the username.
    pub fn username(mut self, name: String) -> Self {
        self.username = Some(name);
        self
    }

    /// Sets the UUID.
    pub fn uuid(mut self, id: String) -> Self {
        self.uuid = Some(id);
        self
    }

    /// Sets the access token.
    pub fn access_token(mut self, token: String) -> Self {
        self.access_token = Some(token);
        self
    }

    /// Sets the user type.
    pub fn user_type(mut self, user_type: String) -> Self {
        self.user_type = user_type;
        self
    }

    /// Sets the version type (e.g., "release", "snapshot").
    pub fn version_type(mut self, version_type: String) -> Self {
        self.version_type = version_type;
        self
    }

    /// Sets the assets directory.
    pub fn assets_dir(mut self, dir: PathBuf) -> Self {
        self.assets_dir = Some(dir);
        self
    }

    /// Sets the list of library paths.
    pub fn libraries(mut self, libs: Vec<PathBuf>) -> Self {
        self.libraries = libs;
        self
    }

    /// Sets the main JAR path.
    pub fn main_jar(mut self, main_jar: PathBuf) -> Self {
        self.main_jar = Some(main_jar);
        self
    }

    /// Sets the Java major version.
    pub const fn java_major_version(mut self, java_major_version: u8) -> Self {
        self.java_major_version = Some(java_major_version);
        self
    }

    /// Sets whether to use Rosetta on macOS.
    pub const fn use_rosetta(mut self, use_rosetta: bool) -> Self {
        self.use_rosetta = use_rosetta;
        self
    }

    /// Builds a `MinecraftCommand` from the configured parameters.
    ///
    /// Validates that all required fields are set, applies defaults for optional ones,
    /// and constructs the `CommandConfig` to pass to `MinecraftCommand::new`.
    pub fn build(self) -> Result<MinecraftCommand> {
        let config = CommandConfig {
            java_path: self
                .java_path
                .ok_or_else(|| anyhow::anyhow!("Java path not set"))?,
            game_dir: self
                .game_dir
                .ok_or_else(|| anyhow::anyhow!("Game directory not set"))?,
            version_details: self
                .version_details
                .ok_or_else(|| anyhow::anyhow!("Version details not set"))?,
            username: self.username.unwrap_or_else(|| "Player".to_string()),
            uuid: self
                .uuid
                .unwrap_or_else(|| "00000000-0000-0000-0000-000000000000".to_string()),
            access_token: self.access_token.unwrap_or_else(|| "null".to_string()),
            user_type: self.user_type,
            version_type: self.version_type,
            assets_dir: self
                .assets_dir
                .ok_or_else(|| anyhow::anyhow!("Assets directory not set"))?,
            libraries: self.libraries,
            main_jar: self
                .main_jar
                .ok_or_else(|| anyhow::anyhow!("Main jar not set"))?,
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
