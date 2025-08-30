#!/usr/bin/env python3

# Minecraft launcher (temporary for beta versions).

import minecraft_launcher_lib
import subprocess
import uuid
import sys
import json
import platform
import logging
import urllib3
import requests
from pathlib import Path
from packaging import version as pkg_version

# HTTP timeouts.
requests.adapters.DEFAULT_RETRIES = 3
requests.adapters.DEFAULT_TIMEOUT = 30

# urllib3 timeouts.
urllib3.util.timeout.DEFAULT_TIMEOUT = 30

# Get Minecraft directory.
def get_minecraft_directory():
    return minecraft_launcher_lib.utils.get_minecraft_directory()

def is_apple_silicon():
    return platform.system() == "Darwin" and platform.machine() == "arm64"

def needs_rosetta(minecraft_version):
    if not is_apple_silicon():
        return False
    
    try:
        # Versions before 1.20.2 need Rosetta on Apple Silicon
        return pkg_version.parse(minecraft_version) < pkg_version.parse("1.20.2")
    except:
        # If version parsing fails, assume it needs Rosetta for safety
        return True

# Install Minecraft version.
def install_minecraft_version(version):
    """Install Minecraft version"""
    try:
        minecraft_directory = get_minecraft_directory()
        
        # Check if Rosetta is needed for older versions on Apple Silicon
        if needs_rosetta(version):
            logging.info(f"Version {version} requires Rosetta on Apple Silicon")
        
        # Configure session with timeout
        session = requests.Session()
        session.timeout = 30
        
        # Install the version using minecraft_launcher_lib with timeout
        minecraft_launcher_lib.install.install_minecraft_version(
            version, 
            minecraft_directory,
            callback={"setStatus": lambda x: None, "setProgress": lambda x: None, "setMax": lambda x: None}
        )
        
        logging.info(f"Version {version} installed successfully")
        return True
        
    except FileExistsError as e:
        # Handle the case where natives directory already exists
        if "META-INF" in str(e) and "natives" in str(e):
            logging.info(f"Version {version} natives already exist, installation completed")
            return True
        else:
            logging.error(f"File exists error installing version {version}: {e}")
            return False
    except Exception as e:
        logging.error(f"Error installing version {version}: {e}")
        return False

# Launch Minecraft.
def launch_minecraft(username, version):
    """Launch Minecraft with the specified version"""
    try:
        minecraft_directory = get_minecraft_directory()
        
        # Check if Rosetta is needed for older versions on Apple Silicon
        if needs_rosetta(version):
            logging.info(f"Launching {version} with Rosetta compatibility")
        
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
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
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

# Launch Minecraft with log streaming
def launch_minecraft_with_logs(username, version):
    """Launch Minecraft and stream logs to stdout"""
    try:
        minecraft_directory = get_minecraft_directory()
        
        # Check if Rosetta is needed for older versions on Apple Silicon
        if needs_rosetta(version):
            logging.info(f"Launching {version} with Rosetta compatibility")
        
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

        # Launch Minecraft with stdout/stderr capture
        process = subprocess.Popen(
            command, 
            stdout=subprocess.PIPE, 
            stderr=subprocess.STDOUT, 
            text=True, 
            bufsize=1,
            universal_newlines=True
        )
        
        # Send initial success message
        print(json.dumps({
            "type": "launch_result",
            "success": True,
            "pid": process.pid,
            "message": f"Minecraft {version} launched successfully"
        }), flush=True)
        
        # Stream logs in real-time
        try:
            for line in iter(process.stdout.readline, ''):
                if line:
                    # Send log line to Rust
                    print(json.dumps({
                        "type": "log",
                        "line": line.strip(),
                        "pid": process.pid
                    }), flush=True)
        except Exception as e:
            logging.error(f"Error reading logs: {e}")
        
        # Wait for process to complete and get exit code
        exit_code = process.wait()
        
        # Send final status
        print(json.dumps({
            "type": "exit",
            "pid": process.pid,
            "exit_code": exit_code,
            "message": f"Minecraft process exited with code {exit_code}"
        }), flush=True)
        
        return exit_code
        
    except Exception as e:
        error_msg = f"Error launching Minecraft: {e}"
        logging.error(error_msg)
        print(json.dumps({
            "type": "error",
            "success": False,
            "message": error_msg
        }), flush=True)
        return 1

# Entry point when called from Rust launcher.
if __name__ == "__main__":
    if len(sys.argv) < 2:
        logging.error("Usage: launcher.py <command> [args...]")
        print(json.dumps({"success": False, "error": "Invalid arguments"}))
        exit(1)

    command = sys.argv[1]

    if command == "install" and len(sys.argv) == 3:
        # Install version
        version = sys.argv[2]
        success = install_minecraft_version(version)
        result = {"success": success}
        print(json.dumps(result))
        if not success:
            exit(1)
    elif command == "launch" and len(sys.argv) == 4:
        # Launch Minecraft
        username = sys.argv[2]
        version = sys.argv[3]
        result = launch_minecraft(username, version)
        print(json.dumps(result))
        if not result["success"]:
            exit(1)
    elif command == "launch_with_logs" and len(sys.argv) == 4:
        # Launch Minecraft with log streaming
        username = sys.argv[2]
        version = sys.argv[3]
        exit_code = launch_minecraft_with_logs(username, version)
        exit(exit_code)
    elif command == "logs" and len(sys.argv) == 3:
        # Get logs from running process
        pid = int(sys.argv[2])
        result = get_minecraft_logs(pid)
        print(json.dumps(result))
    else:
        logging.error("Invalid command or arguments")
        print(json.dumps({"success": False, "error": "Invalid command"}))
        exit(1)
