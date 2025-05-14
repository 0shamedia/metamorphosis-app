use tauri::WebviewWindow;
use tauri::{Window, AppHandle, Manager, Wry}; // Import Manager, Wry
use tauri::Emitter; // Add this to import the Emitter trait
use serde_json::json;
use log::{error, info};
use std::time::Duration;
use tokio::time::sleep;
use std::path::PathBuf; // Added for path operations
use std::fs; // Added for directory creation
use serde::Serialize; // For SetupProgressPayload

use crate::dependency_management; // For calling actual installation
use crate::comfyui_sidecar; // For calling actual sidecar start

// Unified Setup Progress Payload
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SetupProgressPayload {
    phase: String,
    current_step: String,
    progress: u8, // 0-100
    #[serde(skip_serializing_if = "Option::is_none")]
    detail_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

// Helper to emit unified setup progress
pub fn emit_setup_progress(
    app_handle: &AppHandle<Wry>,
    phase: &str,
    current_step: &str,
    progress: u8,
    detail_message: Option<String>,
    error: Option<String>,
) {
    let payload = SetupProgressPayload {
        phase: phase.to_string(),
        current_step: current_step.to_string(),
        progress,
        detail_message,
        error,
    };
    if let Err(e) = app_handle.emit("setup-progress", payload) {
        error!("Failed to emit setup-progress event: {}", e);
    }
}


// Setup phases (kept for reference, but string literals will be used in emit_setup_progress)
#[derive(Debug, Clone, serde::Serialize)]
pub enum SetupPhase {
    Checking,
    InstallingComfyui,
    PythonSetup,
    DownloadingModels,
    Finalizing,
    Complete,
    Error,
}

// Model download status (may become obsolete if model download is fully integrated into setup-progress)
#[derive(Debug, Clone, serde::Serialize)]
pub enum ModelStatus {
    Queued,
    Downloading,
    Verifying,
    Completed,
    Error,
}

// Model information (may become obsolete)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub progress: f32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}


#[tauri::command]
pub async fn check_initialization_status(window: WebviewWindow) -> Result<(), String> {
    // Record start time for performance tracking
    let start_time = std::time::Instant::now();
    info!("[SETUP] check_initialization_status started");
    
    // Log system information for diagnostics
    info!("[SETUP] System information:");
    info!("[SETUP] OS: {}", std::env::consts::OS);
    info!("[SETUP] Arch: {}", std::env::consts::ARCH);
    info!("[SETUP] Current dir: {:?}", std::env::current_dir().unwrap_or_default());
    
    // Make sure the window is visible first
    info!("[SETUP] Attempting to show window...");
    let show_start = std::time::Instant::now();
    
    match window.show() {
        Ok(_) => {
            let elapsed = show_start.elapsed();
            info!("[SETUP] Window successfully shown in {:?}", elapsed);
        },
        Err(e) => {
            error!("[SETUP] Error showing window: {} (after {:?})", e, show_start.elapsed());
            return Err(format!("Failed to show window: {}", e));
        }
    }
    
    // Check for window dimensions
    match window.inner_size() {
        Ok(size) => {
            info!("[SETUP] Window dimensions: {}x{}", size.width, size.height);
        },
        Err(e) => {
            error!("[SETUP] Error getting window dimensions: {}", e);
        }
    }
    
    // Send initial status - we're initializing
    info!("[SETUP] Emitting initializing status...");
    let emit_start = std::time::Instant::now();
    
    match window.emit("initialization-status", json!({
        "status": "initializing",
        "message": "Initializing Metamorphosis..."
    })) {
        Ok(_) => {
            let elapsed = emit_start.elapsed();
            info!("[SETUP] Successfully emitted initializing status in {:?}", elapsed);
        },
        Err(e) => {
            error!("[SETUP] Error emitting initialization status: {} (after {:?})", e, emit_start.elapsed());
            return Err(format!("Failed to emit status: {}", e));
        }
    }
    
    // Here we could check if there are any required files that need to be present
    // For now, we'll just simulate a brief check
    info!("[SETUP] Performing initialization checks...");
    let check_start = std::time::Instant::now();
    
    // info!("[SETUP] Simulating initialization check (sleeping for 1500ms)...");
    // sleep(Duration::from_millis(1500)).await;

    let app_handle = window.app_handle();

    // Check 1: Verify Application Data Directory
    info!("[SETUP] Check 1: Verifying Application Data Directory...");
    match app_handle.path().app_data_dir() {
        Ok(app_data_path) => {
            if !app_data_path.exists() {
                if let Err(e) = fs::create_dir_all(&app_data_path) {
                    let error_msg = format!("Failed to create app data directory at {:?}: {}", app_data_path, e);
                    error!("[SETUP] {}", error_msg);
                    window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
                    return Err(error_msg);
                }
                info!("[SETUP] Created app data directory at {:?}", app_data_path);
            } else {
                info!("[SETUP] App data directory verified at {:?}", app_data_path);
            }
            window.emit("initialization-status", json!({ "status": "progress", "stage": "VerifyingAppDataDir", "progress": 25, "message": "Verifying application data..." })).map_err(|e| e.to_string())?;
        }
        Err(e) => {
            let error_msg = format!("Failed to resolve application data directory path: {}", e);
            error!("[SETUP] {}", error_msg);
            window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
            return Err(error_msg);
        }
    }
    info!("[SETUP] Check 1 completed in {:?}", check_start.elapsed());
    let check_2_start = std::time::Instant::now();

    let check_2_start = std::time::Instant::now();

    // Check 2: Check Python Executable Path
    info!("[SETUP] Check 2: Checking Python Executable Path...");
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
                info!("[SETUP] Python executable path verified at {:?}", python_path);
                window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingPythonPath", "progress": 50, "message": "Verifying Python environment..." })).map_err(|e| e.to_string())?;
            } else {
                let error_msg = format!("Python executable not found or is not a file at resolved path: {:?}", python_path);
                error!("[SETUP] {}", error_msg);
                window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
                return Err(error_msg);
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to determine Python executable path: {}", e);
            error!("[SETUP] {}", error_msg);
            window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
            return Err(error_msg);
        }
    }
    info!("[SETUP] Check 2 completed in {:?}", check_2_start.elapsed());
    let check_3_start = std::time::Instant::now();

    let check_3_start = std::time::Instant::now();

    // Check 3: Check ComfyUI Directory Path
    info!("[SETUP] Check 3: Checking ComfyUI Directory Path...");
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
                info!("[SETUP] ComfyUI directory path verified at {:?}", comfyui_path);
                window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingComfyUIPath", "progress": 75, "message": "Verifying ComfyUI components..." })).map_err(|e| e.to_string())?;
            } else {
                let error_msg = format!("ComfyUI directory not found or is not a directory at resolved path: {:?}", comfyui_path);
                error!("[SETUP] {}", error_msg);
                window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
                return Err(error_msg);
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to determine ComfyUI directory path: {}", e);
            error!("[SETUP] {}", error_msg);
            window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
            return Err(error_msg);
        }
    }
    info!("[SETUP] Check 3 completed in {:?}", check_3_start.elapsed());
    info!("[SETUP] All initialization checks completed in {:?}", check_start.elapsed());

    // Send ready status
    info!("[SETUP] Emitting ready status...");
    let ready_emit_start = std::time::Instant::now();

    match window.emit("initialization-status", json!({
        "status": "ready",
        "message": "Initialization complete. Ready to proceed."
    })) {
        Ok(_) => {
            let elapsed = ready_emit_start.elapsed();
            info!("[SETUP] Successfully emitted ready status in {:?}", elapsed);
        },
        Err(e) => {
            error!("[SETUP] Error emitting ready status: {} (after {:?})", e, ready_emit_start.elapsed());
            return Err(format!("Failed to emit ready status: {}", e));
        }
    }

    let total_elapsed = start_time.elapsed();
    info!("[SETUP] Initialization status check complete in {:?}", total_elapsed);
    Ok(())
}

