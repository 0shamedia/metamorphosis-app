// metamorphosis-app/src-tauri/src/setup_manager/verification.rs
use tauri::{Window, WebviewWindow, AppHandle, Manager, Wry, Emitter};
use serde_json::json;
use log::{error, info};
use std::path::PathBuf;
use std::fs;

use super::types::SetupStatusEvent; // Import from the new types module

// Note: comfyui_sidecar and dependency_management are kept as crate level for now.
// If they are also refactored into managers, these paths would change.
// use crate::dependency_management;
// use crate::comfyui_sidecar;


#[tauri::command]
pub async fn check_initialization_status(window: WebviewWindow) -> Result<(), String> {
    // Record start time for performance tracking
    let start_time = std::time::Instant::now();
    info!("[SETUP_VERIFICATION] check_initialization_status started");
    
    // Log system information for diagnostics
    info!("[SETUP_VERIFICATION] System information:");
    info!("[SETUP_VERIFICATION] OS: {}", std::env::consts::OS);
    info!("[SETUP_VERIFICATION] Arch: {}", std::env::consts::ARCH);
    info!("[SETUP_VERIFICATION] Current dir: {:?}", std::env::current_dir().unwrap_or_default());
    
    // Make sure the window is visible first
    info!("[SETUP_VERIFICATION] Attempting to show window...");
    let show_start = std::time::Instant::now();
    
    match window.show() {
        Ok(_) => {
            let elapsed = show_start.elapsed();
            info!("[SETUP_VERIFICATION] Window successfully shown in {:?}", elapsed);
        },
        Err(e) => {
            error!("[SETUP_VERIFICATION] Error showing window: {} (after {:?})", e, show_start.elapsed());
            return Err(format!("Failed to show window: {}", e));
        }
    }
    
    // Check for window dimensions
    match window.inner_size() {
        Ok(size) => {
            info!("[SETUP_VERIFICATION] Window dimensions: {}x{}", size.width, size.height);
        },
        Err(e) => {
            error!("[SETUP_VERIFICATION] Error getting window dimensions: {}", e);
        }
    }
    
    // Send initial status - we're initializing
    info!("[SETUP_VERIFICATION] Emitting initializing status...");
    let emit_start = std::time::Instant::now();
    
    match window.emit("initialization-status", json!({
        "status": "initializing",
        "message": "Initializing Metamorphosis..."
    })) {
        Ok(_) => {
            let elapsed = emit_start.elapsed();
            info!("[SETUP_VERIFICATION] Successfully emitted initializing status in {:?}", elapsed);
        },
        Err(e) => {
            error!("[SETUP_VERIFICATION] Error emitting initialization status: {} (after {:?})", e, emit_start.elapsed());
            return Err(format!("Failed to emit status: {}", e));
        }
    }
    
    info!("[SETUP_VERIFICATION] Performing initialization checks...");
    let check_start = std::time::Instant::now();

    let app_handle = window.app_handle();

    // Check 1: Verify Application Data Directory
    info!("[SETUP_VERIFICATION] Check 1: Verifying Application Data Directory...");
    match app_handle.path().app_data_dir() {
        Ok(app_data_path) => {
            if !app_data_path.exists() {
                if let Err(e) = fs::create_dir_all(&app_data_path) {
                    let error_msg = format!("Failed to create app data directory at {:?}: {}", app_data_path, e);
                    error!("[SETUP_VERIFICATION] {}", error_msg);
                    window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
                    return Err(error_msg);
                }
                info!("[SETUP_VERIFICATION] Created app data directory at {:?}", app_data_path);
            } else {
                info!("[SETUP_VERIFICATION] App data directory verified at {:?}", app_data_path);
            }
            window.emit("initialization-status", json!({ "status": "progress", "stage": "VerifyingAppDataDir", "progress": 25, "message": "Verifying application data..." })).map_err(|e| e.to_string())?;
        }
        Err(e) => {
            let error_msg = format!("Failed to resolve application data directory path: {}", e);
            error!("[SETUP_VERIFICATION] {}", error_msg);
            window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
            return Err(error_msg);
        }
    }
    info!("[SETUP_VERIFICATION] Check 1 completed in {:?}", check_start.elapsed());
    
    let check_2_start = std::time::Instant::now();

    // Check 2: Check Python Executable Path
    info!("[SETUP_VERIFICATION] Check 2: Checking Python Executable Path...");
    let python_executable_path_result: Result<PathBuf, String> = {
        let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get current exe path: {}", e))?;
        let exe_dir = exe_path.parent().ok_or_else(|| "Failed to get executable directory".to_string())?;
        if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .ok_or_else(|| "Failed to get parent of CARGO_MANIFEST_DIR for python executable".to_string())?
                .join("target")
                .join("debug")
                .join("vendor")
                .join("python")
                .join("python.exe")
        } else {
            exe_dir.join("vendor").join("python").join("python.exe")
        }
        .canonicalize() // Resolve to absolute path to be sure
        .map_err(|e| format!("Failed to canonicalize python path: {}", e))
    };

    match python_executable_path_result {
        Ok(python_path) => {
            if python_path.exists() && python_path.is_file() {
                info!("[SETUP_VERIFICATION] Python executable path verified at {:?}", python_path);
                window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingPythonPath", "progress": 50, "message": "Verifying Python environment..." })).map_err(|e| e.to_string())?;
            } else {
                let error_msg = format!("Python executable not found or is not a file at resolved path: {:?}", python_path);
                error!("[SETUP_VERIFICATION] {}", error_msg);
                window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
                return Err(error_msg);
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to determine Python executable path: {}", e);
            error!("[SETUP_VERIFICATION] {}", error_msg);
            window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
            return Err(error_msg);
        }
    }
    info!("[SETUP_VERIFICATION] Check 2 completed in {:?}", check_2_start.elapsed());
    
    let check_3_start = std::time::Instant::now();

    // Check 3: Check ComfyUI Directory Path
    info!("[SETUP_VERIFICATION] Check 3: Checking ComfyUI Directory Path...");
    let comfyui_directory_path_result: Result<PathBuf, String> = {
        let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get current exe path: {}", e))?;
        let exe_dir = exe_path.parent().ok_or_else(|| "Failed to get executable directory".to_string())?;
        if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .ok_or_else(|| "Failed to get parent of CARGO_MANIFEST_DIR for comfyui_dir".to_string())?
                .join("target")
                .join("debug")
                .join("vendor")
                .join("comfyui")
        } else {
            exe_dir.join("vendor").join("comfyui")
        }
        .canonicalize() // Resolve to absolute path
        .map_err(|e| format!("Failed to canonicalize comfyui path: {}", e))
    };

    match comfyui_directory_path_result {
        Ok(comfyui_path) => {
            if comfyui_path.exists() && comfyui_path.is_dir() {
                info!("[SETUP_VERIFICATION] ComfyUI directory path verified at {:?}", comfyui_path);
                window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingComfyUIPath", "progress": 75, "message": "Verifying ComfyUI components..." })).map_err(|e| e.to_string())?;
            } else {
                let error_msg = format!("ComfyUI directory not found or is not a directory at resolved path: {:?}", comfyui_path);
                error!("[SETUP_VERIFICATION] {}", error_msg);
                window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
                return Err(error_msg);
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to determine ComfyUI directory path: {}", e);
            error!("[SETUP_VERIFICATION] {}", error_msg);
            window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
            return Err(error_msg);
        }
    }
    info!("[SETUP_VERIFICATION] Check 3 completed in {:?}", check_3_start.elapsed());
    info!("[SETUP_VERIFICATION] All initialization checks completed in {:?}", check_start.elapsed());

    // Send ready status
    info!("[SETUP_VERIFICATION] Emitting ready status...");
    let ready_emit_start = std::time::Instant::now();

    match window.emit("initialization-status", json!({
        "status": "ready",
        "message": "Initialization complete. Ready to proceed."
    })) {
        Ok(_) => {
            let elapsed = ready_emit_start.elapsed();
            info!("[SETUP_VERIFICATION] Successfully emitted ready status in {:?}", elapsed);
        },
        Err(e) => {
            error!("[SETUP_VERIFICATION] Error emitting ready status: {} (after {:?})", e, ready_emit_start.elapsed());
            return Err(format!("Failed to emit ready status: {}", e));
        }
    }

    let total_elapsed = start_time.elapsed();
    info!("[SETUP_VERIFICATION] Initialization status check complete in {:?}", total_elapsed);
    Ok(())
}


