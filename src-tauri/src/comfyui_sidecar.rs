use tauri::{AppHandle, Wry};
use tauri::async_runtime::{self};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use log::{info, error};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use scopeguard; // Import scopeguard for the cleanup guard

// Import necessary modules from the crate
use crate::gpu_detection::{GpuInfo, GpuType, get_gpu_info};
use crate::dependency_management;

// Global static variable to hold the child process handle
static COMFYUI_CHILD_PROCESS: Lazy<Mutex<Option<CommandChild>>> = Lazy::new(|| Mutex::new(None));

// Function to start the sidecar
pub fn start_comfyui_sidecar(app_handle: AppHandle<Wry>) {
    async_runtime::spawn(async move {
        info!("Attempting to start ComfyUI sidecar process...");

        // Install Python dependencies if not already installed
        if let Err(e) = dependency_management::install_python_dependencies(&app_handle) {
            error!("Failed to install Python dependencies: {}", e);
            // Depending on the desired behavior, you might want to show an error to the user
            // or prevent the ComfyUI sidecar from starting. For now, we just log the error.
            return;
        }

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

        // Construct the path relative to the executable directory's parent
        // build.rs copies 'vendor' to the executable's parent directory (e.g., target/vendor)
        // So from target/debug/, we go up one level (../) to target/ and then into vendor/
        let comfyui_dir = exe_dir.join("../vendor/comfyui"); // Also update comfyui_dir path

        // Get the path to the virtual environment's Python executable
        let venv_dir = comfyui_dir.join(".venv");
        let venv_python_executable = if cfg!(target_os = "windows") {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python")
        };


        // Check if the constructed paths exist
        if !venv_python_executable.exists() {
            error!("Venv Python executable not found at expected path: {}", venv_python_executable.display());
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

        info!("Using Venv Python executable: {}", venv_python_executable.display());
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

        info!("Attempting to spawn ComfyUI process...");
        let (mut rx, child) = match app_handle.shell().command(venv_python_executable.to_string_lossy().to_string())
            .args(args.clone())
            .current_dir(&comfyui_dir) // Pass as reference
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