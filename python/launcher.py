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
        # Handle snapshot versions (e.g., 25w35a)
        if 'w' in minecraft_version and minecraft_version[0:2].isdigit():
            # Extract year from snapshot (e.g., "25" from "25w35a")
            year = int(minecraft_version[0:2])
            # Snapshots from 2023 (23w) and later support ARM64 natively
            return year < 23
        
        # Versions before 1.20.2 need Rosetta on Apple Silicon
        return pkg_version.parse(minecraft_version) < pkg_version.parse("1.20.2")
    except:
        # If version parsing fails, assume it needs Rosetta for safety
        return True

# Install Minecraft version.
def install_minecraft_version(version, minecraft_directory):
    """Install Minecraft version"""
    try:
        
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

# Launch Minecraft with log streaming
def launch_minecraft(username, version, minecraft_directory, game_dir=None):
    """Launch Minecraft and stream logs to stdout"""
    try:
        # Generate Minecraft launch command using minecraft_launcher_lib
        options = {
            "username": username,
            "uuid": str(uuid.uuid4()),
            "token": "dummy_token",
            "gameDirectory": game_dir or minecraft_directory,
            "jvmArguments": ["-Xmx2G", "-Xms1G"]
        }
        
        # For older versions that need Rosetta, use x86_64 Java
        if needs_rosetta(version) and is_apple_silicon():
            # Use x86_64 Java 8 for older Minecraft versions
            java_8_path = "/Library/Java/JavaVirtualMachines/jdk1.8.0_351.jdk/Contents/Home/bin/java"
            if Path(java_8_path).exists():
                options["executablePath"] = java_8_path
                logging.info(f"Using x86_64 Java 8 for {version}")
            else:
                logging.warning(f"x86_64 Java 8 not found, using system Java with Rosetta")
        
        command = minecraft_launcher_lib.command.get_minecraft_command(
            version, minecraft_directory, options
        )
        
        # Check if Rosetta is needed for older versions on Apple Silicon
        if needs_rosetta(version):
            logging.info(f"Launching {version} with Rosetta compatibility")
            # Prepend arch -x86_64 to the entire command
            command = ["arch", "-x86_64"] + command

        logging.info(f"Launching Minecraft {version} for user {username}")
        logging.info(f"Command: {' '.join(command)}")

        # Launch Minecraft with stdout/stderr capture
        process = subprocess.Popen(
            command, 
            stdout=subprocess.PIPE, 
            stderr=subprocess.PIPE,  # Separate stderr to capture all logs
            text=True, 
            bufsize=0,  # Unbuffered for real-time logs
            universal_newlines=True,
            shell=False
        )
        
        # Send initial success message
        print(json.dumps({
            "type": "launch_result",
            "success": True,
            "pid": process.pid,
            "message": f"Minecraft {version} launched successfully"
        }), flush=True)
        
        # Stream logs in real-time from both stdout and stderr
        import threading
        
        def read_stdout():
            try:
                for line in iter(process.stdout.readline, ''):
                    if line:
                        print(json.dumps({
                            "type": "log",
                            "line": line.strip(),
                            "pid": process.pid
                        }), flush=True)
            except Exception as e:
                logging.error(f"Error reading stdout: {e}")
        
        def read_stderr():
            try:
                for line in iter(process.stderr.readline, ''):
                    if line:
                        print(json.dumps({
                            "type": "log",
                            "line": f"[STDERR] {line.strip()}",
                            "pid": process.pid
                        }), flush=True)
            except Exception as e:
                logging.error(f"Error reading stderr: {e}")
        
        # Start reading threads
        stdout_thread = threading.Thread(target=read_stdout)
        stderr_thread = threading.Thread(target=read_stderr)
        stdout_thread.daemon = True
        stderr_thread.daemon = True
        stdout_thread.start()
        stderr_thread.start()

        # Wait for process to complete and get exit code
        exit_code = process.wait()
        
        # Wait for reading threads to complete
        stdout_thread.join(timeout=5)
        stderr_thread.join(timeout=5)
        
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

    if command == "install" and len(sys.argv) == 4:
        # Install version
        version = sys.argv[2]
        minecraft_dir = sys.argv[3]
        success = install_minecraft_version(version, minecraft_dir)
        result = {"success": success}
        print(json.dumps(result))
        if not success:
            exit(1)
    elif command == "launch" and len(sys.argv) == 6:
        # Launch Minecraft with log streaming
        username = sys.argv[2]
        version = sys.argv[3]
        minecraft_dir = sys.argv[4]
        game_dir = sys.argv[5]
        exit_code = launch_minecraft(username, version, minecraft_dir, game_dir)
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
