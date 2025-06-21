use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::Manager; // Add path here
use log::{info, error};

const PID_FILE_NAME: &str = "dev-server.pid";

fn get_pid_file_path(app_handle: &tauri::AppHandle) -> Option<PathBuf> {
    if let Some(app_data_dir) = app_handle.path().app_local_data_dir().ok() {
        let pid_file_path = app_data_dir.join(PID_FILE_NAME);
        info!("[DEV_SERVER_MANAGER] PID file path: {}", pid_file_path.display());
        Some(pid_file_path)
    } else {
        error!("[DEV_SERVER_MANAGER] Could not determine app local data directory.");
        None
    }
}

pub fn stop_dev_server(app_handle: &tauri::AppHandle) {
    info!("[DEV_SERVER_MANAGER] Attempting to stop development server.");
    if let Some(pid_file_path) = get_pid_file_path(app_handle) {
        if pid_file_path.exists() {
            info!("[DEV_SERVER_MANAGER] PID file found at {}. Attempting to stop development server via kill script.", pid_file_path.display());
            // Execute the kill-dev-server.js script
            let script_path = PathBuf::from("../scripts/kill-dev-server.js");
            let current_dir = std::env::current_dir().unwrap_or_default();
            let full_script_path = current_dir.join(&script_path);

            info!("[DEV_SERVER_MANAGER] Executing kill script: node {}", full_script_path.display());
            match Command::new("node")
                .arg(full_script_path)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        info!("[DEV_SERVER_MANAGER] kill-dev-server.js executed successfully.");
                        // The script itself should delete the PID file, but we can double check
                        if pid_file_path.exists() {
                            match fs::remove_file(&pid_file_path) {
                                Ok(_) => info!("[DEV_SERVER_MANAGER] PID file removed: {}", pid_file_path.display()),
                                Err(e) => error!("[DEV_SERVER_MANAGER] Failed to remove PID file {}: {}", pid_file_path.display(), e),
                            }
                        }
                    } else {
                        error!("[DEV_SERVER_MANAGER] kill-dev-server.js failed with status: {}", output.status);
                        error!("[DEV_SERVER_MANAGER] Stdout: {}", String::from_utf8_lossy(&output.stdout));
                        error!("[DEV_SERVER_MANAGER] Stderr: {}", String::from_utf8_lossy(&output.stderr));
                    }
                },
                Err(e) => error!("[DEV_SERVER_MANAGER] Failed to execute kill-dev-server.js: {}", e),
            }
        } else {
            info!("[DEV_SERVER_MANAGER] PID file not found at {}. Development server might not be running or was already terminated.", pid_file_path.display());
        }
    }
}