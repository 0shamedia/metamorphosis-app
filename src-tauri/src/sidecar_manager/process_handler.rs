// metamorphosis-app/src-tauri/src/sidecar_manager/process_handler.rs

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Wry};
use tauri_plugin_shell::ShellExt;
use once_cell::sync::Lazy;
use log::{info, error};
use std::collections::HashMap;

// Internal imports from sibling modules
use super::event_utils::{emit_backend_status, COMFYUI_PORT};

// Crate-level imports
use crate::gpu_detection::{get_gpu_info, GpuType};
use crate::setup_manager::python_utils::{get_conda_env_python_executable_path, get_conda_executable_path};
use crate::process_manager::ProcessManager;

// Global static variables for process management
// COMFYUI_CHILD_PROCESS is now deprecated and handled by the central ProcessManager.
pub static IS_ATTEMPTING_SPAWN: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
pub static RESTART_ATTEMPTS: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));
pub static LAST_RESTART_TIME: Lazy<Mutex<Option<Instant>>> = Lazy::new(|| Mutex::new(None));
pub const MAX_RESTARTS_PER_HOUR: u32 = 5;


// Renamed: Internal function to actually spawn the sidecar process
// Assumes dependencies are already installed. Returns Result.
// This function is intended for internal use by orchestration functions.
pub(super) async fn spawn_comfyui_process(app_handle: AppHandle<Wry>) -> Result<(), String> {
    log::error!("[EARLY_SPAWN_DEBUG] INTERNAL spawn_comfyui_process INVOKED");
    
    // The check for an existing process is now implicitly handled by the orchestration logic
    // which should not call this if a process is running. The ProcessManager will log if a
    // process with the same name is spawned.

    // The scopeguard for IS_ATTEMPTING_SPAWN is managed by the caller.

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

        let conda_exe_path = get_conda_executable_path(&app_handle).await?;
        if !conda_exe_path.exists() {
            let err_msg = format!("Conda executable not found at expected path: {}", conda_exe_path.display());
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

        info!("Using Conda executable: {}", conda_exe_path.display());
        info!("Using ComfyUI script: {}", main_script.display());
        info!("Setting CWD to: {}", comfyui_dir.display());

        let gpu_info = get_gpu_info();
        info!("Detected GPU Info: {:?}", gpu_info);

        let mut comfyui_args = vec![
            "main.py".to_string(), // Relative to the CWD, which is comfyui_dir
            "--listen".to_string(),
            "--front-end-version".to_string(),
            "Comfy-Org/ComfyUI_frontend@v1.18.2".to_string(),
            "--port".to_string(),
            COMFYUI_PORT.to_string(),
            "--enable-cors-header".to_string(),
            "*".to_string(),
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
            GpuType::Unknown => {
                info!("Unknown GPU type detected, adding --cpu flag.");
                true
            }
        };

        if use_cpu {
            comfyui_args.push("--cpu".to_string());
        }

        // 1. Capture the environment from Conda
        info!("Capturing environment variables from conda env 'comfyui_env'...");
        let env_capture_output = app_handle.shell()
            .command(&conda_exe_path)
            .args(&["run", "-n", "comfyui_env", "cmd", "/c", "set"])
            .output()
            .await
            .map_err(|e| format!("Failed to capture conda environment: {}", e))?;

        if !env_capture_output.status.success() {
            return Err(format!("Failed to capture conda environment. Stderr: {}", String::from_utf8_lossy(&env_capture_output.stderr)));
        }

        let env_vars: HashMap<String, String> = String::from_utf8_lossy(&env_capture_output.stdout)
            .lines()
            .filter_map(|line| {
                if let Some((key, value)) = line.split_once('=') {
                    Some((key.to_string(), value.to_string()))
                } else {
                    None
                }
            })
            .collect();
        info!("Successfully captured {} environment variables.", env_vars.len());

        // 2. Get python executable and spawn with the captured env
        let python_exe_path = get_conda_env_python_executable_path(&app_handle, "comfyui_env").await?;
        info!("Direct Python executable path: {}", python_exe_path.display());

        let final_command = format!("{} {}", python_exe_path.to_string_lossy(), comfyui_args.join(" "));
        info!("Final ComfyUI launch command: {}", final_command);
        info!("Working Directory: {}", comfyui_dir.display());

        info!("Preparing to spawn ComfyUI process via ProcessManager...");
        let command = app_handle.shell().command(python_exe_path)
            .args(comfyui_args)
            .current_dir(&comfyui_dir)
            .envs(env_vars);

        ProcessManager::spawn_managed_process(
            &app_handle,
            "comfyui_sidecar".to_string(),
            command,
        ).await?;

        info!("ComfyUI sidecar process has been handed off to the ProcessManager.");

        // The ProcessManager now handles logging stdout/stderr and termination.
        // Health check is handled by the calling orchestration function.

        Ok(())
    }

// The is_comfyui_process_active and stop_comfyui_sidecar functions are now deprecated.
// Process lifecycle and state are managed by the central ProcessManager.