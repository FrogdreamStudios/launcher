//! Windows updater helper that replaces the main executable after it exits.
//! This is necessary because Windows locks running executables and prevents replacement.

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} <temp_file_path> <target_exe_path>", args[0]);
        std::process::exit(1);
    }
    
    let temp_file = &args[1];
    let target_exe = &args[2];
    
    println!("Windows Updater Helper");
    println!("Temp file: {}", temp_file);
    println!("Target executable: {}", target_exe);
    
    // Wait for the main process to exit
    println!("Waiting for main process to exit...");
    thread::sleep(Duration::from_secs(2));
    
    // Try to replace the executable multiple times if needed
    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 10;
    
    while attempts < MAX_ATTEMPTS {
        match replace_executable(temp_file, target_exe) {
            Ok(_) => {
                println!("Successfully replaced executable!");
                break;
            }
            Err(e) => {
                attempts += 1;
                eprintln!("Attempt {}/{} failed: {}", attempts, MAX_ATTEMPTS, e);
                
                if attempts < MAX_ATTEMPTS {
                    println!("Retrying in 1 second...");
                    thread::sleep(Duration::from_secs(1));
                } else {
                    eprintln!("Failed to replace executable after {} attempts", MAX_ATTEMPTS);
                    std::process::exit(1);
                }
            }
        }
    }
    
    // Clean up temp file
    if let Err(e) = fs::remove_file(temp_file) {
        eprintln!("Warning: Failed to remove temp file {}: {}", temp_file, e);
    }
    
    // Start the updated application
    println!("Starting updated application...");
    match Command::new(target_exe).spawn() {
        Ok(_) => {
            println!("Successfully started updated application");
        }
        Err(e) => {
            eprintln!("Failed to start updated application: {}", e);
            std::process::exit(1);
        }
    }
}

fn replace_executable(temp_file: &str, target_exe: &str) -> Result<(), String> {
    // Check if temp file exists
    if !Path::new(temp_file).exists() {
        return Err(format!("Temp file does not exist: {}", temp_file));
    }
    
    // Check if target executable exists
    if !Path::new(target_exe).exists() {
        return Err(format!("Target executable does not exist: {}", target_exe));
    }
    
    // Create backup of current executable
    let backup_path = format!("{}.backup", target_exe);
    if let Err(e) = fs::copy(target_exe, &backup_path) {
        return Err(format!("Failed to create backup: {}", e));
    }
    
    // Try to replace the executable
    match fs::copy(temp_file, target_exe) {
        Ok(_) => {
            // Remove backup on success
            let _ = fs::remove_file(&backup_path);
            Ok(())
        }
        Err(e) => {
            // Restore backup on failure
            if let Err(restore_err) = fs::copy(&backup_path, target_exe) {
                eprintln!("Critical error: Failed to restore backup: {}", restore_err);
            }
            let _ = fs::remove_file(&backup_path);
            Err(format!("Failed to replace executable: {}", e))
        }
    }
}