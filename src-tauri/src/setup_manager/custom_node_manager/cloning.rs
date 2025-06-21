// metamorphosis-app/src-tauri/src/setup_manager/custom_node_manager/cloning.rs

use std::fs as std_fs;
use std::path::Path;
use std::pin::Pin;
use std::future::Future;
use std::env;
use tokio::fs;
use tauri::{AppHandle, Wry};
use uuid::Uuid;
use log::{info, error, warn};
use tauri_plugin_shell::ShellExt;

use crate::setup_manager::event_utils::{
    emit_custom_node_clone_start,
    emit_custom_node_clone_success,
    emit_custom_node_already_exists,
    emit_custom_node_clone_failed,
};
use crate::setup_manager::python_utils::get_comfyui_directory_path;
use crate::process_manager::ProcessManager;

/// Generic function to clone a custom node repository and install its dependencies.
pub async fn clone_repository_to_custom_nodes(
    app_handle: &AppHandle<Wry>,
    node_name: &str,
    repo_url: &str,
    install_dependencies_fn: Option<for<'a> fn(&'a AppHandle<Wry>, &str, &Path) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>>,
) -> Result<(), String> {
    info!("[CUSTOM_NODE_SETUP] Attempting to clone {}...", node_name);
    emit_custom_node_clone_start(app_handle, node_name);

    let comfyui_base_path = get_comfyui_directory_path(app_handle)?;
    let custom_nodes_dir = comfyui_base_path.join("custom_nodes");
    let target_dir = custom_nodes_dir.join(node_name);

    if !custom_nodes_dir.exists() {
        std_fs::create_dir_all(&custom_nodes_dir).map_err(|e| {
            let err_msg = format!("Failed to create custom_nodes directory at {}: {}", custom_nodes_dir.display(), e);
            error!("[CUSTOM_NODE_SETUP] {}", err_msg);
            emit_custom_node_clone_failed(app_handle, node_name, &err_msg);
            err_msg
        })?;
        info!("[CUSTOM_NODE_SETUP] Created custom_nodes directory: {}", custom_nodes_dir.display());
    }

    if target_dir.exists() {
        let is_empty = std_fs::read_dir(&target_dir).map_err(|e| e.to_string())?.next().is_none();
        if is_empty {
            warn!("[CUSTOM_NODE_SETUP] Target directory {} exists but is empty. Deleting and re-cloning.", target_dir.display());
            std_fs::remove_dir_all(&target_dir).map_err(|e| e.to_string())?;
        } else {
            info!("[CUSTOM_NODE_SETUP] Target directory {} for {} already exists and is not empty. Skipping clone.", target_dir.display(), node_name);
            emit_custom_node_already_exists(app_handle, node_name);
            if let Some(install_fn) = install_dependencies_fn {
                return install_fn(app_handle, node_name, &target_dir).await;
            }
            return Ok(());
        }
    }

    info!("[CUSTOM_NODE_SETUP] Cloning {} into {}", repo_url, target_dir.display());

    let git_target_path_arg_string = if cfg!(windows) {
        let path_str_cow = target_dir.to_string_lossy();
        if path_str_cow.starts_with("\\\\?\\") {
            path_str_cow.trim_start_matches("\\\\?\\").to_string()
        } else {
            path_str_cow.into_owned()
        }
    } else {
        target_dir.to_string_lossy().into_owned()
    };

    let command = app_handle.shell().command("git")
        .args(&["clone", repo_url, &git_target_path_arg_string]);

    let result = ProcessManager::spawn_and_wait_for_process(app_handle, command, &format!("git_clone_{}", node_name)).await
        .map_err(|e| {
            let err_msg = if e.contains("No such file or directory") { // A bit fragile, but Command::spawn error is not specific enough
                "Git command not found. Please ensure Git is installed and in your system's PATH.".to_string()
            } else {
                format!("Failed to execute git clone command for {}: {}", node_name, e)
            };
            error!("[CUSTOM_NODE_SETUP] {}", err_msg);
            emit_custom_node_clone_failed(app_handle, node_name, &err_msg);
            err_msg
        })?;


    let success = result.exit_code.map_or(false, |c| c == 0) && result.signal.is_none();

    if success {
        info!("[CUSTOM_NODE_SETUP] Successfully cloned {}.", node_name);
        emit_custom_node_clone_success(app_handle, node_name);
        if let Some(install_fn) = install_dependencies_fn {
            return install_fn(app_handle, node_name, &target_dir).await;
        }
        Ok(())
    } else {
        let stderr_output = result.stderr.join("\n");
        let stdout_output = result.stdout.join("\n");
        let err_msg = format!(
            "Failed to clone {}. Git command exited with code {:?}. Stderr: {}. Stdout: {}",
            node_name, result.exit_code, stderr_output.trim(), stdout_output.trim()
        );
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, node_name, &err_msg);
        Err(err_msg)
    }
}

