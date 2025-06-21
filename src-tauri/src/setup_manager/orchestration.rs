// metamorphosis-app/src-tauri/src/setup_manager/orchestration.rs
use tauri::{AppHandle, Manager, Wry}; // Emitter might not be directly used here but good to have if needed
use log::{error, info, warn}; // Added warn
use std::fs;
use std::path::PathBuf; // Added this import
use tauri::Emitter; // Added Emitter

/// Determines the application's root directory based on whether it's a debug or release build.
/// This logic is adapted from `sidecar_manager/process_handler.rs`.
pub(crate) fn get_app_root_path() -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        // In debug mode, CARGO_MANIFEST_DIR points to src-tauri.
        // We want the parent of src-tauri, which is the app root.
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(|p| p.to_path_buf()) // Convert &Path to PathBuf
            .ok_or_else(|| "Failed to get parent of CARGO_MANIFEST_DIR".to_string())
            .map_err(|e| format!("Error getting app root path in debug: {}", e))
    } else {
        // In release mode, the executable is typically in `app_root/target/release/app_name.exe`
        // or `app_root/app_name.app/Contents/MacOS/app_name` on macOS.
        // We need to go up multiple levels to reach the app root.
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;
        let exe_dir = exe_path.parent()
            .ok_or_else(|| format!("Failed to get parent directory of executable: {}", exe_path.display()))?
            .to_path_buf();

        // For Windows/Linux, exe_dir is typically `target/release` or `target/debug`
        // For macOS, exe_dir is typically `Contents/MacOS` inside the .app bundle
        // We need to go up until we find the app root.
        // This assumes the app root is the grandparent of the target/release or target/debug folder.
        // Or for macOS, the parent of the .app bundle.
        let mut current_path = exe_dir.clone();
        // Traverse up until we find a directory that doesn't look like a build artifact
        // or we reach a reasonable limit.
        // A more robust solution might involve checking for a specific marker file in the app root.
        // For now, we'll assume the structure is consistent.
        
        // On Windows/Linux, from `target/release` or `target/debug`, parent is `target`, then parent is `app_root`.
        // On macOS, from `Contents/MacOS`, parent is `Contents`, then parent is `app_name.app`, then parent is `app_root`.
        // So, for release, we need to go up 2 levels for Windows/Linux, and 3 levels for macOS.
        
        #[cfg(target_os = "windows")]
        let app_root = current_path.parent() // target/release -> target
                                   .and_then(|p| p.parent()) // target -> src-tauri
                                   .and_then(|p| p.parent()) // src-tauri -> app_root
                                   .ok_or_else(|| format!("Failed to get app root from executable path: {}", exe_dir.display()))?;
        #[cfg(target_os = "linux")]
        let app_root = current_path.parent() // target/release -> target
                                   .and_then(|p| p.parent()) // target -> src-tauri
                                   .and_then(|p| p.parent()) // src-tauri -> app_root
                                   .ok_or_else(|| format!("Failed to get app root from executable path: {}", exe_dir.display()))?;
        #[cfg(target_os = "macos")]
        let app_root = current_path.parent() // Contents/MacOS -> Contents
                                   .and_then(|p| p.parent()) // Contents -> app_name.app
                                   .and_then(|p| p.parent()) // app_name.app -> app_root
                                   .ok_or_else(|| format!("Failed to get app root from executable path: {}", exe_dir.display()))?;

        Ok(app_root.to_path_buf())
    }
}

// Miniconda Constants
const MINICONDA_INSTALLER_WIN_FILENAME: &str = "Miniconda3-latest-Windows-x86_64.exe";
const MINICONDA_INSTALLER_LINUX_FILENAME: &str = "Miniconda3-latest-Linux-x86_64.sh";
const MINICONDA_INSTALLER_MACOS_FILENAME: &str = "Miniconda3-latest-MacOSX-x86_64.pkg";
const MINICONDA_INSTALLER_MACOS_ARM64_FILENAME: &str = "Miniconda3-latest-MacOSX-arm64.pkg";
const INSTALLERS_SUBDIR: &str = "resources/installers";
pub(crate) const MINICONDA_INSTALL_DIR_NAME: &str = "miniconda3";
const MINICONDA_INSTALLED_MARKER: &str = ".miniconda_installed.marker";

