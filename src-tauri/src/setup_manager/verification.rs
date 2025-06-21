// metamorphosis-app/src-tauri/src/setup_manager/verification.rs
use tauri::{WebviewWindow, AppHandle, Manager, Wry, Emitter};
use tauri_plugin_shell::ShellExt;
use serde_json::json;
use tokio::io::AsyncWriteExt;
use log::{error, info, warn};
use std::path::{Path, PathBuf};
use std::fs;
use crate::process_manager::ProcessManager;
// use std::env; // No longer needed

// Import new python_utils functions
use crate::setup_manager::python_utils::{
    get_comfyui_directory_path,
    get_conda_env_python_executable_path,
};
use crate::setup_manager::orchestration::get_app_root_path; // Import get_app_root_path

// use super::types::SetupStatusEvent;

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

    // Check 2: Check Miniconda Installation
    info!("[SETUP_VERIFICATION] Check 2: Checking Miniconda Installation...");
    let app_root_path = get_app_root_path()?;
    let miniconda_install_path = app_root_path.join("miniconda3");
    let miniconda_marker_path = app_root_path.join(".miniconda_installed.marker");

    if miniconda_install_path.exists() && miniconda_install_path.is_dir() && miniconda_marker_path.exists() && miniconda_marker_path.is_file() {
        info!("[SETUP_VERIFICATION] Miniconda installation verified at {:?}", miniconda_install_path);
        window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingMiniconda", "progress": 50, "message": "Miniconda installation found." })).map_err(|e| e.to_string())?;
    } else {
        let warning_msg = format!("Miniconda installation not found or incomplete at {:?}. This is expected on first run and will be installed.", miniconda_install_path);
        warn!("[SETUP_VERIFICATION] {}", warning_msg);
        window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingMiniconda", "progress": 50, "message": warning_msg })).map_err(|e| e.to_string())?;
    }
    info!("[SETUP_VERIFICATION] Check 2 completed in {:?}", check_2_start.elapsed());
    
    let check_3_start = std::time::Instant::now();

    // Check 3: Check ComfyUI Directory Path
    info!("[SETUP_VERIFICATION] Check 3: Checking ComfyUI Directory Path...");
    let comfyui_directory_path_result = get_comfyui_directory_path(&app_handle);

    match comfyui_directory_path_result {
        Ok(comfyui_path) => {
            if comfyui_path.exists() && comfyui_path.is_dir() {
                info!("[SETUP_VERIFICATION] ComfyUI directory path verified at {:?}", comfyui_path);
                window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingComfyUIPath", "progress": 75, "message": "ComfyUI directory found." })).map_err(|e| e.to_string())?;
            } else {
                let warning_msg = format!("ComfyUI directory not found or is not a directory at resolved path: {:?}. This is expected on first run and will be installed.", comfyui_path);
                warn!("[SETUP_VERIFICATION] {}", warning_msg);
                // Emit a warning status or just log and continue. Let's just log and continue for now.
                window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingComfyUIPath", "progress": 75, "message": warning_msg })).map_err(|e| e.to_string())?;
            }
        }
        Err(e) => {
            let warning_msg = format!("Failed to determine ComfyUI directory path: {}. This is expected on first run and will be installed.", e);
            warn!("[SETUP_VERIFICATION] {}", warning_msg);
            // Emit a warning status or just log and continue. Let's just log and continue for now.
            window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingComfyUIPath", "progress": 75, "message": warning_msg })).map_err(|e| e.to_string())?;
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

// Removed local get_comfyui_vendor_paths, will use python_utils directly.

pub async fn run_quick_verification(app_handle: &AppHandle<Wry>) -> Result<bool, String> {
    info!("[QUICK VERIFY] Starting quick verification process...");

    let comfyui_dir = get_comfyui_directory_path(app_handle)?;
    let _venv_python_executable = get_conda_env_python_executable_path(app_handle, "comfyui_env").await?;
    // venv_dir can be derived if needed: venv_python_executable.parent().unwrap().parent().unwrap()
    // For the check, we primarily need comfyui_dir and venv_python_executable.
    // Let's get venv_dir explicitly for the check.
    let _venv_dir = comfyui_dir.join(".venv");


    // Quick verification should only check for the presence of core files/directories placed by the build script.
    // The .venv and its contents are created during the full setup, so they should not be checked here.


    // 3. Check for vendor/comfyui directory
    let comfyui_dir = get_comfyui_directory_path(app_handle)?;
    info!("[QUICK VERIFY] Checking for vendor/comfyui directory at {}", comfyui_dir.display());
    if !comfyui_dir.exists() || !comfyui_dir.is_dir() {
        info!("[QUICK VERIFY] FAILED: vendor/comfyui directory not found at {}", comfyui_dir.display());
        return Ok(false);
    }
    info!("[QUICK VERIFY] PASSED: vendor/comfyui directory found.");

    // 4. Check for vendor/comfyui/main.py
    let main_py_path = comfyui_dir.join("main.py");
    info!("[QUICK VERIFY] Checking for main.py at {}", main_py_path.display());
    if !main_py_path.exists() || !main_py_path.is_file() {
        info!("[QUICK VERIFY] FAILED: main.py not found at {}", main_py_path.display());
        return Ok(false);
    }
    info!("[QUICK VERIFY] PASSED: main.py exists.");

    // ComfyUI sidecar start and health check will be handled by a subsequent command
    // after SplashScreen receives BackendFullyVerifiedAndReady.
    info!("[QUICK VERIFY] All essential file existence checks passed.");
    Ok(true)
}

// Verification step names (for event payloads and logging)
const VERIFICATION_EVENT_IPADAPTER_DIR: &str = "Verifying IPAdapter Plus directory";
const VERIFICATION_EVENT_PYTHON_ENV: &str = "Verifying Python environment integrity";
// const VERIFICATION_EVENT_ONNXRUNTIME_IMPORT: &str = "Verifying onnxruntime import"; // Unused
// const VERIFICATION_EVENT_INSIGHTFACE_IMPORT: &str = "Verifying insightface import"; // Unused

// Tauri Event names (constants for consistency)
const EVT_VERIFICATION_STEP_START: &str = "VerificationStepStart";
const EVT_VERIFICATION_STEP_SUCCESS: &str = "VerificationStepSuccess";
const EVT_VERIFICATION_STEP_FAILED: &str = "VerificationStepFailed";

/// Checks if the ComfyUI_IPAdapter_plus custom node directory exists.
pub async fn check_ipadapter_plus_directory_exists(
    app_handle: &AppHandle<Wry>,
    comfyui_base_path: &Path,
) -> Result<bool, String> {
    let step_name = VERIFICATION_EVENT_IPADAPTER_DIR;
    info!("[VERIFY] Starting: {}", step_name);
    app_handle.emit(EVT_VERIFICATION_STEP_START, json!({ "stepName": step_name })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_START, e))?;

    let ipadapter_dir = comfyui_base_path.join("custom_nodes").join("ComfyUI_IPAdapter_plus");
    info!("[VERIFY] Checking for directory: {}", ipadapter_dir.display());

    if ipadapter_dir.exists() && ipadapter_dir.is_dir() {
        info!("[VERIFY] SUCCESS: {} found at {}", step_name, ipadapter_dir.display());
        app_handle.emit(EVT_VERIFICATION_STEP_SUCCESS, json!({ "stepName": step_name, "details": format!("Directory found at {}", ipadapter_dir.display()) })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_SUCCESS, e))?;
        Ok(true)
    } else {
        let err_msg = format!("Directory not found or is not a directory: {}", ipadapter_dir.display());
        warn!("[VERIFY] FAILED: {} - {}", step_name, err_msg);
        app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name, "error": err_msg.clone(), "details": null })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_FAILED, e))?;
        Ok(false) // Indicates check performed, but condition not met
    }
}


