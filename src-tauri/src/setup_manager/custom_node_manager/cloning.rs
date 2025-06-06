// metamorphosis-app/src-tauri/src/setup_manager/custom_node_manager/cloning.rs

use std::fs as std_fs;
use std::path::Path;
use std::process::Stdio as ProcessStdio;
use std::pin::Pin;
use std::future::Future;
use std::env;
use tokio::fs;
use tokio::process::Command as TokioCommand;
use tauri::{AppHandle, Wry};
use log::{info, error, debug};

use crate::setup_manager::event_utils::{
    emit_custom_node_clone_start,
    emit_custom_node_clone_success,
    emit_custom_node_already_exists,
    emit_custom_node_clone_failed,
};
use crate::setup_manager::python_utils::get_comfyui_directory_path;

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
        info!("[CUSTOM_NODE_SETUP] Target directory {} for {} already exists. Skipping clone.", target_dir.display(), node_name);
        emit_custom_node_already_exists(app_handle, node_name);
        if let Some(install_fn) = install_dependencies_fn {
            return install_fn(app_handle, node_name, &target_dir).await;
        }
        return Ok(());
    }

    info!("[CUSTOM_NODE_SETUP] Cloning {} into {}", repo_url, target_dir.display());

    let mut command = TokioCommand::new("git");
    let git_target_path_arg_string;
    if cfg!(windows) {
        let path_str_cow = target_dir.to_string_lossy();
        if path_str_cow.starts_with("\\\\?\\") {
            git_target_path_arg_string = path_str_cow.trim_start_matches("\\\\?\\").to_string();
            debug!("[CUSTOM_NODE_SETUP] Using cleaned path for git clone (Windows): {}", git_target_path_arg_string);
        } else {
            git_target_path_arg_string = path_str_cow.into_owned();
        }
    } else {
        git_target_path_arg_string = target_dir.to_string_lossy().into_owned();
    }

    command.arg("clone").arg(repo_url).arg(git_target_path_arg_string)
        .stdout(ProcessStdio::piped())
        .stderr(ProcessStdio::piped());

    let child = command.spawn().map_err(|e| {
        let err_msg = if e.kind() == std::io::ErrorKind::NotFound {
            "Git command not found. Please ensure Git is installed and in your system's PATH.".to_string()
        } else {
            format!("Failed to execute git clone command for {}: {}", node_name, e)
        };
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, node_name, &err_msg);
        err_msg
    })?;

    let output = child.wait_with_output().await.map_err(|e| {
        let err_msg = format!("Failed to wait for git clone command for {}: {}", node_name, e);
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, node_name, &err_msg);
        err_msg
    })?;

    if output.status.success() {
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let filtered_stdout = stdout_str.lines().filter(|line| !line.contains("SKIPPING LINK")).collect::<Vec<&str>>().join("\n");
        info!("[CUSTOM_NODE_SETUP] Successfully cloned {}. Output: {}", node_name, filtered_stdout);
        emit_custom_node_clone_success(app_handle, node_name);
        if let Some(install_fn) = install_dependencies_fn {
            return install_fn(app_handle, node_name, &target_dir).await;
        }
        Ok(())
    } else {
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let filtered_stderr = stderr_str.lines().filter(|line| !line.contains("SKIPPING LINK")).collect::<Vec<&str>>().join("\n");
        let filtered_stdout = stdout_str.lines().filter(|line| !line.contains("SKIPPING LINK")).collect::<Vec<&str>>().join("\n");
        let err_msg = format!(
            "Failed to clone {}. Git command exited with error. Status: {}. Stderr: {}. Stdout: {}",
            node_name, output.status, filtered_stderr.trim(), filtered_stdout.trim()
        );
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, node_name, &err_msg);
        Err(err_msg)
    }
}

/// Generic function to clone a custom node repository to a temporary directory.
pub async fn clone_repository_to_temp(
    app_handle: &AppHandle<Wry>,
    node_name: &str,
    repo_url: &str,
) -> Result<std::path::PathBuf, String> {
    info!("[CUSTOM_NODE_SETUP] Attempting to clone {} to a temporary directory...", node_name);

    // Create a unique temporary directory for cloning
    let temp_clone_dir_name = format!("{}_temp_{}", node_name, uuid::Uuid::new_v4());
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

    let mut command = TokioCommand::new("git");
    let git_temp_clone_path_arg_string;
    if cfg!(windows) {
        let path_str_cow = temp_clone_path.to_string_lossy();
        if path_str_cow.starts_with("\\\\?\\") {
            git_temp_clone_path_arg_string = path_str_cow.trim_start_matches("\\\\?\\").to_string();
        } else {
            git_temp_clone_path_arg_string = path_str_cow.into_owned();
        }
    } else {
        git_temp_clone_path_arg_string = temp_clone_path.to_string_lossy().into_owned();
    }

    command.arg("clone").arg(repo_url).arg(&git_temp_clone_path_arg_string)
        .stdout(ProcessStdio::piped())
        .stderr(ProcessStdio::piped());

    let child = command.spawn().map_err(|e| {
        let err_msg = if e.kind() == std::io::ErrorKind::NotFound {
            "Git command not found. Please ensure Git is installed and in your system's PATH.".to_string()
        } else {
            format!("Failed to execute git clone command for {} (temp): {}", node_name, e)
        };
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, node_name, &err_msg);
        err_msg
    })?;

    let output = child.wait_with_output().await.map_err(|e| {
        let err_msg = format!("Failed to wait for git clone command for {} (temp): {}", node_name, e);
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, node_name, &err_msg);
        err_msg
    })?;

    if !output.status.success() {
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let err_msg = format!(
            "Failed to clone {} (temp). Git command exited with error. Status: {}. Stderr: {}. Stdout: {}",
            node_name, output.status, stderr_str.trim(), stdout_str.trim()
        );
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, node_name, &err_msg);
        // Cleanup temp dir on failure
        if temp_clone_path.exists() {
            if let Err(e_rm) = fs::remove_dir_all(&temp_clone_path).await {
                log::warn!("[CUSTOM_NODE_SETUP] Failed to clean up temporary directory {} after failed clone: {}", temp_clone_path.display(), e_rm);
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
    RGTHREE_NODES_NODE_NAME, RGTHREE_NODES_REPO_URL,
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



pub async fn clone_rgthree_comfy_nodes(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    clone_repository_to_custom_nodes(
        app_handle,
        RGTHREE_NODES_NODE_NAME,
        RGTHREE_NODES_REPO_URL,
        None, // No specific dependencies to install for rgthree-comfy via requirements.txt
    ).await
}