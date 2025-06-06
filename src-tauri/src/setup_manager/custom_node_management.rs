// metamorphosis-app/src-tauri/src/setup_manager/custom_node_management.rs
use std::fs as std_fs; // Renamed to avoid conflict with tokio::fs
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader}; // Added import
use std::path::Path;
use std::process::Stdio as ProcessStdio;
use std::pin::Pin; // Added for Future type
use std::future::Future; // Added for Future type
use std::env; // Added back for consts::OS
use tokio::fs;
use tokio::process::Command as TokioCommand;
use tauri::{AppHandle, Wry}; // Removed unused Manager
use log::{info, error, warn, debug};
use serde_json::json;

use super::event_utils::{
    emit_custom_node_clone_start,
    emit_custom_node_clone_success,
    emit_custom_node_already_exists,
    emit_custom_node_clone_failed,
    // Assuming these will be added or are available:
    // emit_event, // Generic event emitter
};
// Import new python_utils functions
use super::python_utils::{
    get_python_version,
    download_file,
    get_comfyui_directory_path,
    get_venv_python_executable_path,
    // get_script_path, // If needed later for script-based checks
};
use crate::setup_manager::event_utils::emit_event; // Using the specific emit_event
use crate::setup_manager::dependency_manager::install_custom_node_dependencies; // Changed from crate::dependency_management
use crate::gpu_detection::{get_gpu_info, GpuType}; // Import GPU detection utilities


// Removed local get_comfyui_base_path, will use python_utils::get_comfyui_directory_path
// Removed local get_venv_python_executable, will use python_utils::get_venv_python_executable_path