/// Start the application setup process
#[tauri::command]
pub async fn start_application_setup(app_handle: AppHandle<Wry>) -> Result<(), String> {
    // Spawn the setup process in the background
    let handle_clone = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = orchestrate_full_setup(handle_clone.clone()).await { // Clone handle_clone for orchestrate_full_setup
            error!("Full setup orchestration failed: {}", e);
            // Notify the frontend of the error using the new helper
             emit_setup_progress(
                &handle_clone, // Use the cloned handle for emitting error
                "error",
                "Critical Setup Error",
                0,
                Some("The application setup encountered a critical error and could not complete.".to_string()),
                Some(e.clone()), // Send the error message
            );
        }
    });
    
    Ok(())
}

/// Orchestrates the entire application setup process.
async fn orchestrate_full_setup(app_handle: AppHandle<Wry>) -> Result<(), String> {
    info!("Starting full application setup orchestration...");

    // Attempt to stop any existing ComfyUI sidecar process first
    info!("[SETUP] Attempting to stop any pre-existing ComfyUI sidecar process...");
    comfyui_sidecar::stop_comfyui_sidecar(); // Call the public stop function
    info!("[SETUP] Pre-existing ComfyUI sidecar stop attempt complete.");

    // Phase 1: Checking (Initial system checks, disk space etc.)
    emit_setup_progress(&app_handle, "checking", "Starting system checks...", 0, None, None);
    // Actual checks from check_initialization_status can be integrated or called here.
    // For now, simulate a brief check.
    // Getting the main window to pass to check_initialization_status
    let main_window = app_handle.get_webview_window("main").ok_or_else(|| {
        let msg = "Failed to get main window for initial checks".to_string();
        error!("{}", msg);
        msg
    })?;
    match check_initialization_status(main_window).await {
        Ok(_) => emit_setup_progress(&app_handle, "checking", "System checks complete.", 100, None, None),
        Err(e) => {
            let err_msg = format!("Initial system checks failed: {}", e);
            error!("{}", err_msg);
            emit_setup_progress(&app_handle, "error", "System Check Failed", 0, Some(err_msg.clone()), Some(e));
            return Err(err_msg);
        }
    }

    // Phase 2 & 3: Python Environment & ComfyUI Dependencies
    emit_setup_progress(&app_handle, "python_setup", "Initializing Python environment setup...", 0, None, None);
    match dependency_management::install_python_dependencies_with_progress(&app_handle).await {
        Ok(_) => {
            info!("Python dependencies installed successfully.");
            emit_setup_progress(&app_handle, "python_setup", "Python environment setup complete.", 100, None, None);
        }
        Err(e) => {
            let err_msg = format!("Python dependency installation failed: {}", e);
            error!("{}", err_msg);
            emit_setup_progress(&app_handle, "error", "Python Setup Failed", 0, Some(err_msg.clone()), Some(e.to_string()));
            return Err(err_msg);
        }
    }

    // Phase 4: Downloading Models (Simulated for now, to be replaced with actual logic)
    emit_setup_progress(&app_handle, "downloading_models", "Preparing to download AI models...", 0, None, None);
    let models_to_download = vec!["Stable Diffusion v1.5", "VAE Model", "Character Base LoRA"];
    let num_models = models_to_download.len();
    if num_models > 0 { 
        for (idx, model_name) in models_to_download.iter().enumerate() {
            let phase_start_progress = ((idx * 100) / num_models) as u8;
            emit_setup_progress(&app_handle, "downloading_models", &format!("Starting download: {}", model_name), phase_start_progress, None, None);
            for progress_step in 1..=10 {
                sleep(Duration::from_millis(150)).await; 
                let model_progress_percent = progress_step * 10;
                let overall_phase_progress = (((idx * 100) + model_progress_percent) / num_models) as u8;
                emit_setup_progress(
                    &app_handle,
                    "downloading_models",
                    &format!("Downloading {} ({}%)", model_name, model_progress_percent),
                    overall_phase_progress.min(100), 
                    Some(format!("Downloading {} - {}MB / {}MB", model_name, model_progress_percent * 5, 500)), 
                    None,
                );
            }
            let phase_end_progress = (((idx + 1) * 100) / num_models) as u8;
            emit_setup_progress(&app_handle, "downloading_models", &format!("Finished download: {}", model_name), phase_end_progress.min(100), None, None);
        }
    }
    emit_setup_progress(&app_handle, "downloading_models", "All models downloaded.", 100, None, None);


    // Phase 5: Finalizing (Starting ComfyUI Sidecar and Health Check)
    emit_setup_progress(&app_handle, "finalizing", "Starting ComfyUI services...", 0, None, None);
    match comfyui_sidecar::spawn_and_health_check_comfyui(&app_handle).await {
        Ok(_) => {
            info!("ComfyUI services started and healthy.");
            emit_setup_progress(&app_handle, "finalizing", "ComfyUI services healthy.", 100, None, None);
        }
        Err(e) => {
            let err_msg = format!("Failed to start or health check ComfyUI services: {}", e);
            error!("{}", err_msg);
            emit_setup_progress(&app_handle, "error", "ComfyUI Service Failed", 0, Some(err_msg.clone()), Some(e.to_string()));
            return Err(err_msg);
        }
    }

    // Phase 6: Complete
    // Create Master Installation Marker File
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| {
        let err_msg = format!("Failed to get app data dir for master marker: {}", e);
        error!("{}", err_msg);
        err_msg
    })?;
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir).map_err(|e| {
            let err_msg = format!("Failed to create app data dir for master marker at {:?}: {}", app_data_dir, e);
            error!("{}", err_msg);
            err_msg
        })?;
    }
    let master_marker_path = app_data_dir.join("metamorphosis_setup_complete.marker");
    fs::write(&master_marker_path, "setup_completed_successfully").map_err(|e| {
        let err_msg = format!("Failed to write master installation marker at {:?}: {}", master_marker_path, e);
        error!("{}", err_msg);
        err_msg
    })?;
    info!("Master Installation Marker File created at {}", master_marker_path.display());

    emit_setup_progress(&app_handle, "complete", "Setup complete. Ready to launch!", 100, None, None);
    info!("Full application setup orchestration completed successfully.");
    Ok(())
}


