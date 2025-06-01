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

const IPADAPTER_PLUS_REPO_URL: &str = "https://github.com/cubiq/ComfyUI_IPAdapter_plus.git";
const IPADAPTER_PLUS_NODE_NAME: &str = "ComfyUI_IPAdapter_plus";

const IMPACT_PACK_REPO_URL: &str = "https://github.com/ltdrdata/ComfyUI-Impact-Pack.git";
const IMPACT_PACK_NODE_NAME: &str = "ComfyUI-Impact-Pack";

const IMPACT_SUBPACK_REPO_URL: &str = "https://github.com/ltdrdata/ComfyUI-Impact-Subpack.git";
const IMPACT_SUBPACK_NODE_NAME: &str = "ComfyUI-Impact-Subpack";

// New Custom Nodes from architectural doc (verified)
const SMZ_NODES_REPO_URL: &str = "https://github.com/shiimizu/ComfyUI_smZNodes.git";
const SMZ_NODES_NODE_NAME: &str = "ComfyUI_smZNodes";

const INSTANTID_REPO_URL: &str = "https://github.com/cubiq/ComfyUI_InstantID.git";
const INSTANTID_NODE_NAME: &str = "ComfyUI_InstantID";

const IC_LIGHT_REPO_URL: &str = "https://github.com/kijai/ComfyUI-IC-Light.git";
const IC_LIGHT_NODE_NAME: &str = "ComfyUI-IC-Light";

// rgthree-comfy is optional and doesn't have specific python deps or models, standard clone.
const RGTHREE_NODES_REPO_URL: &str = "https://github.com/rgthree/rgthree-comfy.git";
const RGTHREE_NODES_NODE_NAME: &str = "rgthree-comfy";

const COMFYUI_CLIPSEG_REPO_URL: &str = "https://github.com/time-river/ComfyUI-CLIPSeg.git";
const COMFYUI_CLIPSEG_NODE_NAME: &str = "ComfyUI-CLIPSeg";

const ONNXRUNTIME_PACKAGE: &str = "onnxruntime";
// TODO: Investigate installing onnxruntime-gpu as per ComfyUI-InstantID README
const INSIGHTFACE_PACKAGE: &str = "insightface";

const CHECK_ONNX_SCRIPT_NAME: &str = "check_onnx.py";
const CHECK_INSIGHTFACE_SCRIPT_NAME: &str = "check_insightface.py";

// Removed local get_comfyui_base_path, will use python_utils::get_comfyui_directory_path
// Removed local get_venv_python_executable, will use python_utils::get_venv_python_executable_path

/// Executes a Python script and checks its exit code.
async fn execute_python_script_check(
    python_executable: &Path,
    script_content: &str,
    script_name: &str,
    comfyui_base_path: &Path,
) -> Result<bool, String> {
    let script_path = comfyui_base_path.join(script_name);
    fs::write(&script_path, script_content).await.map_err(|e| {
        format!("Failed to write Python script {}: {}", script_path.display(), e)
    })?;

    debug!("Executing Python script check: {} with script {}", python_executable.display(), script_name);
    let mut cmd = TokioCommand::new(python_executable);
    cmd.arg(&script_path)
        .current_dir(comfyui_base_path) // Run script from comfyui_base_path
        .stdout(ProcessStdio::piped())
        .stderr(ProcessStdio::piped());

    let child = cmd.spawn().map_err(|e| format!("Failed to spawn script {}: {}", script_name, e))?;
    let output = child.wait_with_output().await.map_err(|e| format!("Failed to wait for script {}: {}", script_name, e))?;
    
    fs::remove_file(&script_path).await.map_err(|e| format!("Failed to remove script {}: {}", script_path.display(), e))?;

    if output.status.success() {
        debug!("Script {} executed successfully.", script_name);
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!("Script {} failed. Stderr: {}", script_name, stderr.trim());
        Ok(false)
    }
}