// Helper to dynamically create a Python script that checks for a package import.
async fn create_verification_script(app_handle: &AppHandle<Wry>, package_name: &str) -> Result<PathBuf, String> {
    let script_content = format!(
r#"
import sys
try:
    import {package}
    print(f"Successfully imported {package}")
    sys.exit(0)
except ImportError as e:
    print(f"Failed to import {package}: {{e}}", file=sys.stderr)
    sys.exit(1)
"#,
        package = package_name
    );

    let temp_dir = app_handle.path().app_cache_dir().map_err(|e| e.to_string())?;
    if !temp_dir.exists() {
        tokio::fs::create_dir_all(&temp_dir).await.map_err(|e| e.to_string())?;
    }
    let script_path = temp_dir.join(format!("verify_{}.py", package_name));
    
    let mut file = tokio::fs::File::create(&script_path).await.map_err(|e| e.to_string())?;
    file.write_all(script_content.as_bytes()).await.map_err(|e| e.to_string())?;
    
    Ok(script_path)
}


/// Checks if a Python package can be imported by running a specific script in the ComfyUI venv.
pub async fn check_python_package_import(
    app_handle: &AppHandle<Wry>,
    package_name_for_log: &str, // e.g., "onnxruntime"
    venv_python_executable: &Path,
    comfyui_base_path: &Path, // For working directory
) -> Result<(), String> {
    let step_name = format!("Verifying {} import", package_name_for_log);
    info!("[VERIFY] Starting: {}", step_name);
    app_handle.emit(EVT_VERIFICATION_STEP_START, json!({ "stepName": step_name.clone() })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_START, e))?;

    let script_path = create_verification_script(app_handle, package_name_for_log).await?;
    info!("[VERIFY] Using dynamically created script: {} for {}", script_path.display(), package_name_for_log);
    info!("[VERIFY] Using Python executable: {}", venv_python_executable.display());
    info!("[VERIFY] Using ComfyUI base path as CWD: {}", comfyui_base_path.display());

    if !venv_python_executable.exists() {
        let err_msg = format!("Python executable for venv not found at {}", venv_python_executable.display());
        error!("[VERIFY] FAILED (pre-check): {} - {}", step_name, err_msg);
        app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name.clone(), "error": err_msg.clone(), "details": null })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_FAILED, e))?;
        return Err(err_msg);
    }
    if !script_path.exists() {
        let err_msg = format!("Verification script not found at {}", script_path.display());
        error!("[VERIFY] FAILED (pre-check): {} - {}", step_name, err_msg);
        app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name.clone(), "error": err_msg.clone(), "details": null })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_FAILED, e))?;
        return Err(err_msg);
    }

    let command = app_handle.shell().command(venv_python_executable.to_string_lossy().as_ref())
        .arg(&script_path)
        .current_dir(comfyui_base_path.to_string_lossy().as_ref());

    info!("[VERIFY] Executing managed command: {:?} with CWD: {}", command, comfyui_base_path.display());

    let result = ProcessManager::spawn_and_wait_for_process(
        app_handle,
        command,
        &format!("verify_import_{}", package_name_for_log)
    ).await.map_err(|e| {
        let err_msg = format!("Failed to spawn verification script for {}: {}", package_name_for_log, e);
        error!("[VERIFY] {}", err_msg);
        app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name.clone(), "error": err_msg.clone(), "details": null })).ok();
        err_msg
    })?;

    let success = result.exit_code.map_or(false, |c| c == 0) && result.signal.is_none();

    if success {
        let stdout_str = result.stdout.join("\n");
        info!("[VERIFY] SUCCESS: {} imported successfully. Output: {}", package_name_for_log, stdout_str);
        app_handle.emit(EVT_VERIFICATION_STEP_SUCCESS, json!({ "stepName": step_name.clone(), "details": stdout_str })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_SUCCESS, e))?;
        Ok(())
    } else {
        let stdout_str = result.stdout.join("\n");
        let stderr_str = result.stderr.join("\n");
        let err_msg = format!(
            "Failed to import {}. Exit code: {:?}. Stdout: [{}]. Stderr: [{}]",
            package_name_for_log,
            result.exit_code,
            stdout_str,
            stderr_str
        );
        error!("[VERIFY] FAILED: {} - {}", step_name, err_msg);
        app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name.clone(), "error": err_msg.clone(), "details": stderr_str })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_FAILED, e))?;
        Err(err_msg)
    }
}