/// Generic function to clone a custom node repository and install its dependencies.
async fn clone_custom_node_repo(
    app_handle: &AppHandle<Wry>,
    node_name: &str,
    repo_url: &str,
    install_dependencies_fn: Option<fn(&AppHandle<Wry>, &str, &Path) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>>,
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

/// Clones the ComfyUI_IPAdapter_plus custom node repository.
pub async fn clone_comfyui_ipadapter_plus(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    clone_custom_node_repo(
        app_handle,
        IPADAPTER_PLUS_NODE_NAME,
        IPADAPTER_PLUS_REPO_URL,
        Some(|app_handle_param, _node_name, _pack_dir| {
            Box::pin(install_insightface_dependencies(app_handle_param.clone()))
        })
    ).await
}

/// Clones the ComfyUI-Impact-Pack custom node repository and installs its dependencies.
pub async fn clone_comfyui_impact_pack(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    clone_custom_node_repo(
        app_handle,
        IMPACT_PACK_NODE_NAME,
        IMPACT_PACK_REPO_URL,
        Some(|app_handle_param, node_name_param, pack_dir_param| {
            // Ensure pack_dir_param has a 'static lifetime or is owned
            let pack_dir_owned = pack_dir_param.to_path_buf();
            Box::pin(install_custom_node_dependencies(app_handle_param.clone(), node_name_param.to_string(), pack_dir_owned))
        })
    ).await
}

/// Clones the ComfyUI-Impact-Subpack custom node repository and installs its dependencies.
pub async fn clone_comfyui_impact_subpack(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    clone_custom_node_repo(
        app_handle,
        IMPACT_SUBPACK_NODE_NAME,
        IMPACT_SUBPACK_REPO_URL,
        Some(|app_handle_param, node_name_param, pack_dir_param| {
            // Ensure pack_dir_param has a 'static lifetime or is owned
            let pack_dir_owned = pack_dir_param.to_path_buf();
            Box::pin(install_custom_node_dependencies(app_handle_param.clone(), node_name_param.to_string(), pack_dir_owned))
        })
    ).await
}

/// Clones the ComfyUI_smZNodes custom node repository.
pub async fn clone_comfyui_smz_nodes(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    clone_custom_node_repo(
        app_handle,
        SMZ_NODES_NODE_NAME,
        SMZ_NODES_REPO_URL,
        None, // No specific post-clone dependency installation logic beyond standard requirements.txt if present
    )
    .await
}

/// Clones the ComfyUI_InstantID custom node repository and installs its dependencies.
pub async fn clone_comfyui_instantid(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    clone_custom_node_repo(
        app_handle,
        INSTANTID_NODE_NAME,
        INSTANTID_REPO_URL,
        Some(|app_handle_param_ref: &AppHandle<Wry>, node_name_param_ref: &str, pack_dir_param_ref: &Path| {
            // Create owned versions of all captures needed by the async block
            let owned_app_handle_for_deps = app_handle_param_ref.clone();
            let owned_app_handle_for_insightface = app_handle_param_ref.clone();
            let owned_node_name = node_name_param_ref.to_string();
            let owned_pack_dir = pack_dir_param_ref.to_path_buf();

            Box::pin(async move {
                // These owned versions are moved into the async block
                install_custom_node_dependencies(owned_app_handle_for_deps, owned_node_name, owned_pack_dir).await?;
                install_insightface_dependencies(owned_app_handle_for_insightface).await
            })
        }),
    )
    .await
}

/// Clones the ComfyUI-IC-Light custom node repository.
pub async fn clone_comfyui_ic_light(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    clone_custom_node_repo(
        app_handle,
        IC_LIGHT_NODE_NAME,
        IC_LIGHT_REPO_URL,
        Some(|app_handle_param, node_name_param, pack_dir_param| {
            // IC-Light might have its own requirements.txt
            let pack_dir_owned = pack_dir_param.to_path_buf();
            Box::pin(install_custom_node_dependencies(app_handle_param.clone(), node_name_param.to_string(), pack_dir_owned))
        })
    )
    .await
}

/// Clones the rgthree-comfy custom node repository.
pub async fn clone_rgthree_comfy_nodes(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    clone_custom_node_repo(
        app_handle,
        RGTHREE_NODES_NODE_NAME,
        RGTHREE_NODES_REPO_URL,
        None, // rgthree-comfy typically doesn't have a requirements.txt
    )
    .await
}

/// Clones the ComfyUI-CLIPSeg custom node repository and installs its dependencies.
pub async fn clone_comfyui_clipseg(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    info!("[CUSTOM_NODE_SETUP] Attempting to install {}...", COMFYUI_CLIPSEG_NODE_NAME);
    emit_custom_node_clone_start(app_handle, COMFYUI_CLIPSEG_NODE_NAME);

    let comfyui_base_path = get_comfyui_directory_path(app_handle)?;
    let custom_nodes_dir = comfyui_base_path.join("custom_nodes");
    let final_clipseg_py_path = custom_nodes_dir.join("clipseg.py");

    if !custom_nodes_dir.exists() {
        std_fs::create_dir_all(&custom_nodes_dir).map_err(|e| {
            let err_msg = format!("Failed to create custom_nodes directory at {}: {}", custom_nodes_dir.display(), e);
            error!("[CUSTOM_NODE_SETUP] {}", err_msg);
            emit_custom_node_clone_failed(app_handle, COMFYUI_CLIPSEG_NODE_NAME, &err_msg);
            err_msg
        })?;
        info!("[CUSTOM_NODE_SETUP] Created custom_nodes directory: {}", custom_nodes_dir.display());
    }

    if final_clipseg_py_path.exists() {
        info!("[CUSTOM_NODE_SETUP] {} already exists at {}. Skipping installation.", COMFYUI_CLIPSEG_NODE_NAME, final_clipseg_py_path.display());
        emit_custom_node_already_exists(app_handle, COMFYUI_CLIPSEG_NODE_NAME);
        // Even if clipseg.py exists, we might want to ensure dependencies are checked/installed.
        // For now, let's assume if clipseg.py is there, deps were handled.
        // If strict dependency re-check is needed, logic can be added here.
        return Ok(());
    }

    // Create a unique temporary directory for cloning
    let temp_clone_dir_name = format!("ComfyUI-CLIPSeg_temp_{}", uuid::Uuid::new_v4());
    let temp_clone_path = env::temp_dir().join("metamorphosis_clones").join(&temp_clone_dir_name);
    
    // Ensure the parent directory for temp clones exists
    if let Some(parent) = temp_clone_path.parent() {
        if !parent.exists() {
            std_fs::create_dir_all(parent).map_err(|e| {
                format!("Failed to create parent temp clone directory at {}: {}", parent.display(), e)
            })?;
        }
    }


    info!("[CUSTOM_NODE_SETUP] Cloning {} to temporary directory: {}", COMFYUI_CLIPSEG_REPO_URL, temp_clone_path.display());

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

    command.arg("clone").arg(COMFYUI_CLIPSEG_REPO_URL).arg(&git_temp_clone_path_arg_string)
        .stdout(ProcessStdio::piped())
        .stderr(ProcessStdio::piped());

    let child = command.spawn().map_err(|e| {
        let err_msg = if e.kind() == std::io::ErrorKind::NotFound {
            "Git command not found. Please ensure Git is installed and in your system's PATH.".to_string()
        } else {
            format!("Failed to execute git clone command for {} (temp): {}", COMFYUI_CLIPSEG_NODE_NAME, e)
        };
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, COMFYUI_CLIPSEG_NODE_NAME, &err_msg);
        err_msg
    })?;

    let output = child.wait_with_output().await.map_err(|e| {
        let err_msg = format!("Failed to wait for git clone command for {} (temp): {}", COMFYUI_CLIPSEG_NODE_NAME, e);
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, COMFYUI_CLIPSEG_NODE_NAME, &err_msg);
        err_msg
    })?;

    if !output.status.success() {
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let err_msg = format!(
            "Failed to clone {} (temp). Git command exited with error. Status: {}. Stderr: {}. Stdout: {}",
            COMFYUI_CLIPSEG_NODE_NAME, output.status, stderr_str.trim(), stdout_str.trim()
        );
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, COMFYUI_CLIPSEG_NODE_NAME, &err_msg);
        // Cleanup temp dir on failure
        if temp_clone_path.exists() {
            if let Err(e_rm) = fs::remove_dir_all(&temp_clone_path).await {
                warn!("[CUSTOM_NODE_SETUP] Failed to clean up temporary directory {} after failed clone: {}", temp_clone_path.display(), e_rm);
            }
        }
        return Err(err_msg);
    }

    info!("[CUSTOM_NODE_SETUP] Successfully cloned {} to temporary directory {}", COMFYUI_CLIPSEG_NODE_NAME, temp_clone_path.display());

    // Copy clipseg.py
    let temp_clipseg_py_src_path = temp_clone_path.join("custom_nodes").join("clipseg.py"); // Adjusted path
    if !temp_clipseg_py_src_path.exists() {
        let err_msg = format!("clipseg.py not found in temporary clone at {}", temp_clipseg_py_src_path.display());
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, COMFYUI_CLIPSEG_NODE_NAME, &err_msg);
        if temp_clone_path.exists() {
            if let Err(e_rm) = fs::remove_dir_all(&temp_clone_path).await {
                warn!("[CUSTOM_NODE_SETUP] Failed to clean up temporary directory {}: {}", temp_clone_path.display(), e_rm);
            }
        }
        return Err(err_msg);
    }

    info!("[CUSTOM_NODE_SETUP] Copying {} from {} to {}", "clipseg.py", temp_clipseg_py_src_path.display(), final_clipseg_py_path.display());
    if let Err(e) = fs::copy(&temp_clipseg_py_src_path, &final_clipseg_py_path).await {
        let err_msg = format!("Failed to copy clipseg.py from {} to {}: {}", temp_clipseg_py_src_path.display(), final_clipseg_py_path.display(), e);
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, COMFYUI_CLIPSEG_NODE_NAME, &err_msg);
        if temp_clone_path.exists() {
            if let Err(e_rm_inner) = fs::remove_dir_all(&temp_clone_path).await {
                warn!("[CUSTOM_NODE_SETUP] Failed to clean up temporary directory {} after failed copy: {}", temp_clone_path.display(), e_rm_inner);
            }
        }
        return Err(err_msg);
    }
    info!("[CUSTOM_NODE_SETUP] Successfully copied clipseg.py to {}", final_clipseg_py_path.display());


    // Install dependencies from requirements.txt in the temporary clone
    let temp_requirements_txt_path = temp_clone_path.join("requirements.txt");
    if temp_requirements_txt_path.exists() {
        info!("[CUSTOM_NODE_SETUP] Found requirements.txt for {} in temporary clone at {}. Installing dependencies...", COMFYUI_CLIPSEG_NODE_NAME, temp_requirements_txt_path.display());
        if let Err(e) = install_custom_node_dependencies(app_handle.clone(), COMFYUI_CLIPSEG_NODE_NAME.to_string(), temp_clone_path.clone()).await {
            let err_msg = format!("Failed to install dependencies for {}: {}", COMFYUI_CLIPSEG_NODE_NAME, e);
            error!("[CUSTOM_NODE_SETUP] {}", err_msg);
            // Don't emit_custom_node_clone_failed here as install_custom_node_dependencies has its own event emitting
            if temp_clone_path.exists() {
                if let Err(e_rm_inner_deps) = fs::remove_dir_all(&temp_clone_path).await {
                    warn!("[CUSTOM_NODE_SETUP] Failed to clean up temporary directory {} after failed dependency install: {}", temp_clone_path.display(), e_rm_inner_deps);
                }
            }
            return Err(err_msg);
        }
        info!("[CUSTOM_NODE_SETUP] Successfully processed dependencies for {}.", COMFYUI_CLIPSEG_NODE_NAME);
    } else {
        info!("[CUSTOM_NODE_SETUP] No requirements.txt found for {} in temporary clone. Skipping dependency installation.", COMFYUI_CLIPSEG_NODE_NAME);
    }
    
    // Also install general insightface dependencies, as CLIPSeg might rely on onnxruntime which is part of insightface setup
    // This might be redundant if requirements.txt already handles onnxruntime, but it's safer.
    info!("[CUSTOM_NODE_SETUP] Ensuring general onnxruntime/insightface dependencies are met for {}.", COMFYUI_CLIPSEG_NODE_NAME);
    if let Err(e) = install_insightface_dependencies(app_handle.clone()).await {
        let err_msg = format!("Failed to install general insightface/onnxruntime dependencies for {}: {}", COMFYUI_CLIPSEG_NODE_NAME, e);
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        if temp_clone_path.exists() {
            if let Err(e_rm_inner_insight) = fs::remove_dir_all(&temp_clone_path).await {
                warn!("[CUSTOM_NODE_SETUP] Failed to clean up temporary directory {} after failed insightface dependency install: {}", temp_clone_path.display(), e_rm_inner_insight);
            }
        }
        return Err(err_msg);
    }

    // Cleanup temporary directory
    info!("[CUSTOM_NODE_SETUP] Cleaning up temporary directory: {}", temp_clone_path.display());
    if temp_clone_path.exists() { // Check before attempting to remove
        fs::remove_dir_all(&temp_clone_path).await.map_err(|e| {
            // This is not a critical failure for the node installation itself, so just warn.
            let warn_msg = format!("Failed to clean up temporary directory {}: {}. Manual cleanup might be required.", temp_clone_path.display(), e);
            warn!("[CUSTOM_NODE_SETUP] {}", warn_msg);
            // Return Ok here as the primary operation (copying clipseg.py) succeeded.
            // If cleanup failure should be an error, change this to Err(warn_msg).
            warn_msg // Or just return Ok(()) if we don't want to propagate this warning as an error.
                     // For now, let's make it a non-blocking warning.
        }).unwrap_or_else(|_warn_msg| {
            // This block executes if remove_dir_all itself returns an error that we map_err'd.
            // Since we are only warning, we don't need to do anything special here.
        });
    } else {
        info!("[CUSTOM_NODE_SETUP] Temporary directory {} was already removed or never existed.", temp_clone_path.display());
    }


    info!("[CUSTOM_NODE_SETUP] Successfully installed {}.", COMFYUI_CLIPSEG_NODE_NAME);
    emit_custom_node_clone_success(app_handle, COMFYUI_CLIPSEG_NODE_NAME);
    Ok(())
}
/// Installs insightface and its dependencies (onnxruntime).
// Removed comfyui_base_path_arg as it's unused; comfyui_base_path is fetched internally.
pub async fn install_insightface_dependencies(app_handle: AppHandle<Wry>) -> Result<(), String> { // Changed to take owned AppHandle
    let comfyui_base_path = get_comfyui_directory_path(&app_handle)?; // Pass reference here
    info!("[INSIGHTFACE_SETUP] Starting insightface dependency installation in {}...", comfyui_base_path.display());
    
    // Use the utility function to get the venv python executable
    let python_executable = get_venv_python_executable_path(&app_handle)?; // Pass reference here

    if !python_executable.exists() {
        let err_msg = format!("Python executable not found at {}", python_executable.display());
        error!("{}", err_msg);
        emit_event(&app_handle, "PackageInstallFailed", Some(json!({"packageName": "insightface_dependencies", "error": err_msg.clone() }))); // Pass reference here
        return Err(err_msg);
    }
    info!("Using Python executable: {}", python_executable.display());

    // 1. Update pip
    run_pip_command(&app_handle, &python_executable, &["install", "-U", "pip"], "pip", "pip").await?; // Pass reference here

    // 2. Check and install ONNX Runtime (conditionally GPU version)
    let gpu_info = get_gpu_info();
    info!("[INSIGHTFACE_SETUP] Detected GPU Info: {:?}", gpu_info);

    let onnx_package_to_install: &str;
    let onnx_package_display_name: String;

    if gpu_info.gpu_type == GpuType::Nvidia && gpu_info.cuda_version.is_some() {
        // TODO: Confirm exact pip package name for onnxruntime-gpu and if it needs specific CUDA versioning.
        // For now, assuming "onnxruntime-gpu" is a general package that picks up CUDA.
        // A more robust solution might involve mapping cuda_version (e.g., "11.8", "12.1") to specific
        // onnxruntime-gpu wheels if necessary, e.g., onnxruntime-gpu-cuda11, onnxruntime-gpu-cuda12.
        onnx_package_to_install = "onnxruntime-gpu";
        onnx_package_display_name = format!("{} (GPU for CUDA {:?})", ONNXRUNTIME_PACKAGE, gpu_info.cuda_version.unwrap_or_default());
        info!("[INSIGHTFACE_SETUP] Attempting to install ONNX Runtime for GPU (NVIDIA CUDA detected). Package: {}", onnx_package_to_install);
    } else {
        onnx_package_to_install = "onnxruntime";
        onnx_package_display_name = ONNXRUNTIME_PACKAGE.to_string();
        info!("[INSIGHTFACE_SETUP] Attempting to install standard ONNX Runtime (No NVIDIA CUDA detected or GPU is not NVIDIA). Package: {}", onnx_package_to_install);
    }
    
    let onnx_script_content = "import onnxruntime; print(onnxruntime.__version__)";
    // Check if *any* onnxruntime is installed. If so, assume it's okay for now.
    // A more advanced check might verify if the *correct* type (CPU/GPU) is installed.
    let is_onnx_installed = execute_python_script_check(&python_executable, onnx_script_content, CHECK_ONNX_SCRIPT_NAME, &comfyui_base_path).await?;
    
    if is_onnx_installed {
        // TODO: Add a check here to see if the *correct* version (CPU vs GPU) is installed if we want to be more robust.
        // For now, if any onnxruntime is importable, we skip. This might lead to issues if CPU version is present but GPU is needed.
        info!("[INSIGHTFACE_SETUP] An ONNX Runtime (version unknown type) already installed. Display Name: {}", onnx_package_display_name);
        emit_event(&app_handle, "PackageAlreadyInstalled", Some(json!({ "packageName": onnx_package_display_name })));
    } else {
        info!("[INSIGHTFACE_SETUP] Installing {}...", onnx_package_display_name);
        run_pip_command(&app_handle, &python_executable, &["install", onnx_package_to_install], &onnx_package_display_name, "pip").await?;
    }

    // 3. Check and install Insightface
    let insightface_script_content = "import insightface; print(insightface.__version__)";
    let is_insightface_installed = execute_python_script_check(&python_executable, insightface_script_content, CHECK_INSIGHTFACE_SCRIPT_NAME, &comfyui_base_path).await?;
    if is_insightface_installed {
        info!("[INSIGHTFACE_SETUP] {} already installed.", INSIGHTFACE_PACKAGE);
        emit_event(&app_handle, "PackageAlreadyInstalled", Some(json!({ "packageName": INSIGHTFACE_PACKAGE }))); // Pass reference here
        return Ok(());
    }

    info!("[INSIGHTFACE_SETUP] Installing {}...", INSIGHTFACE_PACKAGE);
    let os_type = env::consts::OS;
    match os_type {
        "windows" => {
            let py_version = get_python_version(&app_handle, &python_executable).await.map_err(|e| { // Pass reference here
                let err_msg = format!("Failed to get Python version for Windows wheel selection: {}", e);
                error!("{}", err_msg);
                // get_python_version emits PythonVersionDetected on success,
                // so we only need to emit PackageInstallFailed here.
                emit_event(&app_handle, "PackageInstallFailed", Some(json!({"packageName": INSIGHTFACE_PACKAGE, "error": err_msg.clone(), "detail": "Python version detection failed."}))); // Pass reference here
                err_msg
            })?;
            // emit_event for PythonVersionDetected is handled by get_python_version itself.

            // Wheel URLs from the plan:
            // Python 3.10: https://github.com/Gourieff/Assets/raw/main/Insightface/insightface-0.7.3-cp310-cp310-win_amd64.whl
            // Python 3.11: https://github.com/Gourieff/Assets/raw/main/Insightface/insightface-0.7.3-cp311-cp311-win_amd64.whl
            // Python 3.12: (Assuming a similar pattern or a generic one if available)
            // For now, let's use a placeholder or the most common one if a direct 3.12 isn't listed.
            // The plan mentions 3.10, 3.11, 3.12. Let's assume 3.12 is also cp312.
            let wheel_url = match py_version.as_str() {
                "3.10" => "https://github.com/Gourieff/Assets/raw/main/Insightface/insightface-0.7.3-cp310-cp310-win_amd64.whl",
                "3.11" => "https://github.com/Gourieff/Assets/raw/main/Insightface/insightface-0.7.3-cp311-cp311-win_amd64.whl",
                "3.12" => "https://github.com/Gourieff/Assets/raw/main/Insightface/insightface-0.7.3-cp312-cp312-win_amd64.whl", // Assuming this exists
                _ => {
                    let err_msg = format!("Unsupported Python version for Insightface prebuilt wheel: {}. Please use Python 3.10, 3.11, or 3.12.", py_version);
                    error!("{}", err_msg);
                    emit_event(&app_handle, "PackageInstallFailed", Some(json!({"packageName": INSIGHTFACE_PACKAGE, "error": err_msg.clone()}))); // Pass reference here
                    return Err(err_msg);
                }
            };
            let wheel_name = Path::new(wheel_url).file_name().unwrap_or_default().to_str().unwrap_or("insightface_wheel.whl");
            
            let temp_dir = comfyui_base_path.join("temp_downloads"); // Or use app_handle.path().app_cache_dir()
            fs::create_dir_all(&temp_dir).await.map_err(|e| format!("Failed to create temp dir for wheel: {}", e))?;

            let wheel_path = download_file(wheel_url, &temp_dir, wheel_name, &app_handle).await?; // Pass reference here
            run_pip_command(&app_handle, &python_executable, &["install", wheel_path.to_str().unwrap()], INSIGHTFACE_PACKAGE, "wheel").await?; // Pass reference here
            fs::remove_file(&wheel_path).await.map_err(|e| format!("Failed to remove temporary wheel file {}: {}", wheel_path.display(), e))?;
            info!("[INSIGHTFACE_SETUP] Successfully installed {} from wheel.", INSIGHTFACE_PACKAGE);
        }
        "macos" | "linux" => {
            let install_result = run_pip_command(&app_handle, &python_executable, &["install", "-U", "insightface"], INSIGHTFACE_PACKAGE, "pip").await; // Pass reference here
            if let Err(e) = install_result {
                // Check for common compilation error messages
                let os_hint = if e.to_lowercase().contains("cmake") || e.to_lowercase().contains("build_ext") || e.to_lowercase().contains("failed building wheel") {
                    if os_type == "macos" {
                        Some("Installation failed. This might be due to missing build tools. Please try running 'xcode-select --install' in your terminal and then restart the application or retry the setup.".to_string())
                    } else { // linux
                        Some("Installation failed. This might be due to missing build tools. Please try running 'sudo apt update && sudo apt install build-essential python3-dev cmake' (or equivalent for your distribution) in your terminal and then restart the application or retry the setup.".to_string())
                    }
                } else {
                    None
                };

                let err_msg = format!("Failed to install {} via pip on {}: {}. Hint: {:?}", INSIGHTFACE_PACKAGE, os_type, e, os_hint);
                error!("{}", err_msg);
                 emit_event(
                    &app_handle, // Pass reference here
                    "PackageInstallFailed",
                    Some(json!({ "packageName": INSIGHTFACE_PACKAGE, "error": e, "osHint": os_hint })),
                );
                return Err(err_msg);
            }
        }
        _ => {
            let err_msg = format!("Unsupported OS for Insightface installation: {}", os_type);
            error!("{}", err_msg);
            emit_event(&app_handle, "PackageInstallFailed", Some(json!({"packageName": INSIGHTFACE_PACKAGE, "error": err_msg.clone()}))); // Pass reference here
            return Err(err_msg);
        }
    }

    info!("[INSIGHTFACE_SETUP] Insightface dependency installation completed.");
    Ok(())
}