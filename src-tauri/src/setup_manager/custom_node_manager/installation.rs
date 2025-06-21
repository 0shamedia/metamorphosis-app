// metamorphosis-app/src-tauri/src/setup_manager/custom_node_manager/installation.rs

use tauri::{AppHandle, Wry};
use log::{info, warn};

use crate::setup_manager::dependency_manager::command_runner::run_command_for_setup_progress;
use crate::setup_manager::python_utils::get_conda_executable_path;


/// Installs custom node dependencies using `conda run`.
pub async fn install_custom_node_dependencies(app_handle: &AppHandle<Wry>, node_name: String, pack_dir: std::path::PathBuf) -> Result<(), String> {
    let conda_executable = get_conda_executable_path(app_handle).await?;
    let env_name = "comfyui_env";
    info!("[CUSTOM_NODE_DEPENDENCY_INSTALL] Installing dependencies for {} using conda run...", node_name);

    let requirements_path = pack_dir.join("requirements.txt");
    if requirements_path.exists() {
        info!("[CUSTOM_NODE_DEPENDENCY_INSTALL] Found requirements.txt for {}. Installing...", node_name);
        
        let args: Vec<String> = vec![
            "run".to_string(),
            "-n".to_string(),
            env_name.to_string(),
            "python".to_string(),
            "-m".to_string(),
            "pip".to_string(),
            "install".to_string(),
            "-r".to_string(),
            requirements_path.to_str().unwrap().to_string(),
        ];
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let phase = "installing_custom_nodes"; // Consistent phase name
        let current_step_base = &format!("Installing dependencies for {}", node_name);
        let progress_current_phase = 0; // This is a sub-step, so it starts at 0
        let progress_weight_of_this_command = 100; // It's the only command in this function
        let initial_message = format!("Installing dependencies for {} from requirements.txt", node_name);
        let success_message = format!("Dependencies for {} installed.", node_name);

        run_command_for_setup_progress(
            app_handle,
            phase,
            current_step_base,
            progress_current_phase,
            progress_weight_of_this_command,
            &conda_executable,
            &args_refs,
            &pack_dir,
            &initial_message,
            &success_message,
        ).await.map_err(|e| e.to_string())?;
        
        info!("[CUSTOM_NODE_DEPENDENCY_INSTALL] Dependencies for {} installed successfully.", node_name);
    } else {
        warn!("[CUSTOM_NODE_DEPENDENCY_INSTALL] No requirements.txt found for {}. Skipping dependency installation.", node_name);
    }
    Ok(())
}