/// Checks the integrity of the Python virtual environment and verifies key package imports.
/// This is a more thorough check than the quick file-based verification.
pub async fn check_python_environment_integrity(app_handle: &AppHandle<Wry>) -> Result<bool, String> {
    let step_name = VERIFICATION_EVENT_PYTHON_ENV;
    info!("[VERIFY] Starting: {}", step_name);
    app_handle.emit(EVT_VERIFICATION_STEP_START, json!({ "stepName": step_name })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_START, e))?;

    let comfyui_dir_result = get_comfyui_directory_path(app_handle);
    let comfyui_dir = match comfyui_dir_result {
        Ok(path) => path,
        Err(e) => {
            let err_msg = format!("Failed to get ComfyUI directory path: {}", e);
            warn!("[VERIFY] FAILED (pre-check): {} - {}", step_name, err_msg);
            app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name, "error": err_msg.clone(), "details": null })).map_err(|emit_err| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_FAILED, emit_err))?;
            return Ok(false); // Indicate verification failed, but not a critical error
        }
    };

    let venv_python_executable_result = get_conda_env_python_executable_path(app_handle, "comfyui_env").await;
    let venv_python_executable = match venv_python_executable_result {
        Ok(path) => path,
        Err(e) => {
            let err_msg = format!("Failed to get venv Python executable path: {}", e);
            warn!("[VERIFY] FAILED (pre-check): {} - {}", step_name, err_msg);
            app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name, "error": err_msg.clone(), "details": null })).map_err(|emit_err| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_FAILED, emit_err))?;
            return Ok(false); // Indicate verification failed, but not a critical error
        }
    };

    // 1. Check if Python executable exists within the Conda environment
    info!("[VERIFY] Checking for Python executable in Conda environment at {}", venv_python_executable.display());
    if !venv_python_executable.exists() || !venv_python_executable.is_file() {
        let err_msg = format!("Python executable for Conda environment not found at {}", venv_python_executable.display());
        warn!("[VERIFY] FAILED: {} - {}", step_name, err_msg);
        app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name, "error": err_msg.clone(), "details": null })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_FAILED, e))?;
        return Ok(false);
    }
    info!("[VERIFY] PASSED: Python executable exists in Conda environment.");

    // 2. Verify key package imports (e.g., torch, torchvision, numpy, etc.)
    // This requires running a Python script within the venv.
    // We can reuse the check_python_package_import function.

    // 3. Verify key package imports (e.g., torch, torchvision, numpy, etc.)
    // This requires running a Python script within the venv.
    // We can reuse the check_python_package_import function.
    let packages_to_verify = vec!["torch", "torchvision", "numpy", "requests", "Pillow"]; // Add other critical packages
    let mut all_packages_ok = true;
    let mut failed_packages = Vec::new();

    for package in packages_to_verify {
        match check_python_package_import(app_handle, package, &venv_python_executable, &comfyui_dir).await {
            Ok(_) => {
                info!("[VERIFY] PASSED: {} import successful.", package);
            }
            Err(e) => {
                let err_msg = format!("Failed to verify {} import: {}", package, e);
                warn!("[VERIFY] FAILED: {} - {}", step_name, err_msg);
                all_packages_ok = false;
                failed_packages.push(err_msg);
            }
        }
    }

    if all_packages_ok {
        info!("[VERIFY] SUCCESS: All key Python packages imported successfully. Python environment integrity check passed.");
        app_handle.emit(EVT_VERIFICATION_STEP_SUCCESS, json!({ "stepName": step_name, "details": "All key packages imported successfully." })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_SUCCESS, e))?;
        Ok(true)
    } else {
        let err_msg = format!("Python environment integrity check failed. Failed packages: {}", failed_packages.join(", "));
        warn!("[VERIFY] FAILED: {}", err_msg);
        app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name, "error": err_msg.clone(), "details": failed_packages.join("\n") })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_FAILED, e))?;
        Ok(false)
    }
}

