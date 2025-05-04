use std::path::PathBuf;
use std::error::Error; // Import the Error trait
use tauri::{AppHandle, Manager, Wry};
use tauri::path::BaseDirectory;
use tauri::async_runtime::{self};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use log::{info, error};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use scopeguard; // Import scopeguard for the cleanup guard
use tauri_plugin_http::reqwest; // For making HTTP requests
use tokio::time::{interval, Duration}; // For periodic health checks
use std::time::Instant; // To track restart times

// Import necessary modules from the crate
use crate::gpu_detection::{GpuInfo, GpuType, get_gpu_info};
use crate::dependency_management;

// Global static variable to hold the child process handle
static COMFYUI_CHILD_PROCESS: Lazy<Mutex<Option<CommandChild>>> = Lazy::new(|| Mutex::new(None));
static RESTART_ATTEMPTS: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));
static LAST_RESTART_TIME: Lazy<Mutex<Option<Instant>>> = Lazy::new(|| Mutex::new(None));
const MAX_RESTARTS_PER_HOUR: u32 = 5; // Define a limit for restarts
const COMFYUI_PORT: u16 = 8188; // TODO: Make this configurable

// Function to start the sidecar
pub fn start_comfyui_sidecar(app_handle: AppHandle<Wry>) {
    // Clone app_handle specifically for the first spawned task
    // Clone app_handle specifically for the first spawned task
    let app_handle_clone_for_sidecar = app_handle.clone();
    async_runtime::spawn(async move {
        let app_handle = app_handle_clone_for_sidecar; // Use the cloned handle inside the async block
        info!("Attempting to start ComfyUI sidecar process on port {}...", COMFYUI_PORT);

        // Check restart limits
        let mut attempts = RESTART_ATTEMPTS.lock().unwrap();
        let mut last_restart = LAST_RESTART_TIME.lock().unwrap();

        let now = Instant::now();
        if let Some(last_time) = *last_restart {
            if now.duration_since(last_time) > Duration::from_secs(3600) { // Reset count after 1 hour
                *attempts = 0;
            }
        }

        if *attempts >= MAX_RESTARTS_PER_HOUR {
            error!("Maximum restart attempts ({}) reached within the last hour. Not attempting to restart ComfyUI.", MAX_RESTARTS_PER_HOUR);
            // TODO: Notify the user in the frontend
            return;
        }

        *attempts += 1;
        *last_restart = Some(now);

        info!("Restart attempt #{}", *attempts);

        // Install Python dependencies if not already installed
        if let Err(e) = dependency_management::install_python_dependencies(&app_handle) {
            error!("Failed to install Python dependencies: {}", e);
            // Depending on the desired behavior, you might want to show an error to the user
            // or prevent the ComfyUI sidecar from starting. For now, we just log the error.
            return;
        }
        info!("Python dependency installation check/process completed.");

        // Get the path to the directory containing the current executable
        let exe_path = match std::env::current_exe() {
            Ok(path) => path,
            Err(e) => {
                error!("Failed to get current executable path: {}", e);
                return;
            }
        };
        let exe_dir = match exe_path.parent() {
             Some(dir) => dir.to_path_buf(), // Parent exists, convert &Path to PathBuf
             None => {
                 // Handle the case where there is no parent (e.g., root directory)
                 error!("Failed to get parent directory of executable: {}", exe_path.display());
                 // Depending on desired behavior, you might return an error,
                 // use the current directory, or panic. Returning here as before.
                 return;
             }
        };
        info!("Executable directory: {}", exe_dir.display());

        // Get the path to the target directory (parent of the executable directory)
        let target_dir = match exe_dir.parent() {
            Some(dir) => dir.to_path_buf(),
            None => {
                error!("Failed to get target directory from executable path: {}", exe_dir.display());
                return;
            }
        };
        info!("DEBUG: Base directory for vendor resolved at runtime: {:?}", target_dir);

        // Construct the path to the vendor directory relative to the target directory
        let vendor_path = target_dir.join("vendor");

        // Construct the path to the Python installation root within the vendor directory
        let python_root = if std::env::var("TAURI_DEV").is_ok() {
            // In development mode, derive the path from CARGO_MANIFEST_DIR (src-tauri)
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("debug").join("vendor").join("python")
        } else {
            // In bundled mode, the vendor directory is relative to the executable.
            vendor_path.join("python")
        };

        // Construct the path to the ComfyUI directory within the vendor directory
        // Construct the path to the ComfyUI directory within the vendor directory
        // Assuming development mode for now due to path resolution issues
        let comfyui_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("debug").join("vendor").join("comfyui");

        // Construct the path to the Python executable within the virtual environment
        let venv_dir = comfyui_dir.join(".venv");
        let venv_python_executable = if cfg!(target_os = "windows") {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python")
        };

        // Check if the constructed paths exist
        if !venv_python_executable.exists() {
            error!("Virtual environment Python executable not found at expected path: {}", venv_python_executable.display());
            return;
        }
         if !comfyui_dir.exists() {
             error!("ComfyUI directory not found at expected path: {}", comfyui_dir.display());
             return;
         }

        let main_script = comfyui_dir.join("main.py");

        if !main_script.exists() {
            error!("ComfyUI main.py not found at: {}", main_script.display());
            return;
        }
        if !comfyui_dir.exists() {
            error!("ComfyUI directory not found at: {}", comfyui_dir.display());
            return;
        }

        info!("Using Python executable: {}", venv_python_executable.display());
        info!("Using ComfyUI script: {}", main_script.display());
        info!("Setting CWD to: {}", comfyui_dir.display());

        // Get detailed GPU information
        let gpu_info = get_gpu_info();
        info!("Detected GPU Info: {:?}", gpu_info);

        let mut args = vec![
            main_script.to_string_lossy().into_owned(),
            "--listen".to_string(), // Add --listen argument
            "--front-end-version".to_string(), // Add frontend version argument
            "Comfy-Org/ComfyUI_frontend@v1.18.2".to_string(), // Specify frontend version v1.18.2
            "--port".to_string(), // Add --port argument
            COMFYUI_PORT.to_string(), // Specify the port
        ];

        // Determine whether to use --cpu flag based on detected GPU type
        let use_cpu = match gpu_info.gpu_type {
            GpuType::Nvidia => {
                // For NVIDIA, we assume GPU mode unless CUDA version detection failed or is not ideal.
                // For now, we'll launch in GPU mode if NVIDIA is detected.
                info!("NVIDIA GPU detected, launching in GPU mode.");
                false
            }
            GpuType::Amd => {
                // TODO: Implement logic for AMD GPUs. ComfyUI supports ROCm.
                info!("AMD GPU detected. Currently defaulting to CPU mode. ROCm support needs to be implemented.");
                true // Default to CPU for now
            }
            GpuType::Intel => {
                // TODO: Implement logic for Intel GPUs. ComfyUI supports Intel.
                info!("Intel GPU detected. Currently defaulting to CPU mode. Intel GPU support needs to be implemented.");
                true // Default to CPU for now
            }
            GpuType::Other | GpuType::Unknown => {
                info!("Other or Unknown GPU type detected, adding --cpu flag.");
                true // Default to CPU for unknown or other types
            }
        };

        if use_cpu {
            args.push("--cpu".to_string());
        }

        info!("ComfyUI args: {:?}", args);

        // Log the final command being executed
        let final_command = format!("{} {}", venv_python_executable.to_string_lossy(), args.join(" "));
        info!("Final ComfyUI launch command: {}", final_command);
        info!("Working Directory: {}", comfyui_dir.display());
        info!("Command path being used: {}", venv_python_executable.to_string_lossy());
        info!("Working directory being used: {}", comfyui_dir.display());

        info!("Attempting to spawn ComfyUI process...");
        let (mut rx, child) = match app_handle.shell().command(&venv_python_executable)
            .args(args.clone())
            .current_dir(&comfyui_dir) // Change working directory to comfyui_dir
            .spawn() {
                Ok((rx, child)) => {
                    info!("ComfyUI process started successfully (PID: {}).", child.pid());
                    info!("Successfully spawned ComfyUI process."); // Added log
                    (rx, child)
                },
                Err(e) => {
                    error!("Failed to spawn ComfyUI process: {}", e);
                    error!("Spawn failed for ComfyUI process."); // Added log
                    // Depending on the desired behavior, you might want to show an error to the user
                    // or prevent the application from continuing. For now, we just log the error.
                    return;
                }
            };

        // Asynchronous logging using CommandEvent
        async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        info!("[ComfyUI stdout] {}", String::from_utf8_lossy(&line));
                    }
                    CommandEvent::Stderr(line) => {
                        error!("[ComfyUI stderr] {}", String::from_utf8_lossy(&line)); // Log stderr as error
                    }
                    CommandEvent::Terminated(exit_status) => {
                        info!("[ComfyUI] Process terminated with status: {:?}", exit_status);
                        // Update the global child process handle to None
                        *COMFYUI_CHILD_PROCESS.lock().unwrap() = None;
                        info!("ComfyUI child process handle cleared on termination.");
                    }
                    _ => {} // Ignore other events for now
                }
            }
            info!("[ComfyUI] Output stream processing finished.");
        });


        // Store the child handle
        *COMFYUI_CHILD_PROCESS.lock().unwrap() = Some(child);
        info!("ComfyUI child process handle stored successfully."); // Added log

    });

    // Start health monitoring task after attempting to start the process
    // Clone app_handle before moving it into the spawned task
    async_runtime::spawn(monitor_comfyui_health(app_handle.clone()));
}

