#!/usr/bin/env python3

# Minecraft launcher (temporary for beta versions).

import minecraft_launcher_lib
import subprocess
import uuid
import argparse
import sys
import json
import requests
import os
import platform
import shutil
import logging
from pathlib import Path

# Configure logging.
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
from packaging import version as pkg_version

# Minecraft directory.
def get_minecraft_directory():
    return minecraft_launcher_lib.utils.get_minecraft_directory()

# Get version manifest from Mojang.
def get_version_manifest():
    try:
        response = requests.get("https://launchermeta.mojang.com/mc/game/version_manifest.json")
        response.raise_for_status()
        return response.json()
    except Exception as e:
        logging.error(f"Error fetching version manifest: {e}")
        return None

# Get available versions.
def get_available_versions():
    manifest = get_version_manifest()
    if manifest:
        return [version["id"] for version in manifest["versions"]]
    return []

# Get full version manifest in JSON format.
def get_full_manifest():
    manifest = get_version_manifest()
    if manifest:
        return json.dumps(manifest, indent=2)
    return json.dumps({"latest": {"release": "1.21.4", "snapshot": "24w51a"}, "versions": []})

def is_apple_silicon():
    """Check if running on Apple Silicon Mac"""
    return platform.system() == "Darwin" and platform.machine() == "arm64"

def needs_lwjgl_fix(minecraft_version):
    """Check if Minecraft version needs LWJGL Apple Silicon fix"""
    if not is_apple_silicon():
        return False
    
    try:
        # Versions before 1.19 need LWJGL fix on Apple Silicon
        return pkg_version.parse(minecraft_version) < pkg_version.parse("1.19")
    except:
        # If version parsing fails, assume it needs fix for safety
        return True

def download_arm64_lwjgl_libraries(minecraft_directory, minecraft_version):
    """Download and replace LWJGL libraries with Apple Silicon compatible versions"""
    try:
        libraries_dir = Path(minecraft_directory) / "libraries" / "org" / "lwjgl"
        
        # LWJGL 3.3.0 libraries with ARM64 support
        lwjgl_libraries = {
            "lwjgl": "3.3.0",
            "lwjgl-glfw": "3.3.0", 
            "lwjgl-jemalloc": "3.3.0",
            "lwjgl-openal": "3.3.0",
            "lwjgl-opengl": "3.3.0",
            "lwjgl-stb": "3.3.0",
            "lwjgl-tinyfd": "3.3.0"
        }
        
        for lib_name, lib_version in lwjgl_libraries.items():
            # Download ARM64 native library
            native_url = f"https://repo1.maven.org/maven2/org/lwjgl/{lib_name}/{lib_version}/{lib_name}-{lib_version}-natives-macos-arm64.jar"
            
            # Create library directory structure
            lib_dir = libraries_dir / lib_name / lib_version
            lib_dir.mkdir(parents=True, exist_ok=True)
            
            # Download native library
            native_file = lib_dir / f"{lib_name}-{lib_version}-natives-macos-arm64.jar"
            
            if not native_file.exists():
                logging.info(f"Downloading ARM64 native library: {lib_name}")
                response = requests.get(native_url)
                if response.status_code == 200:
                    with open(native_file, 'wb') as f:
                        f.write(response.content)
                    logging.info(f"Downloaded: {native_file}")
                else:
                    logging.error(f"Failed to download {native_url}: {response.status_code}")
        
        logging.info(f"ARM64 LWJGL libraries prepared for Minecraft {minecraft_version}")
        return True
        
    except Exception as e:
        logging.error(f"Error downloading ARM64 LWJGL libraries: {e}")
        return False

