// metamorphosis-app/src-tauri/src/setup_manager/orchestration.rs
use tauri::{AppHandle, Manager, Wry}; // Emitter might not be directly used here but good to have if needed
use log::{error, info, warn}; // Added warn
use std::fs;
use tauri::Emitter; // Added Emitter

use super::event_utils::emit_setup_progress;
use super::types::SetupStatusEvent;
// Updated verification imports
use super::verification::{
    check_initialization_status, run_quick_verification, check_ipadapter_plus_directory_exists, check_python_package_import
    // get_comfyui_vendor_paths, // This will be replaced by python_utils
};
// Import new python_utils functions
use crate::setup_manager::python_utils::{
    get_comfyui_directory_path, get_venv_python_executable_path
    // get_bundled_python_executable_path, // Not directly used here, but available
    // get_script_path, // Not directly used here
    // get_vendor_path, // No longer directly used here, comfyui_directory_path is used
};
use crate::setup_manager::{get_core_models_list, download_and_place_models}; // Uncommented and added functions
use crate::setup_manager::custom_node_management; // Uncommented and added custom_node_management
use crate::setup_manager::dependency_manager; // Changed from crate::dependency_management
 
// Note: comfyui_sidecar is kept as crate level for now.
// If it is also refactored into managers, these paths would change.
// use crate::comfyui_sidecar; // Removed direct import
use crate::sidecar_manager::spawn_and_health_check_comfyui; // Imported function directly
use crate::sidecar_manager::process_handler::{is_comfyui_process_active, stop_comfyui_sidecar}; // Imported functions directly