// Function to monitor the health of the ComfyUI sidecar
async fn monitor_comfyui_health(app_handle: AppHandle<Wry>) {
    let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds
    info!("Starting ComfyUI health monitoring...");
    tokio::time::sleep(Duration::from_secs(45)).await; // Initial delay before first health check
    info!("Initial delay complete, starting periodic health checks.");

    loop {
        interval.tick().await; // Wait for the next tick

        // Check if the process is running within a block to manage lock scope
        let is_running = {
            let child_process_guard = COMFYUI_CHILD_PROCESS.lock().unwrap();
            child_process_guard.is_some()
        }; // The lock is dropped here as child_process_guard goes out of scope

        if !is_running {
            info!("ComfyUI process is not running, attempting to restart...");
            let app_handle_clone = app_handle.clone(); // Clone app_handle before moving into the spawned task
            start_comfyui_sidecar(app_handle_clone);
            continue; // Skip health check for this interval as we just attempted a restart
        }

        // Perform a simple HTTP GET request to the ComfyUI API endpoint using reqwest::Client
        let health_url = format!("http://localhost:{}/queue", COMFYUI_PORT); // A simple endpoint to check responsiveness
        let client = reqwest::Client::new();
        match client.get(&health_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("ComfyUI health check successful.");
                    // Reset restart attempts on successful health check after a failure
                    let mut attempts = RESTART_ATTEMPTS.lock().unwrap();
                    if *attempts > 0 {
                         info!("ComfyUI is healthy, resetting restart attempts.");
                        *attempts = 0;
                    }
                } else {
                    error!("ComfyUI health check failed: Received non-success status code: {}", response.status());
                    // Process might be running but unresponsive, attempt restart
                    let app_handle_clone = app_handle.clone(); // Clone app_handle before moving into the spawned task
                    start_comfyui_sidecar(app_handle_clone);
                }
            }
            Err(e) => {
                if e.is_connect() {
                    // Log connection errors with more detail, including the source if available
                    let mut error_msg = format!("ComfyUI health check failed: Connection error: {}", e);
                    if let Some(source) = e.source() {
                        error_msg.push_str(&format!(" Source: {}", source));
                    }
                    error!("{}", error_msg);
                } else if e.is_timeout() {
                     error!("ComfyUI health check failed: Timeout error: {}", e);
                } else if e.is_request() {
                     error!("ComfyUI health check failed: Request error: {}", e);
                }
                else {
                    error!("ComfyUI health check failed: Other error: {}", e);
                }
                // Process is likely crashed or not listening, attempt restart
                start_comfyui_sidecar(app_handle.clone());
            }
        }
    }
}


// Function to stop the sidecar
pub fn stop_comfyui_sidecar() {
    info!("Attempting to stop ComfyUI sidecar process...");
    if let Ok(mut child_lock) = COMFYUI_CHILD_PROCESS.lock() {
        if let Some(child) = child_lock.take() { // Take ownership from the Option
            info!("Found ComfyUI process (PID: {}), attempting to kill...", child.pid());
            match child.kill() {
                Ok(_) => info!("ComfyUI process killed successfully."),
                Err(e) => error!("Failed to kill ComfyUI process: {}", e),
            }
        } else {
            info!("No active ComfyUI process found to stop.");
        }
    } else {
        error!("Failed to acquire lock for ComfyUI process handle.");
    }
}