def patch_version_json_for_arm64(minecraft_directory, minecraft_version):
    """Patch version JSON to use ARM64 LWJGL libraries"""
    try:
        version_json_path = Path(minecraft_directory) / "versions" / minecraft_version / f"{minecraft_version}.json"
        
        if not version_json_path.exists():
            logging.error(f"Version JSON not found: {version_json_path}")
            return False
            
        # Read version JSON
        with open(version_json_path, 'r') as f:
            version_data = json.load(f)
        
        # Update LWJGL libraries to use ARM64 natives
        if 'libraries' in version_data:
            for library in version_data['libraries']:
                if 'name' in library and library['name'].startswith('org.lwjgl:'):
                    lib_name = library['name'].split(':')[1]  # Extract library name
                    
                    # Update natives mapping
                    if 'natives' in library and 'osx' in library['natives']:
                        library['natives']['osx'] = 'natives-macos-arm64'
                    
                    # Update downloads to point to ARM64 versions
                    if 'downloads' in library and 'classifiers' in library['downloads']:
                        classifiers = library['downloads']['classifiers']
                        
                        # Remove old macOS natives and add ARM64 version
                        if 'natives-macos' in classifiers:
                            old_classifier = classifiers['natives-macos']
                            
                            # Create new ARM64 classifier
                            arm64_classifier = {
                                'path': old_classifier['path'].replace('natives-macos.jar', 'natives-macos-arm64.jar'),
                                'sha1': '',  # Will be updated when downloaded
                                'size': 0,   # Will be updated when downloaded
                                'url': f"https://repo1.maven.org/maven2/org/lwjgl/{lib_name}/3.3.0/{lib_name}-3.3.0-natives-macos-arm64.jar"
                            }
                            
                            # Replace the classifier
                            classifiers['natives-macos-arm64'] = arm64_classifier
                            del classifiers['natives-macos']
        
        # Write back the modified JSON
        with open(version_json_path, 'w') as f:
            json.dump(version_data, f, indent=2)
        
        logging.info(f"Patched version JSON for ARM64: {version_json_path}")
        return True
        
    except Exception as e:
        logging.error(f"Error patching version JSON: {e}")
        return False

# Install Minecraft version.
def install_minecraft_version(version="1.20.1"):
    try:
        minecraft_directory = get_minecraft_directory()
        
        # For Apple Silicon and older versions, use custom installation process
        if is_apple_silicon() and needs_lwjgl_fix(version):
            logging.info(f"Installing Minecraft {version} with Apple Silicon compatibility...")
            
            # Use forge installation method which is more flexible
            try:
                # Install without natives first
                minecraft_launcher_lib.install.install_minecraft_version(
                    version, 
                    minecraft_directory,
                    callback={"setStatus": lambda x: None, "setProgress": lambda x: None, "setMax": lambda x: None}
                )
            except Exception as install_error:
                if 'natives-macos-arm64' in str(install_error):
                    logging.info(f"Handling ARM64 natives compatibility...")
                    
                    # Manually install base version files
                    try:
                        # Get version manifest
                        manifest_url = "https://launchermeta.mojang.com/mc/game/version_manifest.json"
                        response = requests.get(manifest_url)
                        if response.status_code != 200:
                            logging.error(f"Failed to get version manifest: {response.status_code}")
                            return False
                        
                        version_manifest = response.json()
                        version_info = None
                        for v in version_manifest['versions']:
                            if v['id'] == version:
                                version_info = v
                                break
                        
                        if not version_info:
                            logging.error(f"Version {version} not found in manifest")
                            return False
                        
                        # Download version JSON
                        version_dir = Path(minecraft_directory) / "versions" / version
                        version_dir.mkdir(parents=True, exist_ok=True)
                        
                        version_json_path = version_dir / f"{version}.json"
                        if not version_json_path.exists():
                            response = requests.get(version_info['url'])
                            if response.status_code == 200:
                                with open(version_json_path, 'w') as f:
                                    json.dump(response.json(), f, indent=2)
                        
                        # Download JAR file
                        jar_path = version_dir / f"{version}.jar"
                        if not jar_path.exists():
                            with open(version_json_path, 'r') as f:
                                version_data = json.load(f)
                            
                            if 'downloads' in version_data and 'client' in version_data['downloads']:
                                jar_url = version_data['downloads']['client']['url']
                                response = requests.get(jar_url)
                                if response.status_code == 200:
                                    with open(jar_path, 'wb') as f:
                                        f.write(response.content)
                        
                        # Apply LWJGL fix
                        download_arm64_lwjgl_libraries(minecraft_directory, version)
                        patch_version_json_for_arm64(minecraft_directory, version)
                        
                        logging.info(f"Successfully installed {version} with ARM64 compatibility")
                        return True
                        
                    except Exception as manual_error:
                        logging.error(f"Manual installation failed: {manual_error}")
                        return False
                else:
                    raise install_error
        else:
            # Normal installation for newer versions or non-Apple Silicon
            minecraft_launcher_lib.install.install_minecraft_version(version, minecraft_directory)
        
        logging.info(f"Version {version} installed successfully")
        return True
    except Exception as e:
        logging.error(f"Error installing version {version}: {e}")
        return False