/// The main entry point command to determine setup status and initialize if necessary.
#[tauri::command]
pub async fn get_setup_status_and_initialize(app_handle: AppHandle<Wry>) -> Result<(), String> {
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

    // Original logic (commented out for testing):
    /*
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
    */
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

    if is_comfyui_process_active() { // Changed call to use direct import
        info!("[SETUP_ORCHESTRATION] ComfyUI process is already active. Assuming it's from quick verification and healthy. Skipping stop/restart.");
        comfyui_was_already_running_and_assumed_healthy = true;
    } else {
        info!("[SETUP_ORCHESTRATION] No active ComfyUI process found. Proceeding with stop (no-op) and start.");
        // Attempt to stop any existing ComfyUI sidecar process first
        info!("[SETUP_ORCHESTRATION] Attempting to stop any pre-existing ComfyUI sidecar process...");
        stop_comfyui_sidecar(); // Changed call to use direct import
        info!("[SETUP_ORCHESTRATION] Pre-existing ComfyUI sidecar stop attempt complete.");
    }

    // Phase 1: Checking (Initial system checks, disk space etc.)
    emit_setup_progress(&app_handle, "checking", "Running system checks", 0, Some("Checking system requirements and environment...".to_string()), None);
    
    let main_window = app_handle.get_webview_window("main").ok_or_else(|| {
        let msg = "Failed to get main window for initial checks".to_string();
        error!("{}", msg);
        msg
    })?;
    match check_initialization_status(main_window).await {
        Ok(_) => emit_setup_progress(&app_handle, "checking", "System checks complete", 100, Some("All system requirements met.".to_string()), None),
        Err(e) => {
            let err_msg = format!("Initial system checks failed: {}", e);
            error!("{}", err_msg);
            emit_setup_progress(&app_handle, "error", "System Check Failed", 0, Some(err_msg.clone()), Some(e));
            return Err(err_msg);
        }
    }
 
     // Phase 2 & 3: Python Environment & ComfyUI Dependencies
     emit_setup_progress(&app_handle, "python_setup", "Setting up Python environment", 0, Some("Initializing Python virtual environment and dependencies...".to_string()), None);
 
 
     // Check if Python environment is already set up and dependencies are installed
     match super::verification::check_python_environment_integrity(&app_handle).await {
         Ok(true) => {
             info!("[SETUP_ORCHESTRATION] Python environment and dependencies already verified. Skipping installation.");
             emit_setup_progress(&app_handle, "python_setup", "Python environment ready", 100, Some("Python environment and dependencies are already set up.".to_string()), None);
         }
         Ok(false) => {
             info!("[SETUP_ORCHESTRATION] Python environment verification failed. Proceeding with installation.");
             match dependency_manager::install_python_dependencies_with_progress(&app_handle).await {
                 Ok(_) => {
                     info!("Python dependencies installed successfully.");
                 }
                 Err(e) => {
                     let err_msg = format!("Python dependency installation failed: {}", e);
                     error!("{}", err_msg);
                     emit_setup_progress(&app_handle, "error", "Python Setup Failed", 0, Some(err_msg.clone()), Some(e.to_string()));
                     return Err(err_msg);
                 }
             }
         }
         Err(e) => {
             let err_msg = format!("Error during Python environment verification: {}", e);
             error!("{}", err_msg);
             emit_setup_progress(&app_handle, "error", "Python Verification Error", 0, Some(err_msg.clone()), Some(e.to_string()));
             return Err(err_msg);
         }
     }
 
      // Phase: Installing Custom Nodes
     // This phase is added before model downloading, as custom nodes might define model locations or types.
     emit_setup_progress(&app_handle, "installing_custom_nodes", "Setting up custom nodes", 0, Some("Cloning and installing required custom nodes...".to_string()), None);
     
     // Check if IPAdapter Plus directory exists before attempting to clone
     let comfyui_dir_for_ipadapter_check = get_comfyui_directory_path(&app_handle)?;
     match super::verification::check_ipadapter_plus_directory_exists(&app_handle, &comfyui_dir_for_ipadapter_check).await {
         Ok(true) => {
             info!("[SETUP_ORCHESTRATION] ComfyUI_IPAdapter_plus directory already verified. Skipping clone.");
             emit_setup_progress(&app_handle, "installing_custom_nodes", "IPAdapter+ node already exists", 100, Some("ComfyUI_IPAdapter_plus node is already installed.".to_string()), None);
         }
         Ok(false) | Err(_) => { // Proceed with clone if not found or error during check
             info!("[SETUP_ORCHESTRATION] ComfyUI_IPAdapter_plus directory verification failed or directory not found. Proceeding with clone.");
             match custom_node_management::clone_comfyui_ipadapter_plus(&app_handle).await {
                 Ok(_) => {
                     info!("ComfyUI_IPAdapter_plus cloned successfully or already exists.");
                 }
                 Err(e) => {
                     let err_msg = format!("Failed to setup ComfyUI_IPAdapter_plus: {}", e);
                     error!("{}", err_msg);
                     // It's a non-critical error for now, so we log it and continue.
                     // We'll emit a specific step error within the "installing_custom_nodes" phase.
                     emit_setup_progress(
                         &app_handle,
                         "installing_custom_nodes", // Keep the phase
                         "IPAdapter+ Clone Failed", // Specific step that failed
                         50, // Indicate partial progress or an issue within this phase
                         Some(err_msg.clone()), // Detail message for the frontend
                         Some(e.to_string()), // Error string
                     );
                     warn!("Continuing setup despite custom node ComfyUI_IPAdapter_plus failing to clone: {}", e);
                 }
             }
         }
      }
 
      // Install ComfyUI-Impact-Pack
      info!("[SETUP_ORCHESTRATION] Attempting to clone ComfyUI-Impact-Pack...");
      match custom_node_management::clone_comfyui_impact_pack(&app_handle).await {
          Ok(_) => {
              info!("ComfyUI-Impact-Pack cloned successfully or already exists and dependencies checked/installed.");
              emit_setup_progress(&app_handle, "installing_custom_nodes", "Impact Pack Setup Complete", 75, Some("ComfyUI-Impact-Pack processed.".to_string()), None);
          }
          Err(e) => {
              let err_msg = format!("Failed to setup ComfyUI-Impact-Pack: {}", e);
              error!("{}", err_msg);
              emit_setup_progress(&app_handle, "installing_custom_nodes", "Impact Pack Setup Failed", 60, Some(err_msg.clone()), Some(e.to_string()));
              warn!("Continuing setup despite ComfyUI-Impact-Pack failing: {}", e);
          }
      }
  
      // Install ComfyUI-Impact-Subpack
      info!("[SETUP_ORCHESTRATION] Attempting to clone ComfyUI-Impact-Subpack...");
      match custom_node_management::clone_comfyui_impact_subpack(&app_handle).await {
          Ok(_) => {
              info!("ComfyUI-Impact-Subpack cloned successfully or already exists and dependencies checked/installed.");
              emit_setup_progress(&app_handle, "installing_custom_nodes", "Impact Subpack Setup Complete", 80, Some("ComfyUI-Impact-Subpack processed.".to_string()), None); // Adjusted progress
          }
          Err(e) => {
              let err_msg = format!("Failed to setup ComfyUI-Impact-Subpack: {}", e);
              error!("{}", err_msg);
              emit_setup_progress(&app_handle, "installing_custom_nodes", "Impact Subpack Setup Failed", 75, Some(err_msg.clone()), Some(e.to_string())); // Adjusted progress
              warn!("Continuing setup despite ComfyUI-Impact-Subpack failing: {}", e);
          }
      }
  
      // Install ComfyUI_smZNodes
      info!("[SETUP_ORCHESTRATION] Attempting to clone ComfyUI_smZNodes...");
      match custom_node_management::clone_comfyui_smz_nodes(&app_handle).await {
          Ok(_) => {
              info!("ComfyUI_smZNodes cloned successfully or already exists.");
              emit_setup_progress(&app_handle, "installing_custom_nodes", "smZNodes Setup Complete", 85, Some("ComfyUI_smZNodes processed.".to_string()), None);
          }
          Err(e) => {
              let err_msg = format!("Failed to setup ComfyUI_smZNodes: {}", e);
              error!("{}", err_msg);
              emit_setup_progress(&app_handle, "installing_custom_nodes", "smZNodes Setup Failed", 80, Some(err_msg.clone()), Some(e.to_string()));
              warn!("Continuing setup despite ComfyUI_smZNodes failing: {}", e);
          }
      }
  
      // Install ComfyUI_InstantID
      info!("[SETUP_ORCHESTRATION] Attempting to clone ComfyUI_InstantID...");
      match custom_node_management::clone_comfyui_instantid(&app_handle).await {
          Ok(_) => {
              info!("ComfyUI_InstantID cloned successfully or already exists and dependencies checked/installed.");
              emit_setup_progress(&app_handle, "installing_custom_nodes", "InstantID Setup Complete", 90, Some("ComfyUI_InstantID processed.".to_string()), None);
          }
          Err(e) => {
              let err_msg = format!("Failed to setup ComfyUI_InstantID: {}", e);
              error!("{}", err_msg);
              emit_setup_progress(&app_handle, "installing_custom_nodes", "InstantID Setup Failed", 85, Some(err_msg.clone()), Some(e.to_string()));
              warn!("Continuing setup despite ComfyUI_InstantID failing: {}", e);
          }
      }
  
      // Install ComfyUI-IC-Light
      info!("[SETUP_ORCHESTRATION] Attempting to clone ComfyUI-IC-Light...");
      match custom_node_management::clone_comfyui_ic_light(&app_handle).await {
          Ok(_) => {
              info!("ComfyUI-IC-Light cloned successfully or already exists.");
              emit_setup_progress(&app_handle, "installing_custom_nodes", "IC-Light Setup Complete", 95, Some("ComfyUI-IC-Light processed.".to_string()), None);
          }
          Err(e) => {
              let err_msg = format!("Failed to setup ComfyUI-IC-Light: {}", e);
              error!("{}", err_msg);
              emit_setup_progress(&app_handle, "installing_custom_nodes", "IC-Light Setup Failed", 90, Some(err_msg.clone()), Some(e.to_string()));
              warn!("Continuing setup despite ComfyUI-IC-Light failing: {}", e);
          }
      }
      
      // Install rgthree-comfy (optional, standard clone)
      info!("[SETUP_ORCHESTRATION] Attempting to clone rgthree-comfy...");
      match custom_node_management::clone_rgthree_comfy_nodes(&app_handle).await {
          Ok(_) => {
              info!("rgthree-comfy cloned successfully or already exists.");
              // No specific progress step for this optional one, phase completion below will cover it.
          }
          Err(e) => {
              let err_msg = format!("Failed to setup rgthree-comfy: {}", e);
              error!("{}", err_msg);
              // Log warning, but don't emit a specific error progress step for this optional node.
              warn!("Continuing setup despite rgthree-comfy failing: {}", e);
          }
      }
    
      // Install ComfyUI-CLIPSeg
      info!("[SETUP_ORCHESTRATION] Attempting to clone ComfyUI-CLIPSeg...");
      match custom_node_management::clone_comfyui_clipseg(&app_handle).await {
          Ok(_) => {
              info!("ComfyUI-CLIPSeg cloned successfully or already exists.");
              // No specific progress step, covered by phase completion
          }
          Err(e) => {
              let err_msg = format!("Failed to setup ComfyUI-CLIPSeg: {}", e);
              error!("{}", err_msg);
              // Log warning, but don't emit a specific error progress step for this node.
              warn!("Continuing setup despite ComfyUI-CLIPSeg failing: {}", e);
          }
      }
  
      // Final progress for custom node phase
      emit_setup_progress(&app_handle, "installing_custom_nodes", "Custom Node Setup Finished", 100, Some("Custom node setup finished. Some optional nodes may have warnings if they failed.".to_string()), None);
      // End of Installing Custom Nodes Phase
   
       // Phase 3.5: Verification of Custom Nodes and Dependencies
      emit_setup_progress(&app_handle, "verifying_dependencies", "Verifying installations", 0, Some("Verifying custom node and Python package installations...".to_string()), None);
     
    let comfyui_dir_for_verify = get_comfyui_directory_path(&app_handle).map_err(|e| {
        let err_msg = format!("Failed to get ComfyUI directory for verification: {}", e);
        error!("[SETUP_ORCHESTRATION] {}", err_msg);
        emit_setup_progress(&app_handle, "error", "Verification Path Error (ComfyUI Dir)", 0, Some(err_msg.clone()), Some(e.clone()));
        err_msg
    })?;
 
    let venv_python_exe_for_verify = get_venv_python_executable_path(&app_handle).map_err(|e| {
        let err_msg = format!("Failed to get venv Python executable for verification: {}", e);
        error!("[SETUP_ORCHESTRATION] {}", err_msg);
        emit_setup_progress(&app_handle, "error", "Verification Path Error (Venv Python)", 0, Some(err_msg.clone()), Some(e.clone()));
        err_msg
    })?;
 
    // Verify IPAdapter Plus Directory (This is a redundant check if the previous phase passed, but kept for safety)
    match check_ipadapter_plus_directory_exists(&app_handle, &comfyui_dir_for_verify).await {
        Ok(true) => {
            info!("[SETUP_ORCHESTRATION] IPAdapter Plus directory verification successful.");
            emit_setup_progress(&app_handle, "verifying_dependencies", "IPAdapter+ directory found", 33, Some("ComfyUI_IPAdapter_plus directory verified.".to_string()), None);
        }
        Ok(false) => {
            let warn_msg = "ComfyUI_IPAdapter_plus directory not found during verification. IPAdapter features may be unavailable.".to_string();
            warn!("[SETUP_ORCHESTRATION] {}", warn_msg);
            // Emitting progress with a warning, not halting
            emit_setup_progress(&app_handle, "verifying_dependencies", "IPAdapter+ directory NOT found (Warning)", 33, Some(warn_msg), None);
        }
        Err(e) => {
            let err_msg = format!("Error checking IPAdapter Plus directory during verification: {}", e);
            error!("[SETUP_ORCHESTRATION] {}", err_msg);
            // Emitting progress with an error, but not halting for this specific check as per current plan for directory
            emit_setup_progress(&app_handle, "verifying_dependencies", "IPAdapter+ directory check error", 33, Some(err_msg), None);
        }
    }
 
    // Verify onnxruntime import
    match check_python_package_import(&app_handle, "onnxruntime", "script_check_onnx.py", &venv_python_exe_for_verify, &comfyui_dir_for_verify).await {
        Ok(_) => {
            info!("[SETUP_ORCHESTRATION] onnxruntime import verification successful.");
            emit_setup_progress(&app_handle, "verifying_dependencies", "onnxruntime import successful", 66, Some("onnxruntime imported successfully.".to_string()), None);
        }
        Err(e) => {
            let err_msg = format!("Failed to verify onnxruntime import: {}. Critical features may be unavailable.", e);
            error!("[SETUP_ORCHESTRATION] {}", err_msg);
            emit_setup_progress(&app_handle, "error", "ONNXRuntime Verification Failed", 0, Some(err_msg.clone()), Some(e));
            return Err(err_msg); // Halting setup
        }
    }
 
    // Verify insightface import
    match check_python_package_import(&app_handle, "insightface", "script_check_insightface.py", &venv_python_exe_for_verify, &comfyui_dir_for_verify).await {
        Ok(_) => {
            info!("[SETUP_ORCHESTRATION] insightface import verification successful.");
            emit_setup_progress(&app_handle, "verifying_dependencies", "insightface import successful", 100, Some("insightface imported successfully. Verification complete.".to_string()), None);
        }
        Err(e) => {
            let err_msg = format!("Failed to verify insightface import: {}. Critical features may be unavailable.", e);
            error!("[SETUP_ORCHESTRATION] {}", err_msg);
            emit_setup_progress(&app_handle, "error", "InsightFace Verification Failed", 0, Some(err_msg.clone()), Some(e));
            return Err(err_msg); // Halting setup
        }
    }
    // End of Verification Phase
 
    // Phase 4: Downloading Models
    emit_setup_progress(&app_handle, "downloading_models", "Downloading AI models", 0, Some("Starting download of core AI models...".to_string()), None);
 
    // Determine ComfyUI models base path
    let comfyui_dir_for_models = get_comfyui_directory_path(&app_handle)?;
    let comfyui_models_base_path = comfyui_dir_for_models.join("models");
    info!("[SETUP_ORCHESTRATION] Determined ComfyUI models base path: {}", comfyui_models_base_path.display());
 
    if !comfyui_models_base_path.exists() {
        fs::create_dir_all(&comfyui_models_base_path).map_err(|e| {
            format!("Failed to create ComfyUI models base directory at {}: {}", comfyui_models_base_path.display(), e)
        })?;
        info!("[SETUP_ORCHESTRATION] Created ComfyUI models base directory: {}", comfyui_models_base_path.display());
    }
    
    let core_models = get_core_models_list();
    if core_models.is_empty() {
        info!("[SETUP_ORCHESTRATION] No core models configured for download.");
        emit_setup_progress(&app_handle, "downloading_models", "No models to download", 100, Some("No core AI models configured for download.".to_string()), None);
    } else {
        // Check if core models already exist and are verified
        match super::verification::check_core_models_exist(&app_handle).await {
            Ok(true) => {
                info!("[SETUP_ORCHESTRATION] Core models already verified. Skipping download.");
                emit_setup_progress(&app_handle, "downloading_models", "Core models already exist", 100, Some("Core AI models are already installed.".to_string()), None);
            }
            Ok(false) | Err(_) => { // Proceed with download if not found or error during check
                info!("[SETUP_ORCHESTRATION] Core models verification failed or models not found. Proceeding with download.");
                // Progress for this phase is now emitted by download_and_place_models
                match download_and_place_models(app_handle.clone(), &core_models, &comfyui_models_base_path).await {
                    Ok(_) => {
                        info!("All core models processed successfully.");
                        // Final 100% progress for this phase is emitted by download_and_place_models
                    }
                    Err(e) => {
                        let err_msg = format!("Failed to download one or more core models: {}", e);
                        error!("{}", err_msg);
                        // The `download_and_place_models` function emits overall progress,
                        // but we also need to signify the phase ended in error.
                        emit_setup_progress(&app_handle, "error", "Model Download Failed", 0, Some(err_msg.clone()), Some(e.to_string()));
                        return Err(err_msg);
                    }
                }
            }
        }
    }

    // Phase 5: Finalizing (Starting ComfyUI Sidecar and Health Check)
    if !comfyui_was_already_running_and_assumed_healthy {
        info!("[SETUP_ORCHESTRATION] ComfyUI was not already running or assumed healthy. Starting ComfyUI services...");
        emit_setup_progress(&app_handle, "finalizing", "Starting ComfyUI services", 0, Some("Launching and verifying ComfyUI backend...".to_string()), None);
        match spawn_and_health_check_comfyui(&app_handle).await { // Changed call to use direct import
            Ok(_) => {
                info!("ComfyUI services started and healthy.");
                emit_setup_progress(&app_handle, "finalizing", "ComfyUI services ready", 100, Some("ComfyUI backend is running and responsive.".to_string()), None);
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
        emit_setup_progress(&app_handle, "finalizing", "ComfyUI services already running", 100, Some("ComfyUI backend was already running.".to_string()), None);
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

    emit_setup_progress(&app_handle, "complete", "Setup complete", 100, Some("Metamorphosis is ready to launch!".to_string()), None);
    info!("Full application setup orchestration completed successfully.");
    Ok(())
}


/// Retry the application setup process
#[tauri::command]
pub async fn retry_application_setup(app_handle: AppHandle<Wry>) -> Result<(), String> {
    start_application_setup(app_handle).await
}