/// Retry the application setup process
#[tauri::command]
pub async fn retry_application_setup(app_handle: AppHandle<Wry>) -> Result<(), String> {
    start_application_setup(app_handle).await
}

// The old run_setup_process function is now removed as its logic is replaced by orchestrate_full_setup
// and calls to actual implementation modules.

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum SetupStatusEvent {
    BackendFullyVerifiedAndReady,
    FullSetupRequired { reason: String },
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


async fn run_quick_verification(app_handle: &AppHandle<Wry>) -> Result<bool, String> {
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

#[tauri::command]
pub async fn get_setup_status_and_initialize(app_handle: AppHandle<Wry>) -> Result<(), String> {
    info!("[SETUP LIFECYCLE] get_setup_status_and_initialize called.");
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let master_marker_path = app_data_dir.join("metamorphosis_setup_complete.marker");

    if master_marker_path.exists() {
        info!("[SETUP LIFECYCLE] Master Installation Marker found at {}. Performing quick verification.", master_marker_path.display());
        match run_quick_verification(&app_handle).await {
            Ok(true) => {
                info!("[SETUP LIFECYCLE] Quick verification PASSED.");
                app_handle.emit("setup_status", SetupStatusEvent::BackendFullyVerifiedAndReady).map_err(|e| e.to_string())?;
                info!("[SETUP LIFECYCLE] Emitted BackendFullyVerifiedAndReady.");
            }
            Ok(false) => {
                info!("[SETUP LIFECYCLE] Quick verification FAILED. Invalidating master marker.");
                if let Err(e) = fs::remove_file(&master_marker_path) {
                    error!("[SETUP LIFECYCLE] Failed to delete master marker file at {}: {}", master_marker_path.display(), e);
                    // Not returning error here, will proceed to emit full_setup_required
                } else {
                    info!("[SETUP LIFECYCLE] Master marker file deleted: {}", master_marker_path.display());
                }
                app_handle.emit("setup_status", SetupStatusEvent::FullSetupRequired { reason: "Quick verification failed.".to_string() }).map_err(|e| e.to_string())?;
                info!("[SETUP LIFECYCLE] Emitted FullSetupRequired (reason: verification failed).");
            }
            Err(e) => {
                error!("[SETUP LIFECYCLE] Error during quick verification: {}. Assuming full setup required and invalidating marker.", e);
                 if master_marker_path.exists() {
                    if let Err(remove_err) = fs::remove_file(&master_marker_path) {
                        error!("[SETUP LIFECYCLE] Failed to delete master marker file at {}: {}", master_marker_path.display(), remove_err);
                    } else {
                        info!("[SETUP LIFECYCLE] Master marker file deleted due to verification error: {}", master_marker_path.display());
                    }
                }
                app_handle.emit("setup_status", SetupStatusEvent::FullSetupRequired { reason: format!("Error during verification: {}", e) }).map_err(|e| e.to_string())?;
                info!("[SETUP LIFECYCLE] Emitted FullSetupRequired (reason: verification error).");
            }
        }
    } else {
        info!("[SETUP LIFECYCLE] Master Installation Marker NOT found at {}. Full setup required.", master_marker_path.display());
        app_handle.emit("setup_status", SetupStatusEvent::FullSetupRequired { reason: "New installation or previous setup incomplete/corrupted.".to_string() }).map_err(|e| e.to_string())?;
        info!("[SETUP LIFECYCLE] Emitted FullSetupRequired (reason: new installation).");
    }
    Ok(())
}