/// Generic function to clone a custom node repository to a temporary directory.
/// This function is no longer used for CLIPSeg, but kept for potential future use.
pub async fn clone_repository_to_temp(
    app_handle: &AppHandle<Wry>,
    node_name: &str,
    repo_url: &str,
) -> Result<std::path::PathBuf, String> {
    info!("[CUSTOM_NODE_SETUP] Attempting to clone {} to a temporary directory...", node_name);

    // Create a unique temporary directory for cloning
    let temp_clone_dir_name = format!("{}_temp_{}", node_name, Uuid::new_v4());
    let temp_clone_path = env::temp_dir().join("metamorphosis_clones").join(&temp_clone_dir_name);
    
    // Ensure the parent directory for temp clones exists
    if let Some(parent) = temp_clone_path.parent() {
        if !parent.exists() {
            std_fs::create_dir_all(parent).map_err(|e| {
                format!("Failed to create parent temp clone directory at {}: {}", parent.display(), e)
            })?;
        }
    }

    info!("[CUSTOM_NODE_SETUP] Cloning {} to temporary directory: {}", repo_url, temp_clone_path.display());

    let git_temp_clone_path_arg_string = if cfg!(windows) {
        let path_str_cow = temp_clone_path.to_string_lossy();
        if path_str_cow.starts_with("\\\\?\\") {
            path_str_cow.trim_start_matches("\\\\?\\").to_string()
        } else {
            path_str_cow.into_owned()
        }
    } else {
        temp_clone_path.to_string_lossy().into_owned()
    };

    let command = app_handle.shell().command("git")
        .args(&["clone", repo_url, &git_temp_clone_path_arg_string]);

    let result = ProcessManager::spawn_and_wait_for_process(app_handle, command, &format!("git_clone_temp_{}", node_name)).await
        .map_err(|e| {
            let err_msg = if e.contains("No such file or directory") {
                "Git command not found. Please ensure Git is installed and in your system's PATH.".to_string()
            } else {
                format!("Failed to execute git clone command for {} (temp): {}", node_name, e)
            };
            error!("[CUSTOM_NODE_SETUP] {}", err_msg);
            emit_custom_node_clone_failed(app_handle, node_name, &err_msg);
            err_msg
        })?;

    let success = result.exit_code.map_or(false, |c| c == 0) && result.signal.is_none();

    if !success {
        let stderr_output = result.stderr.join("\n");
        let stdout_output = result.stdout.join("\n");
        let err_msg = format!(
            "Failed to clone {} (temp). Git command exited with code {:?}. Stderr: {}. Stdout: {}",
            node_name, result.exit_code, stderr_output.trim(), stdout_output.trim()
        );
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, node_name, &err_msg);
        if temp_clone_path.exists() {
            if let Err(e_rm) = fs::remove_dir_all(&temp_clone_path).await {
                warn!("[CUSTOM_NODE_SETUP] Failed to clean up temporary directory {} after failed clone: {}", temp_clone_path.display(), e_rm);
            }
        }
        return Err(err_msg);
    }

    info!("[CUSTOM_NODE_SETUP] Successfully cloned {} to temporary directory {}", node_name, temp_clone_path.display());
    Ok(temp_clone_path)
}

