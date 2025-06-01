// metamorphosis-app/src-tauri/src/setup_manager/verification.rs
use tauri::{WebviewWindow, AppHandle, Manager, Wry, Emitter};
use serde_json::json;
use log::{error, info, warn};
use std::path::Path; // PathBuf is no longer directly used here
use std::fs;
use std::process::{Command, Stdio};
use std::io::{BufReader, BufRead};
// use std::env; // No longer needed

// Import new python_utils functions
use crate::setup_manager::python_utils::{
    get_comfyui_directory_path,
    get_bundled_python_executable_path,
    get_venv_python_executable_path,
    get_script_path as get_util_script_path, // Alias to avoid conflict if a local one exists temporarily
};

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

    // Check 2: Check Python Executable Path
    info!("[SETUP_VERIFICATION] Check 2: Checking Python Executable Path...");
    let python_executable_path_result = get_bundled_python_executable_path(&app_handle);

    match python_executable_path_result {
        Ok(python_path) => {
            if python_path.exists() && python_path.is_file() {
                info!("[SETUP_VERIFICATION] Python executable path verified at {:?}", python_path);
                window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingPythonPath", "progress": 50, "message": "Python executable found." })).map_err(|e| e.to_string())?;
            } else {
                let warning_msg = format!("Bundled Python executable not found or is not a file at resolved path: {:?}. This is expected on first run and will be installed.", python_path);
                warn!("[SETUP_VERIFICATION] {}", warning_msg);
                // Emit a warning status or just log and continue. Let's just log and continue for now,
                // as the full setup will handle the installation.
                window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingPythonPath", "progress": 50, "message": warning_msg })).map_err(|e| e.to_string())?;
            }
        }
        Err(e) => {
            let warning_msg = format!("Failed to determine bundled Python executable path: {}. This is expected on first run and will be installed.", e);
            warn!("[SETUP_VERIFICATION] {}", warning_msg);
            // Emit a warning status or just log and continue. Let's just log and continue for now.
            window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingPythonPath", "progress": 50, "message": warning_msg })).map_err(|e| e.to_string())?;
        }
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
    let _venv_python_executable = get_venv_python_executable_path(app_handle)?;
    // venv_dir can be derived if needed: venv_python_executable.parent().unwrap().parent().unwrap()
    // For the check, we primarily need comfyui_dir and venv_python_executable.
    // Let's get venv_dir explicitly for the check.
    let _venv_dir = comfyui_dir.join(".venv");


    // Quick verification should only check for the presence of core files/directories placed by the build script.
    // The .venv and its contents are created during the full setup, so they should not be checked here.

    // 1. Check for vendor/python directory
    let python_dir = get_bundled_python_executable_path(app_handle)?.parent().ok_or("Failed to get parent of python executable")?.to_path_buf();
    info!("[QUICK VERIFY] Checking for vendor/python directory at {}", python_dir.display());
    if !python_dir.exists() || !python_dir.is_dir() {
        info!("[QUICK VERIFY] FAILED: vendor/python directory not found at {}", python_dir.display());
        return Ok(false);
    }
    info!("[QUICK VERIFY] PASSED: vendor/python directory found.");

    // 2. Check for vendor/python/python.exe (or equivalent)
    let python_executable = get_bundled_python_executable_path(app_handle)?;
    info!("[QUICK VERIFY] Checking for Python executable at {}", python_executable.display());
    if !python_executable.exists() || !python_executable.is_file() {
        info!("[QUICK VERIFY] FAILED: Python executable not found at {}", python_executable.display());
        return Ok(false);
    }
    info!("[QUICK VERIFY] PASSED: Python executable exists.");

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


/// Checks if a Python package can be imported by running a specific script in the ComfyUI venv.
pub async fn check_python_package_import(
    app_handle: &AppHandle<Wry>,
    package_name_for_log: &str, // e.g., "onnxruntime"
    script_name: &str,          // e.g., "script_check_onnx.py"
    venv_python_executable: &Path,
    comfyui_base_path: &Path, // For working directory
) -> Result<(), String> {
    let step_name = format!("Verifying {} import", package_name_for_log);
    info!("[VERIFY] Starting: {}", step_name);
    app_handle.emit(EVT_VERIFICATION_STEP_START, json!({ "stepName": step_name.clone() })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_START, e))?;

    let script_path = get_util_script_path(app_handle, script_name)?; // Use aliased util function
    info!("[VERIFY] Using script: {} for {}", script_path.display(), package_name_for_log);
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

    let mut command = Command::new(venv_python_executable);
    command.arg(&script_path);
    command.current_dir(comfyui_base_path);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    info!("[VERIFY] Executing command: {:?} with CWD: {}", command, comfyui_base_path.display());

    let mut child = command.spawn().map_err(|e| {
        let err_msg = format!("Failed to spawn verification script for {}: {}", package_name_for_log, e);
        error!("[VERIFY] {}", err_msg);
        // Emit event for spawn failure
        app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name.clone(), "error": err_msg.clone(), "details": null })).unwrap_or_else(|emit_err| error!("Failed to emit {} event after spawn error: {}", EVT_VERIFICATION_STEP_FAILED, emit_err));
        err_msg
    })?;

    let mut stdout_lines = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
                    info!("[VERIFY][{}_stdout] {}", package_name_for_log, line);
                    stdout_lines.push(line);
                }
                Err(e) => warn!("[VERIFY][{}_stdout] Error reading line: {}", package_name_for_log, e),
            }
        }
    }

    let mut stderr_lines = Vec::new();
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
                    error!("[VERIFY][{}_stderr] {}", package_name_for_log, line); // Log stderr as error
                    stderr_lines.push(line);
                }
                Err(e) => warn!("[VERIFY][{}_stderr] Error reading line: {}", package_name_for_log, e),
            }
        }
    }
    
    let status = child.wait().map_err(|e| {
        let err_msg = format!("Failed to wait for verification script for {}: {}", package_name_for_log, e);
        error!("[VERIFY] {}", err_msg);
         app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name.clone(), "error": err_msg.clone(), "details": null })).unwrap_or_else(|emit_err| error!("Failed to emit {} event after wait error: {}", EVT_VERIFICATION_STEP_FAILED, emit_err));
        err_msg
    })?;

    let stdout_str = stdout_lines.join("\n");
    let stderr_str = stderr_lines.join("\n");

    if status.success() {
        info!("[VERIFY] SUCCESS: {} imported successfully. Output: {}", package_name_for_log, stdout_str);
        app_handle.emit(EVT_VERIFICATION_STEP_SUCCESS, json!({ "stepName": step_name.clone(), "details": stdout_str })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_SUCCESS, e))?;
        Ok(())
    } else {
        let err_msg = format!(
            "Failed to import {}. Exit code: {}. Stdout: [{}]. Stderr: [{}]",
            package_name_for_log,
            status.code().map_or_else(|| "N/A".to_string(), |c| c.to_string()),
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

    let venv_python_executable_result = get_venv_python_executable_path(app_handle);
    let venv_python_executable = match venv_python_executable_result {
        Ok(path) => path,
        Err(e) => {
            let err_msg = format!("Failed to get venv Python executable path: {}", e);
            warn!("[VERIFY] FAILED (pre-check): {} - {}", step_name, err_msg);
            app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name, "error": err_msg.clone(), "details": null })).map_err(|emit_err| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_FAILED, emit_err))?;
            return Ok(false); // Indicate verification failed, but not a critical error
        }
    };

    let venv_dir = comfyui_dir.join(".venv");

    // 1. Check if .venv directory exists and is a directory
    info!("[VERIFY] Checking for .venv directory at {}", venv_dir.display());
    if !venv_dir.exists() || !venv_dir.is_dir() {
        let err_msg = format!(".venv directory not found or is not a directory at {}", venv_dir.display());
        warn!("[VERIFY] FAILED: {} - {}", step_name, err_msg);
        app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name, "error": err_msg.clone(), "details": null })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_FAILED, e))?;
        return Ok(false);
    }
    info!("[VERIFY] PASSED: .venv directory found.");

    // 2. Check if Python executable exists within .venv
    info!("[VERIFY] Checking for Python executable in .venv at {}", venv_python_executable.display());
    if !venv_python_executable.exists() || !venv_python_executable.is_file() {
        let err_msg = format!("Python executable not found in .venv at {}", venv_python_executable.display());
        warn!("[VERIFY] FAILED: {} - {}", step_name, err_msg);
        app_handle.emit(EVT_VERIFICATION_STEP_FAILED, json!({ "stepName": step_name, "error": err_msg.clone(), "details": null })).map_err(|e| format!("Failed to emit {}: {}", EVT_VERIFICATION_STEP_FAILED, e))?;
        return Ok(false);
    }
    info!("[VERIFY] PASSED: Python executable exists in .venv.");

    // 3. Verify key package imports (e.g., torch, torchvision, numpy, etc.)
    // This requires running a Python script within the venv.
    // We can reuse the check_python_package_import function.
    let packages_to_verify = vec!["torch", "torchvision", "numpy", "requests", "Pillow"]; // Add other critical packages
    let mut all_packages_ok = true;
    let mut failed_packages = Vec::new();

    for package in packages_to_verify {
        let script_name = format!("script_check_{}.py", package.replace("-", "_")); // e.g., script_check_torch.py
        // Create a temporary script file to check import
        let script_content = format!("import {}\nprint('{} import successful')", package, package);
        let temp_script_path = comfyui_dir.join(&script_name); // Place temp script in comfyui dir

        if let Err(e) = tokio::fs::write(&temp_script_path, script_content).await {
            let err_msg = format!("Failed to write temporary verification script {}: {}", script_name, e);
            error!("[VERIFY] {}", err_msg);
            // This is a critical error, but we'll continue checking other packages for now.
            all_packages_ok = false;
            failed_packages.push(format!("Script write failed for {}: {}", package, e));
            continue; // Skip to next package
        }

        match check_python_package_import(app_handle, package, &script_name, &venv_python_executable, &comfyui_dir).await {
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

        // Clean up the temporary script file
        if let Err(e) = tokio::fs::remove_file(&temp_script_path).await {
            warn!("[VERIFY] Failed to remove temporary verification script {}: {}", script_name, e);
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