/// Runs a pip command in the ComfyUI virtual environment.
async fn run_pip_command(
    app_handle: &AppHandle<Wry>,
    python_executable: &Path,
    args: &[&str],
    package_name_for_event: &str, // For emitting specific package events
    method: &str, // "pip" or "wheel"
) -> Result<(), String> {
    info!("[PIP_INSTALL] Running pip command for {}: {:?} with args: {:?}", package_name_for_event, python_executable.display(), args);
    emit_event(
        app_handle,
        "PackageInstallStart",
        Some(json!({ "packageName": package_name_for_event, "method": method })),
    );

    let mut cmd = TokioCommand::new(python_executable);
    cmd.arg("-m").arg("pip").args(args)
        .stdout(ProcessStdio::piped())
        .stderr(ProcessStdio::piped());
    
    // It's generally better to run pip from the comfyui_base_path if not otherwise specified
    if let Some(comfyui_root) = python_executable.ancestors().nth(if cfg!(windows) {3} else {2}) {
        if comfyui_root.join("pyproject.toml").exists() || comfyui_root.join("setup.py").exists() || comfyui_root.join("requirements.txt").exists() {
             cmd.current_dir(comfyui_root);
             debug!("Set pip command working directory to: {}", comfyui_root.display());
        } else {
            warn!("Could not determine a suitable ComfyUI root for pip command from {}", python_executable.display());
        }
    }


    let mut child = cmd.spawn().map_err(|e| {
        let err_msg = format!("Failed to spawn pip command for {}: {}", package_name_for_event, e);
        error!("{}", err_msg);
        emit_event(
            app_handle,
            "PackageInstallFailed",
            Some(json!({ "packageName": package_name_for_event, "error": err_msg.clone(), "osHint": serde_json::Value::Null })),
        );
        err_msg
    })?;

    let stdout = child.stdout.take().ok_or(format!("Failed to capture stdout for pip command for {}", package_name_for_event))?;
    let stderr = child.stderr.take().ok_or(format!("Failed to capture stderr for pip command for {}", package_name_for_event))?;

    let app_handle_clone_stdout = app_handle.clone();
    let package_name_clone_stdout = package_name_for_event.to_string();
    let stdout_task = tokio::task::spawn(async move {
        let mut reader = TokioBufReader::new(stdout);
        let mut line_buf = String::new();
        while let Ok(n) = reader.read_line(&mut line_buf).await {
            if n == 0 { break; } // EOF
            let line_to_process = line_buf.trim_end().to_string();
            info!("[PIP_INSTALL] Stdout for {}: {}", package_name_clone_stdout, line_to_process);

            // More aggressive filtering for pip stdout
            let lower_line = line_to_process.to_lowercase();
            let is_significant_action = lower_line.starts_with("installing ") || // "Installing collected packages", "Installing X..."
                                        lower_line.starts_with("successfully installed") ||
                                        lower_line.starts_with("collecting ") || // Show initial "Collecting X, Y, Z"
                                        lower_line.starts_with("downloading ");  // Show initial "Downloading X"

            let is_too_verbose = lower_line.starts_with("debug:") ||
                                 lower_line.starts_with("requirement already satisfied") ||
                                 lower_line.starts_with("using cached") ||
                                 lower_line.contains("looking in indexes:") ||
                                 lower_line.contains("processing ") && lower_line.contains(".whl") ||
                                 lower_line.contains("http://") || lower_line.contains("https://") || // Filter URLs
                                 lower_line.contains("satisfied constraint") ||
                                 lower_line.contains("source distribution") ||
                                 lower_line.contains("building wheel") ||
                                 lower_line.contains("running setup.py") ||
                                 lower_line.contains("creating build") ||
                                 lower_line.contains("copying") && lower_line.contains("to build") ||
                                 lower_line.starts_with("  ") && package_name_clone_stdout == "pip"; // Filter indented pip update details

            if is_significant_action || !is_too_verbose {
                emit_event(
                    &app_handle_clone_stdout,
                    "pip-output",
                    Some(json!({ "packageName": package_name_clone_stdout, "output": line_to_process, "stream": "stdout" })),
                );
            } else {
                debug!("[PIP_INSTALL] Filtered (stdout) for {}: {}", package_name_clone_stdout, line_to_process);
            }
            line_buf.clear();
        }
    });

    let app_handle_clone_stderr = app_handle.clone();
    let package_name_clone_stderr = package_name_for_event.to_string();
    let stderr_task = tokio::task::spawn(async move {
        let mut reader = TokioBufReader::new(stderr);
        let mut line_buf = String::new();
        while let Ok(n) = reader.read_line(&mut line_buf).await {
            if n == 0 { break; } // EOF
            let line_to_process = line_buf.trim_end().to_string();
            error!("[PIP_INSTALL] Stderr for {}: {}", package_name_clone_stderr, line_to_process);
            
            // More aggressive filtering for pip stderr
            let lower_line = line_to_process.to_lowercase();
            let is_truly_error_indicative = !lower_line.contains("defaulting to user installation") &&
                                           !lower_line.contains("consider adding this directory to path") &&
                                           !lower_line.contains("requirement already satisfied") && // Often not an error
                                           !(lower_line.starts_with("warning: the script ") && lower_line.contains("is installed in")) && // Path warnings
                                           !(lower_line.starts_with("warning:") && lower_line.contains(" βρίσκεται ")) && // Greek path warning
                                           !lower_line.contains("skipping link:"); // Git/symlink messages sometimes go to stderr

            if is_truly_error_indicative {
                emit_event(
                    &app_handle_clone_stderr,
                    "pip-output",
                    Some(json!({ "packageName": package_name_clone_stderr, "output": line_to_process, "stream": "stderr" })),
                );
            } else {
                // Log as info or debug if it's a common, non-critical stderr message
                info!("[PIP_INSTALL] Filtered/Demoted (stderr) for {}: {}", package_name_clone_stderr, line_to_process);
            }
            line_buf.clear();
        }
    });

    let status = child.wait().await.map_err(|e| {
        let err_msg = format!("Failed to wait for pip command for {}: {}", package_name_for_event, e);
        error!("{}", err_msg);
        emit_event(
            app_handle,
            "PackageInstallFailed",
            Some(json!({ "packageName": package_name_for_event, "error": err_msg.clone(), "osHint": serde_json::Value::Null })),
        );
        err_msg
    })?;

    stdout_task.await.map_err(|e| format!("Stdout task for pip command for {} panicked: {:?}", package_name_for_event, e))?;
    stderr_task.await.map_err(|e| format!("Stderr task for pip command for {} panicked: {:?}", package_name_for_event, e))?;


    if status.success() {
        info!("[PIP_INSTALL] Successfully installed/updated {}.", package_name_for_event);
        emit_event(
            app_handle,
            "PackageInstallSuccess",
            Some(json!({ "packageName": package_name_for_event })),
        );
        Ok(())
    } else {
        let err_msg = format!(
            "Pip command for {} failed with status: {:?}",
            package_name_for_event, status
        );
        error!("{}", err_msg);
        emit_event(
            app_handle,
            "PackageInstallFailed",
            Some(json!({ "packageName": package_name_for_event, "error": err_msg.clone(), "osHint": serde_json::Value::Null })), // osHint will be added later for specific cases
        );
        Err(err_msg)
    }
}


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
    clone_custom_node_repo(
        app_handle,
        COMFYUI_CLIPSEG_NODE_NAME,
        COMFYUI_CLIPSEG_REPO_URL,
        Some(|app_handle_param, node_name_param, pack_dir_param| {
            // ComfyUI-CLIPSeg might have its own requirements.txt
            let pack_dir_owned = pack_dir_param.to_path_buf();
            Box::pin(install_custom_node_dependencies(app_handle_param.clone(), node_name_param.to_string(), pack_dir_owned))
        })
    )
    .await
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