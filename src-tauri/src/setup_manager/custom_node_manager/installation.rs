// metamorphosis-app/src-tauri/src/setup_manager/custom_node_manager/installation.rs

use tauri::{AppHandle, Wry};
use log::{info};

use crate::setup_manager::dependency_manager::command_runner::run_command_for_setup_progress;
use crate::setup_manager::python_utils::{get_venv_python_executable_path};

use super::node_definitions::{
    ONNXRUNTIME_PACKAGE,
};

/// Installs custom node dependencies.
pub async fn install_custom_node_dependencies(app_handle: &AppHandle<Wry>, node_name: String, pack_dir: std::path::PathBuf) -> Result<(), String> {
    let python_executable = get_venv_python_executable_path(app_handle)?;
    info!("[CUSTOM_NODE_DEPENDENCY_INSTALL] Installing dependencies for {}...", node_name);

    let requirements_path = pack_dir.join("requirements.txt");
    if requirements_path.exists() {
        info!("[CUSTOM_NODE_DEPENDENCY_INSTALL] Found requirements.txt for {}. Installing...", node_name);
        let phase = "Custom Node Installation";
        let current_step_base = "Installing dependencies for custom node";
        let progress_current_phase = 0;
        let progress_weight_of_this_command = 100; // This command is the whole step for custom node deps
        let initial_message = format!("Installing dependencies for {} from requirements.txt", node_name);
        let error_message_prefix = format!("Failed to install dependencies for {}", node_name);

        run_command_for_setup_progress(
            &app_handle,
            phase,
            current_step_base,
            progress_current_phase,
            progress_weight_of_this_command,
            &python_executable,
            &["-m", "pip", "install", "-r", requirements_path.to_str().unwrap()],
            &pack_dir, // Pass &pack_dir for current_dir
            &initial_message,
            &error_message_prefix,
        ).await.map_err(|e| e.to_string())?; // Convert error to String
        info!("[CUSTOM_NODE_DEPENDENCY_INSTALL] Dependencies for {} installed successfully.", node_name);
    } else {
        info!("[CUSTOM_NODE_DEPENDENCY_INSTALL] No requirements.txt found for {}. Skipping dependency installation.", node_name);
    }
    Ok(())
}