use super::event_utils::emit_setup_progress;
use super::types::SetupStatusEvent;
// Updated verification imports
use super::verification::{
    check_initialization_status, run_quick_verification, check_python_package_import
    // get_comfyui_vendor_paths, // This will be replaced by python_utils
};
// Import new python_utils functions
use crate::setup_manager::python_utils::{
    get_comfyui_directory_path, get_conda_env_python_executable_path,
    // get_bundled_python_executable_path, // Not directly used here, but available
    // get_script_path, // Not directly used here
    // get_vendor_path, // No longer directly used here, comfyui_directory_path is used
};
use crate::setup_manager::{get_core_models_list, download_and_place_models}; // Uncommented and added functions
use crate::setup_manager::custom_node_manager;
use crate::setup_manager::dependency_manager; // Changed from crate::dependency_management
 
// Note: comfyui_sidecar is kept as crate level for now.
// If it is also refactored into managers, these paths would change.
// use crate::comfyui_sidecar; // Removed direct import
use crate::sidecar_manager::spawn_and_health_check_comfyui; // Imported function directly
use crate::process_manager::ProcessManager;


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

    let process_manager = app_handle.state::<ProcessManager>();
    let mut comfyui_was_already_running_and_assumed_healthy = false;

    if process_manager.is_process_running("comfyui_sidecar") {
        info!("[SETUP_ORCHESTRATION] ComfyUI process is already active. Assuming it's from quick verification and healthy. Skipping stop/restart.");
        comfyui_was_already_running_and_assumed_healthy = true;
    } else {
        info!("[SETUP_ORCHESTRATION] No active ComfyUI process found. Proceeding with stop (no-op) and start.");
        // Attempt to stop any existing ComfyUI sidecar process first
        info!("[SETUP_ORCHESTRATION] Attempting to stop any pre-existing ComfyUI sidecar process...");
        process_manager.stop_process("comfyui_sidecar");
        info!("[SETUP_ORCHESTRATION] Pre-existing ComfyUI sidecar stop attempt complete.");
    }

    // Phase: Miniconda Setup (0-20%)
    emit_setup_progress(&app_handle, "installing_miniconda", "Checking Miniconda installation", 0, Some("Verifying Miniconda environment...".to_string()), None);

    let app_root_path = get_app_root_path()?;
    let miniconda_install_path = app_root_path.join(MINICONDA_INSTALL_DIR_NAME);
    let miniconda_marker_path = app_root_path.join(MINICONDA_INSTALLED_MARKER);

    let conda_exe_path = if cfg!(windows) {
        miniconda_install_path.join("Scripts").join("conda.exe")
    } else {
        miniconda_install_path.join("bin").join("conda")
    };

    if miniconda_marker_path.exists() {
        if conda_exe_path.exists() {
            info!("[SETUP_ORCHESTRATION] Miniconda marker found at {} and conda executable exists. Skipping installation.", miniconda_marker_path.display());
            emit_setup_progress(&app_handle, "installing_miniconda", "Miniconda already installed", 20, Some("Miniconda is already set up.".to_string()), None);
        } else {
            warn!("[SETUP_ORCHESTRATION] Miniconda marker found at {} but conda executable does not exist at {}. Removing stale marker and forcing re-installation.", miniconda_marker_path.display(), conda_exe_path.display());
            fs::remove_file(&miniconda_marker_path).map_err(|e| format!("Failed to remove stale Miniconda marker: {}", e))?;
            // Fall through to the installation logic below
            info!("[SETUP_ORCHESTRATION] Miniconda marker NOT found. Proceeding with installation.");
            emit_setup_progress(&app_handle, "installing_miniconda", "Locating Miniconda installer", 5, Some("Searching for bundled Miniconda installer...".to_string()), None);
        }
    } else {
        info!("[SETUP_ORCHESTRATION] Miniconda marker NOT found. Proceeding with installation.");
        emit_setup_progress(&app_handle, "installing_miniconda", "Locating Miniconda installer", 5, Some("Searching for bundled Miniconda installer...".to_string()), None);
        info!("[SETUP_ORCHESTRATION] Miniconda marker NOT found. Proceeding with installation.");
        emit_setup_progress(&app_handle, "installing_miniconda", "Locating Miniconda installer", 5, Some("Searching for bundled Miniconda installer...".to_string()), None);

        let installer_filename = if cfg!(windows) {
            MINICONDA_INSTALLER_WIN_FILENAME
        } else if cfg!(target_os = "linux") {
            MINICONDA_INSTALLER_LINUX_FILENAME
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") { // Apple Silicon
                MINICONDA_INSTALLER_MACOS_ARM64_FILENAME
            } else { // Intel Mac
                MINICONDA_INSTALLER_MACOS_FILENAME
            }
        } else {
            let err_msg = "Unsupported operating system for Miniconda installation.".to_string();
            error!("{}", err_msg);
            emit_setup_progress(&app_handle, "error", "Miniconda Setup Failed", 0, Some(err_msg.clone()), Some(err_msg.clone()));
            return Err(err_msg);
        };

        let installers_dir = app_root_path.join(INSTALLERS_SUBDIR);
        let installer_path = installers_dir.join(installer_filename);

        if !installer_path.exists() {
            let err_msg = format!("Bundled Miniconda installer not found at: {}", installer_path.display());
            error!("{}", err_msg);
            emit_setup_progress(&app_handle, "error", "Miniconda Installer Missing", 0, Some(err_msg.clone()), Some(err_msg.clone()));
            return Err(err_msg);
        }

        emit_setup_progress(&app_handle, "installing_miniconda", "Installing Miniconda", 5, Some("Running Miniconda installer...".to_string()), None);

        let installer_path_str = installer_path.to_string_lossy().to_string();
        let install_path_arg = miniconda_install_path.to_string_lossy().to_string(); // Use backslashes for Windows installer compatibility

        let installer_path_str = installer_path.to_string_lossy().to_string();

        let install_command_result = if cfg!(windows) {
            crate::setup_manager::dependency_manager::command_runner::run_command_for_setup_progress(
                &app_handle,
                "installing_miniconda", // phase
                "Miniconda Installation", // current_step_base
                5, // progress_current_phase (start at 5%)
                15, // progress_weight_of_this_command (contribute 15% to reach 20%)
                &installer_path, // Directly execute the installer
                &[
                    "/S", // Silent install
                    "/InstallationType=JustMe", // Install for current user
                    &format!("/D={}", install_path_arg) // Destination path, must be last
                ],
                &installers_dir, // current_dir (where the installer is located)
                "Running Miniconda installer...", // initial_message
                "Failed to install Miniconda", // error_message_prefix
            ).await
        } else if cfg!(target_os = "linux") {
            crate::setup_manager::dependency_manager::command_runner::run_command_for_setup_progress(
                &app_handle,
                "installing_miniconda", // phase
                "Miniconda Installation", // current_step_base
                5, // progress_current_phase (start at 5%)
                15, // progress_weight_of_this_command (contribute 15% to reach 20%)
                &PathBuf::from("bash"), // command_path
                &[&installer_path_str, "-b", "-p", &install_path_arg], // args
                &installers_dir, // current_dir
                "Running Miniconda installer...", // initial_message
                "Failed to install Miniconda", // error_message_prefix
            ).await
        } else if cfg!(target_os = "macos") {
            warn!("[SETUP_ORCHESTRATION] macOS .pkg silent installation is complex. Attempting .sh style install.");
            crate::setup_manager::dependency_manager::command_runner::run_command_for_setup_progress(
                &app_handle,
                "installing_miniconda", // phase
                "Miniconda Installation", // current_step_base
                5, // progress_current_phase (start at 5%)
                15, // progress_weight_of_this_command (contribute 15% to reach 20%)
                &PathBuf::from("bash"), // command_path
                &[&installer_path_str, "-b", "-p", &install_path_arg], // args
                &installers_dir, // current_dir
                "Running Miniconda installer...", // initial_message
                "Failed to install Miniconda", // error_message_prefix
            ).await
        } else {
            Err("Unsupported OS for Miniconda installation command.".to_string())
        };

        match install_command_result {
            Ok(_) => {
                info!("[SETUP_ORCHESTRATION] Miniconda installed successfully.");
// Wait for conda.exe to appear, as the installer might exit before files are fully written
                let conda_exe_path = if cfg!(windows) {
                    miniconda_install_path.join("Scripts").join("conda.exe")
                } else {
                    miniconda_install_path.join("bin").join("conda")
                };

                crate::setup_manager::python_utils::wait_for_file_to_exist(
                    &app_handle,
                    &conda_exe_path,
                    60, // Timeout after 60 seconds
                    500, // Check every 500 milliseconds
                    "conda executable",
                ).await?;

                // Create marker file in the app root path
                if let Err(e) = fs::write(&miniconda_marker_path, "installed") {
                    error!("[SETUP_ORCHESTRATION] Failed to create Miniconda installed marker file at {}: {}", miniconda_marker_path.display(), e);
                    return Err(format!("Failed to create Miniconda installed marker file: {}", e));
                }
                emit_setup_progress(&app_handle, "installing_miniconda", "Miniconda installation complete", 20, Some("Miniconda installed successfully.".to_string()), None);
            },
            Err(e) => {
                let err_msg = format!("Failed to install Miniconda: {}", e);
                error!("{}", err_msg);
                emit_setup_progress(&app_handle, "error", "Miniconda Installation Failed", 0, Some(err_msg.clone()), Some(e.to_string()));
                return Err(err_msg);
            }
        }
    }

    // Phase 1: Checking (Initial system checks, disk space etc.) (20-30%)
    emit_setup_progress(&app_handle, "checking", "Running system checks", 20, Some("Checking system requirements and environment...".to_string()), None);
    
    let main_window = app_handle.get_webview_window("main").ok_or_else(|| {
        let msg = "Failed to get main window for initial checks".to_string();
        error!("{}", msg);
        msg
    })?;
    match check_initialization_status(main_window).await {
        Ok(_) => emit_setup_progress(&app_handle, "checking", "System checks complete", 30, Some("All system requirements met.".to_string()), None),
        Err(e) => {
            let err_msg = format!("Initial system checks failed: {}", e);
            error!("{}", err_msg);
            emit_setup_progress(&app_handle, "error", "System Check Failed", 0, Some(err_msg.clone()), Some(e));
            return Err(err_msg);
        }
    }
 
     // Phase 2 & 3: Python Environment & ComfyUI Dependencies (30-60%)
     emit_setup_progress(&app_handle, "python_setup", "Setting up Python environment", 30, Some("Initializing Python virtual environment and dependencies...".to_string()), None);
 
 
     // Check if Python environment is already set up and dependencies are installed
     match super::verification::check_python_environment_integrity(&app_handle).await {
         Ok(true) => {
             info!("[SETUP_ORCHESTRATION] Python environment and dependencies already verified. Skipping installation.");
             emit_setup_progress(&app_handle, "python_setup", "Python environment ready", 60, Some("Python environment and dependencies are already set up.".to_string()), None);
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
 
      // Phase: Installing Custom Nodes (60-80%)
     // This phase is added before model downloading, as custom nodes might define model locations or types.
     emit_setup_progress(&app_handle, "installing_custom_nodes", "Setting up custom nodes", 60, Some("Cloning and installing required custom nodes...".to_string()), None);
     
 
       // Install ComfyUI-Impact-Pack
       info!("[SETUP_ORCHESTRATION] Attempting to clone ComfyUI-Impact-Pack...");
       match custom_node_manager::clone_comfyui_impact_pack(&app_handle).await {
           Ok(_) => {
               info!("ComfyUI-Impact-Pack cloned successfully or already exists and dependencies checked/installed.");
               emit_setup_progress(&app_handle, "installing_custom_nodes", "Impact Pack Setup Complete", 68, Some("ComfyUI-Impact-Pack processed.".to_string()), None);
           }
           Err(e) => {
               let err_msg = format!("Failed to setup ComfyUI-Impact-Pack: {}", e);
               error!("{}", err_msg);
               emit_setup_progress(&app_handle, "installing_custom_nodes", "Impact Pack Setup Failed", 64, Some(err_msg.clone()), Some(e.to_string()));
               warn!("Continuing setup despite ComfyUI-Impact-Pack failing: {}", e);
           }
       }
   
       // Install ComfyUI-Impact-Subpack
       info!("[SETUP_ORCHESTRATION] Attempting to clone ComfyUI-Impact-Subpack...");
       match custom_node_manager::clone_comfyui_impact_subpack(&app_handle).await {
           Ok(_) => {
               info!("ComfyUI-Impact-Subpack cloned successfully or already exists and dependencies checked/installed.");
               emit_setup_progress(&app_handle, "installing_custom_nodes", "Impact Subpack Setup Complete", 70, Some("ComfyUI-Impact-Subpack processed.".to_string()), None);
           }
           Err(e) => {
               let err_msg = format!("Failed to setup ComfyUI-Impact-Subpack: {}", e);
               error!("{}", err_msg);
               emit_setup_progress(&app_handle, "installing_custom_nodes", "Impact Subpack Setup Failed", 68, Some(err_msg.clone()), Some(e.to_string()));
               warn!("Continuing setup despite ComfyUI-Impact-Subpack failing: {}", e);
           }
       }
   
       // Install ComfyUI_smZNodes
       info!("[SETUP_ORCHESTRATION] Attempting to clone ComfyUI_smZNodes...");
       match custom_node_manager::clone_comfyui_smz_nodes(&app_handle).await {
           Ok(_) => {
               info!("ComfyUI_smZNodes cloned successfully or already exists.");
               emit_setup_progress(&app_handle, "installing_custom_nodes", "smZNodes Setup Complete", 72, Some("ComfyUI_smZNodes processed.".to_string()), None);
           }
           Err(e) => {
               let err_msg = format!("Failed to setup ComfyUI_smZNodes: {}", e);
               error!("{}", err_msg);
               emit_setup_progress(&app_handle, "installing_custom_nodes", "smZNodes Setup Failed", 70, Some(err_msg.clone()), Some(e.to_string()));
               warn!("Continuing setup despite ComfyUI_smZNodes failing: {}", e);
           }
       }
   
       // Install ComfyUI_ControlNet_Aux
       info!("[SETUP_ORCHESTRATION] Attempting to clone ComfyUI_ControlNet_Aux...");
       match custom_node_manager::clone_comfyui_controlnet_aux(&app_handle).await {
           Ok(_) => {
               info!("ComfyUI_ControlNet_Aux cloned successfully or already exists and dependencies checked/installed.");
               emit_setup_progress(&app_handle, "installing_custom_nodes", "ControlNet Aux Setup Complete", 74, Some("ComfyUI_ControlNet_Aux processed.".to_string()), None);
           }
           Err(e) => {
               let err_msg = format!("Failed to setup ComfyUI_ControlNet_Aux: {}", e);
               error!("{}", err_msg);
               emit_setup_progress(&app_handle, "installing_custom_nodes", "ControlNet Aux Setup Failed", 72, Some(err_msg.clone()), Some(e.to_string()));
               warn!("Continuing setup despite ComfyUI_ControlNet_Aux failing: {}", e);
           }
       }
   
       // Install ComfyUI-CLIPSeg
       info!("[SETUP_ORCHESTRATION] Attempting to clone ComfyUI-CLIPSeg...");
       match custom_node_manager::clone_comfyui_clipseg(&app_handle).await {
           Ok(_) => {
               info!("ComfyUI-CLIPSeg cloned and clipseg.py moved successfully or already exists.");
               emit_setup_progress(&app_handle, "installing_custom_nodes", "CLIPSeg Setup Complete", 76, Some("ComfyUI-CLIPSeg processed.".to_string()), None);
           }
           Err(e) => {
               let err_msg = format!("Failed to setup ComfyUI-CLIPSeg: {}", e);
               error!("{}", err_msg);
               emit_setup_progress(&app_handle, "installing_custom_nodes", "CLIPSeg Setup Failed", 74, Some(err_msg.clone()), Some(e.to_string()));
               warn!("Continuing setup despite ComfyUI-CLIPSeg failing: {}", e);
           }
       }
   
       // Install ComfyUI-RMBG
       info!("[SETUP_ORCHESTRATION] Attempting to clone ComfyUI-RMBG...");
       match custom_node_manager::clone_comfyui_rmbg(&app_handle).await {
           Ok(_) => {
               info!("ComfyUI-RMBG cloned successfully or already exists and dependencies checked/installed.");
               emit_setup_progress(&app_handle, "installing_custom_nodes", "RMBG Setup Complete", 78, Some("ComfyUI-RMBG processed.".to_string()), None);
           }
           Err(e) => {
               let err_msg = format!("Failed to setup ComfyUI-RMBG: {}", e);
               error!("{}", err_msg);
               emit_setup_progress(&app_handle, "installing_custom_nodes", "RMBG Setup Failed", 76, Some(err_msg.clone()), Some(e.to_string()));
               warn!("Continuing setup despite ComfyUI-RMBG failing: {}", e);
           }
       }
   
       // Final progress for custom node phase
       emit_setup_progress(&app_handle, "installing_custom_nodes", "Custom Node Setup Finished", 80, Some("Custom node setup finished. Some optional nodes may have warnings if they failed.".to_string()), None);
       // End of Installing Custom Nodes Phase
   
         // Phase 3.5: Verification of Custom Nodes and Dependencies (80-85%)
        emit_setup_progress(&app_handle, "verifying_dependencies", "Verifying installations", 80, Some("Verifying custom node and Python package installations...".to_string()), None);
     
        let comfyui_dir_for_verify = get_comfyui_directory_path(&app_handle).map_err(|e| {
            let err_msg = format!("Failed to get ComfyUI directory for verification: {}", e);
            error!("[SETUP_ORCHESTRATION] {}", err_msg);
            emit_setup_progress(&app_handle, "error", "Verification Path Error (ComfyUI Dir)", 0, Some(err_msg.clone()), Some(e.clone()));
            err_msg
        })?;
   
        let venv_python_exe_for_verify = get_conda_env_python_executable_path(&app_handle, "comfyui_env").await.map_err(|e| {
            let err_msg = format!("Failed to get venv Python executable for verification: {}", e);
            error!("[SETUP_ORCHESTRATION] {}", err_msg);
            emit_setup_progress(&app_handle, "error", "Verification Path Error (Venv Python)", 0, Some(err_msg.clone()), Some(e.clone()));
            err_msg
        })?;
   
        // Verify onnxruntime import
        match check_python_package_import(&app_handle, "onnxruntime", &venv_python_exe_for_verify, &comfyui_dir_for_verify).await {
            Ok(_) => {
                info!("[SETUP_ORCHESTRATION] onnxruntime import verification successful.");
                emit_setup_progress(&app_handle, "verifying_dependencies", "onnxruntime import successful", 85, Some("onnxruntime imported successfully.".to_string()), None);
            }
            Err(e) => {
                let err_msg = format!("Failed to verify onnxruntime import: {}. Critical features may be unavailable.", e);
                error!("[SETUP_ORCHESTRATION] {}", err_msg);
                emit_setup_progress(&app_handle, "error", "ONNXRuntime Verification Failed", 0, Some(err_msg.clone()), Some(e));
                return Err(err_msg); // Halting setup
            }
        }
   
        // End of Verification Phase
   
        // Phase 4: Downloading Models (85-95%)
        emit_setup_progress(&app_handle, "downloading_models", "Downloading AI models", 85, Some("Starting download of core AI models...".to_string()), None);
   
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
            emit_setup_progress(&app_handle, "downloading_models", "No models to download", 95, Some("No core AI models configured for download.".to_string()), None);
        } else {
            // Check if core models already exist and are verified
            match super::verification::check_core_models_exist(&app_handle).await {
                Ok(true) => {
                    info!("[SETUP_ORCHESTRATION] Core models already verified. Skipping download.");
                    emit_setup_progress(&app_handle, "downloading_models", "Core models already exist", 95, Some("Core AI models are already installed.".to_string()), None);
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
   
        // Phase 5: Finalizing (Starting ComfyUI Sidecar and Health Check) (95-100%)
        if !comfyui_was_already_running_and_assumed_healthy {
            info!("[SETUP_ORCHESTRATION] ComfyUI was not already running or assumed healthy. Starting ComfyUI services...");
            emit_setup_progress(&app_handle, "finalizing", "Starting ComfyUI services", 95, Some("Launching and verifying ComfyUI backend...".to_string()), None);
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