fn get_comfyui_vendor_paths(app_handle: &AppHandle<Wry>) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get current exe path: {}", e))?;
    let exe_dir = exe_path.parent().ok_or_else(|| "Failed to get executable directory".to_string())?;

    let base_path = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| "Failed to get parent of CARGO_MANIFEST_DIR".to_string())?
            .join("target")
            .join("debug")
            .join("vendor")
    } else {
        exe_dir.join("vendor")
    };

    let comfyui_dir = base_path.join("comfyui");
    let venv_dir = comfyui_dir.join(".venv");
    let venv_python_executable = if cfg!(target_os = "windows") {
        venv_dir.join("Scripts").join("python.exe")
    } else {
        venv_dir.join("bin").join("python")
    };
    Ok((comfyui_dir, venv_dir, venv_python_executable))
}


pub async fn run_quick_verification(app_handle: &AppHandle<Wry>) -> Result<bool, String> {
    info!("[QUICK VERIFY] Starting quick verification process...");

    let (comfyui_dir, venv_dir, venv_python_executable) = get_comfyui_vendor_paths(app_handle)?;

    // 1. Venv Integrity: Check .venv directory
    if !venv_dir.exists() || !venv_dir.is_dir() {
        info!("[QUICK VERIFY] FAILED: .venv directory not found at {}", venv_dir.display());
        return Ok(false);
    }
    info!("[QUICK VERIFY] PASSED: .venv directory exists at {}", venv_dir.display());

    // 2. Venv Integrity: Check Python executable within .venv
    if !venv_python_executable.exists() || !venv_python_executable.is_file() {
        info!("[QUICK VERIFY] FAILED: Python executable not found in .venv at {}", venv_python_executable.display());
        return Ok(false);
    }
    info!("[QUICK VERIFY] PASSED: Python executable exists in .venv at {}", venv_python_executable.display());

    // 3. Critical File Existence: Check for vendor/comfyui/main.py
    let main_py_path = comfyui_dir.join("main.py");
    if !main_py_path.exists() || !main_py_path.is_file() {
        info!("[QUICK VERIFY] FAILED: main.py not found at {}", main_py_path.display());
        return Ok(false);
    }
    info!("[QUICK VERIFY] PASSED: main.py exists at {}", main_py_path.display());
    
    // 4. (Optional) ComfyUI Basic Health - can be added later if needed.
    // ComfyUI sidecar start and health check will be handled by a subsequent command
    // after SplashScreen receives BackendFullyVerifiedAndReady.
    info!("[QUICK VERIFY] File-based verification checks passed. Sidecar start deferred.");
    info!("[QUICK VERIFY] All quick file verification checks passed.");
    Ok(true)
}