def check_and_apply_lwjgl_fix(minecraft_directory, minecraft_version):
    """Check if LWJGL fix is needed and apply it if necessary"""
    if not needs_lwjgl_fix(minecraft_version):
        return True
    
    # Check if ARM64 libraries already exist
    libraries_dir = Path(minecraft_directory) / "libraries" / "org" / "lwjgl" / "lwjgl" / "3.3.0"
    arm64_lib = libraries_dir / "lwjgl-3.3.0-natives-macos-arm64.jar"
    
    if not arm64_lib.exists():
        logging.info(f"ARM64 LWJGL libraries not found, downloading for version {minecraft_version}...")
        if not download_arm64_lwjgl_libraries(minecraft_directory, minecraft_version):
            return False
        if not patch_version_json_for_arm64(minecraft_directory, minecraft_version):
            return False
    else:
        logging.info(f"ARM64 LWJGL libraries already available for version {minecraft_version}")
    
    return True

# Launch Minecraft.
def launch_minecraft(username="TestPlayer", version="1.20.1"):
    try:
        minecraft_directory = get_minecraft_directory()
        
        # Apply Apple Silicon LWJGL fix if needed before launch
        if not check_and_apply_lwjgl_fix(minecraft_directory, version):
            return {
                "success": False,
                "pid": None,
                "message": f"Failed to apply Apple Silicon compatibility fix for version {version}"
            }

        # Create launch options
        options = {
            "username": username,
            "uuid": str(uuid.uuid4()),
            "token": "",  # Offline mode
        }

        # Get launch command
        command = minecraft_launcher_lib.command.get_minecraft_command(
            version=version,
            minecraft_directory=minecraft_directory,
            options=options
        )

        logging.info(f"Launching Minecraft {version} for user {username}")

        # Launch Minecraft
        process = subprocess.Popen(command)
        logging.info(f"Minecraft launched with PID: {process.pid}")

        return {
            "success": True,
            "pid": process.pid,
            "message": f"Minecraft {version} launched successfully"
        }

    except Exception as e:
        error_msg = f"Error launching Minecraft: {e}"
        logging.error(error_msg)
        return {
            "success": False,
            "pid": None,
            "message": error_msg
        }

def main():
    parser = argparse.ArgumentParser(description='Minecraft Launcher')
    parser.add_argument('--username', '-u', default='TestPlayer', help='Username for Minecraft')
    parser.add_argument('--version', '-v', default='1.20.1', help='Minecraft version to launch')
    parser.add_argument('--install', '-i', action='store_true', help='Install version before launching')
    parser.add_argument('--list-versions', '-l', action='store_true', help='List available versions')
    parser.add_argument('--get-manifest', '-m', action='store_true', help='Get full version manifest as JSON')
    
    args = parser.parse_args()
    
    if args.get_manifest:
        manifest_json = get_full_manifest()
        print(manifest_json)
        return
    
    if args.list_versions:
        print("Available Minecraft versions:")
        versions = get_available_versions()
        if versions:
            for version in versions[:20]:  # Show first 20 versions
                print(f"  {version}")
            if len(versions) > 20:
                print(f"  ... and {len(versions) - 20} more versions")
        else:
            print("  Failed to fetch versions")
        return
    
    # Install version if requested
    if args.install:
        logging.info(f"Installing Minecraft version {args.version}...")
        if not install_minecraft_version(args.version):
            logging.error(f"Failed to install version {args.version}")
            sys.exit(1)
    
    # Launch Minecraft
    logging.info(f"Launching Minecraft {args.version} for user {args.username}...")
    result = launch_minecraft(args.username, args.version)
    
    # Output result as JSON for bridge compatibility
    print(json.dumps(result))
    
    if not result["success"]:
        sys.exit(1)

if __name__ == "__main__":
    main()