// Specific cloning functions for each custom node
use super::node_definitions::{
    IMPACT_PACK_NODE_NAME, IMPACT_PACK_REPO_URL,
    IMPACT_SUBPACK_NODE_NAME, IMPACT_SUBPACK_REPO_URL,
    SMZ_NODES_NODE_NAME, SMZ_NODES_REPO_URL,
    CONTROLNET_AUX_NODE_NAME, CONTROLNET_AUX_REPO_URL,
    CLIPSEG_NODE_NAME, CLIPSEG_REPO_URL,
    RMBG_NODE_NAME, RMBG_REPO_URL,
};
use super::installation::install_custom_node_dependencies;


pub async fn clone_comfyui_impact_pack(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    clone_repository_to_custom_nodes(
        app_handle,
        IMPACT_PACK_NODE_NAME,
        IMPACT_PACK_REPO_URL,
        Some(|h, n, p| Box::pin(install_custom_node_dependencies(h, n.to_string(), p.to_path_buf()))),
    ).await
}

pub async fn clone_comfyui_impact_subpack(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    clone_repository_to_custom_nodes(
        app_handle,
        IMPACT_SUBPACK_NODE_NAME,
        IMPACT_SUBPACK_REPO_URL,
        Some(|h, n, p| Box::pin(install_custom_node_dependencies(h, n.to_string(), p.to_path_buf()))),
    ).await
}

pub async fn clone_comfyui_smz_nodes(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    clone_repository_to_custom_nodes(
        app_handle,
        SMZ_NODES_NODE_NAME,
        SMZ_NODES_REPO_URL,
        Some(|h, n, p| Box::pin(install_custom_node_dependencies(h, n.to_string(), p.to_path_buf()))),
    ).await
}




pub async fn clone_comfyui_controlnet_aux(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    clone_repository_to_custom_nodes(
        app_handle,
        CONTROLNET_AUX_NODE_NAME,
        CONTROLNET_AUX_REPO_URL,
        Some(|h, n, p| Box::pin(install_custom_node_dependencies(h, n.to_string(), p.to_path_buf()))),
    ).await
}


