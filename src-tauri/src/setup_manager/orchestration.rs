// metamorphosis-app/src-tauri/src/setup_manager/orchestration.rs
use tauri::{AppHandle, Manager, Wry, Emitter}; // Emitter might not be directly used here but good to have if needed
use log::{error, info};
use std::fs;
use std::time::Duration;
use tokio::time::sleep;

use super::event_utils::emit_setup_progress;
use super::types::SetupStatusEvent;
use super::verification::{check_initialization_status, run_quick_verification};

// Note: comfyui_sidecar and dependency_management are kept as crate level for now.
// If they are also refactored into managers, these paths would change.
use crate::dependency_management;
use crate::comfyui_sidecar;


/// The main entry point command to determine setup status and initialize if necessary.
#[tauri::command]
pub async fn get_setup_status_and_initialize(app_handle: AppHandle<Wry>) -> Result<(), String> {
    info!("[SETUP_ORCHESTRATION] get_setup_status_and_initialize called.");
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let master_marker_path = app_data_dir.join("metamorphosis_setup_complete.marker");

    if master_marker_path.exists() {
        info!("[SETUP_ORCHESTRATION] Master Installation Marker found at {}. Performing quick verification.", master_marker_path.display());
        match run_quick_verification(&app_handle).await {
            Ok(true) => {
                info!("[SETUP_ORCHESTRATION] Quick verification PASSED.");
                app_handle.emit("setup_status", SetupStatusEvent::BackendFullyVerifiedAndReady).map_err(|e| e.to_string())?;
                info!("[SETUP_ORCHESTRATION] Emitted BackendFullyVerifiedAndReady.");
            }
            Ok(false) => {
                info!("[SETUP_ORCHESTRATION] Quick verification FAILED. Invalidating master marker.");
                if let Err(e) = fs::remove_file(&master_marker_path) {
                    error!("[SETUP_ORCHESTRATION] Failed to delete master marker file at {}: {}", master_marker_path.display(), e);
                    // Not returning error here, will proceed to emit full_setup_required
                } else {
                    info!("[SETUP_ORCHESTRATION] Master marker file deleted: {}", master_marker_path.display());
                }
                app_handle.emit("setup_status", SetupStatusEvent::FullSetupRequired { reason: "Quick verification failed.".to_string() }).map_err(|e| e.to_string())?;
                info!("[SETUP_ORCHESTRATION] Emitted FullSetupRequired (reason: verification failed).");
            }
            Err(e) => {
                error!("[SETUP_ORCHESTRATION] Error during quick verification: {}. Assuming full setup required and invalidating marker.", e);
                 if master_marker_path.exists() {
                    if let Err(remove_err) = fs::remove_file(&master_marker_path) {
                        error!("[SETUP_ORCHESTRATION] Failed to delete master marker file at {}: {}", master_marker_path.display(), remove_err);
                    } else {
                        info!("[SETUP_ORCHESTRATION] Master marker file deleted due to verification error: {}", master_marker_path.display());
                    }
                }
                app_handle.emit("setup_status", SetupStatusEvent::FullSetupRequired { reason: format!("Error during verification: {}", e) }).map_err(|e| e.to_string())?;
                info!("[SETUP_ORCHESTRATION] Emitted FullSetupRequired (reason: verification error).");
            }
        }
    } else {
        info!("[SETUP_ORCHESTRATION] Master Installation Marker NOT found at {}. Full setup required.", master_marker_path.display());
        app_handle.emit("setup_status", SetupStatusEvent::FullSetupRequired { reason: "New installation or previous setup incomplete/corrupted.".to_string() }).map_err(|e| e.to_string())?;
        info!("[SETUP_ORCHESTRATION] Emitted FullSetupRequired (reason: new installation).");
    }
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

    let mut comfyui_was_already_running_and_assumed_healthy = false;

    if comfyui_sidecar::is_comfyui_process_active() {
        info!("[SETUP_ORCHESTRATION] ComfyUI process is already active. Assuming it's from quick verification and healthy. Skipping stop/restart.");
        comfyui_was_already_running_and_assumed_healthy = true;
    } else {
        info!("[SETUP_ORCHESTRATION] No active ComfyUI process found. Proceeding with stop (no-op) and start.");
        // Attempt to stop any existing ComfyUI sidecar process first
        info!("[SETUP_ORCHESTRATION] Attempting to stop any pre-existing ComfyUI sidecar process...");
        comfyui_sidecar::stop_comfyui_sidecar();
        info!("[SETUP_ORCHESTRATION] Pre-existing ComfyUI sidecar stop attempt complete.");
    }

    // Phase 1: Checking (Initial system checks, disk space etc.)
    emit_setup_progress(&app_handle, "checking", "Starting system checks...", 0, None, None);
    
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
    if !comfyui_was_already_running_and_assumed_healthy {
        info!("[SETUP_ORCHESTRATION] ComfyUI was not already running or assumed healthy. Starting ComfyUI services...");
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
    } else {
        info!("[SETUP_ORCHESTRATION] Skipping ComfyUI service start in orchestrate_full_setup as it was already running.");
        emit_setup_progress(&app_handle, "finalizing", "ComfyUI services already running and assumed healthy.", 100, None, None);
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