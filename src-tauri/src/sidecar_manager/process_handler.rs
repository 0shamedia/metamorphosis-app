// metamorphosis-app/src-tauri/src/sidecar_manager/process_handler.rs

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use std::pin::Pin;
use std::future::Future;
use tauri::{AppHandle, Wry, async_runtime};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use once_cell::sync::Lazy;
use log::{info, error};

// Internal imports from sibling modules
use super::event_utils::{emit_backend_status, COMFYUI_PORT};

// Crate-level imports
use crate::gpu_detection::{get_gpu_info, GpuType}; // GpuInfo is not directly used here but GpuType is

// Global static variables for process management
pub static COMFYUI_CHILD_PROCESS: Lazy<Mutex<Option<CommandChild>>> = Lazy::new(|| Mutex::new(None));
pub static IS_ATTEMPTING_SPAWN: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
pub static RESTART_ATTEMPTS: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));
pub static LAST_RESTART_TIME: Lazy<Mutex<Option<Instant>>> = Lazy::new(|| Mutex::new(None));
pub const MAX_RESTARTS_PER_HOUR: u32 = 5;


// Renamed: Internal function to actually spawn the sidecar process
// Assumes dependencies are already installed. Returns Result.
// This function is intended for internal use by orchestration functions.
pub(super) fn spawn_comfyui_process(app_handle: AppHandle<Wry>) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
    log::error!("[EARLY_SPAWN_DEBUG] INTERNAL spawn_comfyui_process INVOKED");
    
    // This function should not manage IS_ATTEMPTING_SPAWN.
    // That flag is managed by the calling orchestrator.
    // It should, however, check if a process is already running.
    if COMFYUI_CHILD_PROCESS.lock().unwrap().is_some() {
        info!("[PROCESS_HANDLER] spawn_comfyui_process: ComfyUI process is already active. Skipping new spawn.");
        return Box::pin(async { Ok(()) }); // Or perhaps an error indicating it's already running if that's unexpected by caller
    }

    // let app_handle_clone = app_handle.clone(); // This was unused as app_handle is moved into the async block
    Box::pin(async move { // app_handle is moved here
        // The scopeguard for IS_ATTEMPTING_SPAWN is removed from here as it's managed by the caller.

        info!("Attempting to spawn ComfyUI process on port {}...", COMFYUI_PORT);
        // emit_backend_status is now in event_utils
        emit_backend_status(&app_handle, "starting_sidecar", format!("Starting ComfyUI backend on port {}...", COMFYUI_PORT), false);

        let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get current executable path: {}", e))?;
        let exe_dir = exe_path.parent().ok_or_else(|| format!("Failed to get parent directory of executable: {}", exe_path.display()))?.to_path_buf();
        info!("Executable directory: {}", exe_dir.display());

        let target_dir = exe_dir.parent().ok_or_else(|| format!("Failed to get target directory from executable path: {}", exe_dir.display()))?.to_path_buf();
        info!("DEBUG: Base directory for vendor resolved at runtime: {:?}", target_dir);

        let comfyui_dir = if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("Failed to get parent of CARGO_MANIFEST_DIR for debug path construction")
                .join("target")
                .join("debug")
                .join("vendor")
                .join("comfyui")
        } else {
            target_dir.join("vendor").join("comfyui")
        };

        let venv_dir = comfyui_dir.join(".venv");
        let venv_python_executable = if cfg!(target_os = "windows") {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python")
        };

        if !venv_python_executable.exists() {
            let err_msg = format!("Virtual environment Python executable not found at expected path: {}", venv_python_executable.display());
            error!("{}", err_msg);
            emit_backend_status(&app_handle, "backend_error", err_msg.clone(), true);
            return Err(err_msg);
        }
         if !comfyui_dir.exists() {
             let err_msg = format!("ComfyUI directory not found at expected path: {}", comfyui_dir.display());
             error!("{}", err_msg);
             emit_backend_status(&app_handle, "backend_error", err_msg.clone(), true);
             return Err(err_msg);
         }

        let main_script = comfyui_dir.join("main.py");

        if !main_script.exists() {
            let err_msg = format!("ComfyUI main.py not found at: {}", main_script.display());
            error!("{}", err_msg);
            emit_backend_status(&app_handle, "backend_error", err_msg.clone(), true);
            return Err(err_msg);
        }

        info!("Using Python executable: {}", venv_python_executable.display());
        info!("Using ComfyUI script: {}", main_script.display());
        info!("Setting CWD to: {}", comfyui_dir.display());

        let gpu_info = get_gpu_info();
        info!("Detected GPU Info: {:?}", gpu_info);

        let mut args = vec![
            main_script.to_string_lossy().into_owned(),
            "--listen".to_string(), 
            "--front-end-version".to_string(), 
            "Comfy-Org/ComfyUI_frontend@v1.18.2".to_string(), 
            "--port".to_string(), 
            COMFYUI_PORT.to_string(), 
        ];

        let use_cpu = match gpu_info.gpu_type {
            GpuType::Nvidia => {
                info!("NVIDIA GPU detected, launching in GPU mode.");
                false
            }
            GpuType::Amd => {
                info!("AMD GPU detected. Currently defaulting to CPU mode. ROCm support needs to be implemented.");
                true 
            }
            GpuType::Intel => {
                info!("Intel GPU detected. Currently defaulting to CPU mode. Intel GPU support needs to be implemented.");
                true 
            }
            GpuType::Other | GpuType::Unknown => {
                info!("Other or Unknown GPU type detected, adding --cpu flag.");
                true 
            }
        };

        if use_cpu {
            args.push("--cpu".to_string());
        }

        info!("ComfyUI args: {:?}", args);

        let final_command = format!("{} {}", venv_python_executable.to_string_lossy(), args.join(" "));
        info!("Final ComfyUI launch command: {}", final_command);
        info!("Working Directory: {}", comfyui_dir.display());
        info!("Command path being used: {}", venv_python_executable.to_string_lossy());
        info!("Working directory being used: {}", comfyui_dir.display());

        info!("Attempting to spawn ComfyUI process...");
        let (mut rx, child) = match app_handle.shell().command(&venv_python_executable)
            .args(args.clone())
            .current_dir(&comfyui_dir) 
            .spawn() {
                Ok((rx, child)) => {
                    info!("ComfyUI process started successfully (PID: {}).", child.pid());
                    info!("Successfully spawned ComfyUI process."); 
                    (rx, child)
                },
                Err(e) => {
                    let err_msg = format!("Failed to spawn ComfyUI process: {}", e);
                    error!("{}", err_msg);
                    emit_backend_status(&app_handle, "backend_error", err_msg.clone(), true);
                    return Err(err_msg);
                }
            };

        async_runtime::spawn(async move {
            // let app_handle_clone_for_logs = app_handle.clone(); // This clone is not used if emit_backend_status on termination is commented out
            // let app_handle_for_event_emission = app_handle_clone_for_logs; // Renaming for clarity - currently unused
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        info!("[ComfyUI stdout] {}", String::from_utf8_lossy(&line));
                    }
                    CommandEvent::Stderr(line) => {
                        error!("[ComfyUI stderr] {}", String::from_utf8_lossy(&line)); 
                    }
                    CommandEvent::Terminated(exit_status) => {
                        info!("[ComfyUI] Process terminated with status: {:?}", exit_status);
                        let mut child_lock = COMFYUI_CHILD_PROCESS.lock().unwrap();
                        if child_lock.is_some() {
                            *child_lock = None;
                            info!("ComfyUI child process handle cleared on termination.");
                            // emit_backend_status(&app_handle_for_event_emission, "sidecar_terminated", format!("ComfyUI process terminated: {:?}", exit_status), false);
                        }
                    }
                    _ => {} 
                }
            }
            info!("[ComfyUI] Output stream processing finished.");
        });


        *COMFYUI_CHILD_PROCESS.lock().unwrap() = Some(child);
        info!("ComfyUI child process handle stored successfully.");

        // Health check is now handled by the calling orchestration function
        // emit_backend_status(&app_handle, "sidecar_spawned_checking_health", "ComfyUI process spawned. Performing initial health check...".to_string(), false);
        // No direct call to perform_comfyui_health_check here anymore.

        Ok(()) 
    })
}

pub fn is_comfyui_process_active() -> bool {
    COMFYUI_CHILD_PROCESS.lock().unwrap().is_some()
}

pub fn stop_comfyui_sidecar() {
    info!("Attempting to stop ComfyUI sidecar process...");
    if let Ok(mut child_lock) = COMFYUI_CHILD_PROCESS.lock() {
        if let Some(child) = child_lock.take() { 
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