/// Checks if all core model files exist in the ComfyUI models directory.
pub async fn check_core_models_exist(app_handle: &AppHandle<Wry>) -> Result<bool, String> {
    let step_name = "Verifying core models existence";
    info!("[VERIFY] Starting: {}", step_name);
    app_handle.emit(EVT_VERIFICATION_STEP_START, json!({ "stepName": step_name })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_START, e))?;

    let comfyui_dir = get_comfyui_directory_path(app_handle)?;
    let comfyui_models_base_path = comfyui_dir.join("models");
    let core_models = crate::setup_manager::get_core_models_list(); // Use the function from setup_manager

    if core_models.is_empty() {
        info!("[VERIFY] No core models configured. Skipping check.");
        app_handle.emit(EVT_VERIFICATION_STEP_SUCCESS, json!({ "stepName": step_name, "details": "No core models configured." })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_SUCCESS, e))?;
        return Ok(true); // No models to check means they "exist" in a sense
    }

    let mut all_models_exist = true;
    let mut missing_models = Vec::new();

    for model in core_models {
        let model_path = comfyui_models_base_path.join(&model.target_subdir).join(&model.target_filename);
        info!("[VERIFY] Checking for model file: {}", model_path.display());
        if !model_path.exists() || !model_path.is_file() {
            warn!("[VERIFY] MISSING: Model file not found at {}", model_path.display());
            all_models_exist = false;
            missing_models.push(model.target_filename.clone());
        } else {
            info!("[VERIFY] FOUND: Model file exists at {}", model_path.display());
        }
    }

    if all_models_exist {
        info!("[VERIFY] SUCCESS: All core model files found. Core models existence check passed.");
        app_handle.emit(EVT_VERIFICATION_STEP_SUCCESS, json!({ "stepName": step_name, "details": "All core model files found." })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_SUCCESS, e))?;
        Ok(true)
    } else {
        let err_msg = format!("Core model files missing: {}", missing_models.join(", "));
        warn!("[VERIFY] FAILED: {}", err_msg);
        app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name, "error": err_msg.clone(), "details": missing_models.join("\n") })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_FAILED, e))?;
        Ok(false)
    }
}