pub async fn clone_comfyui_clipseg(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    info!("[CUSTOM_NODE_SETUP] Attempting to clone ComfyUI-CLIPSeg and move clipseg.py...");
    emit_custom_node_clone_start(app_handle, CLIPSEG_NODE_NAME);

    let comfyui_base_path = get_comfyui_directory_path(app_handle)?;
    let custom_nodes_dir = comfyui_base_path.join("custom_nodes");
    let clipseg_final_target_path = custom_nodes_dir.join("clipseg.py");
    let clipseg_repo_target_dir = custom_nodes_dir.join(CLIPSEG_NODE_NAME); // e.g., .../custom_nodes/ComfyUI-CLIPSeg

    // 1. Check if the final clipseg.py already exists
    if clipseg_final_target_path.exists() {
        info!("[CUSTOM_NODE_SETUP] clipseg.py already exists in custom_nodes. Skipping clone and move for {}.", CLIPSEG_NODE_NAME);
        emit_custom_node_already_exists(app_handle, CLIPSEG_NODE_NAME);
        return Ok(());
    }

    // 2. Clone the repository directly into custom_nodes/ComfyUI-CLIPSeg
    if !clipseg_repo_target_dir.exists() {
        info!("[CUSTOM_NODE_SETUP] Cloning {} into {}", CLIPSEG_REPO_URL, clipseg_repo_target_dir.display());

        let git_target_path_arg_string = if cfg!(windows) {
            let path_str_cow = clipseg_repo_target_dir.to_string_lossy();
            if path_str_cow.starts_with("\\\\?\\") {
                path_str_cow.trim_start_matches("\\\\?\\").to_string()
            } else {
                path_str_cow.into_owned()
            }
        } else {
            clipseg_repo_target_dir.to_string_lossy().into_owned()
        };

        let command = app_handle.shell().command("git")
            .args(&["clone", CLIPSEG_REPO_URL, &git_target_path_arg_string]);

        let result = ProcessManager::spawn_and_wait_for_process(app_handle, command, &format!("git_clone_{}", CLIPSEG_NODE_NAME)).await
            .map_err(|e| {
                let err_msg = if e.contains("No such file or directory") {
                    "Git command not found. Please ensure Git is installed and in your system's PATH.".to_string()
                } else {
                    format!("Failed to execute git clone command for {}: {}", CLIPSEG_NODE_NAME, e)
                };
                error!("[CUSTOM_NODE_SETUP] {}", err_msg);
                emit_custom_node_clone_failed(app_handle, CLIPSEG_NODE_NAME, &err_msg);
                err_msg
            })?;

        let success = result.exit_code.map_or(false, |c| c == 0) && result.signal.is_none();

        if !success {
            let stderr_output = result.stderr.join("\n");
            let stdout_output = result.stdout.join("\n");
            let err_msg = format!(
                "Failed to clone {}. Git command exited with code {:?}. Stderr: {}. Stdout: {}",
                CLIPSEG_NODE_NAME, result.exit_code, stderr_output.trim(), stdout_output.trim()
            );
            error!("[CUSTOM_NODE_SETUP] {}", err_msg);
            emit_custom_node_clone_failed(app_handle, CLIPSEG_NODE_NAME, &err_msg);
            if clipseg_repo_target_dir.exists() {
                if let Err(e_rm) = fs::remove_dir_all(&clipseg_repo_target_dir).await {
                    warn!("[CUSTOM_NODE_SETUP] Failed to clean up cloned directory {} after failed clone: {}", clipseg_repo_target_dir.display(), e_rm);
                }
            }
            return Err(err_msg);
        }
        info!("[CUSTOM_NODE_SETUP] Successfully cloned {} into {}", CLIPSEG_NODE_NAME, clipseg_repo_target_dir.display());
    } else {
        info!("[CUSTOM_NODE_SETUP] Target directory {} for {} already exists. Skipping clone.", clipseg_repo_target_dir.display(), CLIPSEG_NODE_NAME);
    }

    // 3. Copy clipseg.py from the cloned repository to custom_nodes
    let source_file_path = clipseg_repo_target_dir.join("custom_nodes").join("clipseg.py");

    if !source_file_path.exists() {
        let err_msg = format!("Expected clipseg.py not found in cloned repository at {}. Cannot copy file.", source_file_path.display());
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, CLIPSEG_NODE_NAME, &err_msg);
        return Err(err_msg);
    }

    info!("[CUSTOM_NODE_SETUP] Attempting to copy {} to {}", source_file_path.display(), clipseg_final_target_path.display());
    match fs::copy(&source_file_path, &clipseg_final_target_path).await {
        Ok(_) => {
            info!("[CUSTOM_NODE_SETUP] Successfully copied clipseg.py for {}.", CLIPSEG_NODE_NAME);
        },
        Err(e) => {
            let err_msg = format!("Failed to copy clipseg.py from {} to {}: {}", source_file_path.display(), clipseg_final_target_path.display(), e);
            error!("[CUSTOM_NODE_SETUP] {}", err_msg);
            emit_custom_node_clone_failed(app_handle, CLIPSEG_NODE_NAME, &err_msg);
            return Err(err_msg);
        }
    }

    emit_custom_node_clone_success(app_handle, CLIPSEG_NODE_NAME);
    Ok(())
}

pub async fn clone_comfyui_rmbg(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    clone_repository_to_custom_nodes(
        app_handle,
        RMBG_NODE_NAME,
        RMBG_REPO_URL,
        Some(|h, n, p| Box::pin(install_custom_node_dependencies(h, n.to_string(), p.to_path_buf()))),
    ).await
}

