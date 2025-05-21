// metamorphosis-app/src-tauri/src/setup_manager/custom_node_management.rs
use std::fs as std_fs; // Renamed to avoid conflict with tokio::fs
use std::path::Path; // PathBuf is unused
use std::process::Stdio as ProcessStdio;
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

const IPADAPTER_PLUS_REPO_URL: &str = "https://github.com/cubiq/ComfyUI_IPAdapter_plus.git";
const IPADAPTER_PLUS_NODE_NAME: &str = "ComfyUI_IPAdapter_plus";

const ONNXRUNTIME_PACKAGE: &str = "onnxruntime";
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


    let child = cmd.spawn().map_err(|e| {
        let err_msg = format!("Failed to spawn pip command for {}: {}", package_name_for_event, e);
        error!("{}", err_msg);
        emit_event(
            app_handle,
            "PackageInstallFailed",
            Some(json!({ "packageName": package_name_for_event, "error": err_msg.clone(), "osHint": serde_json::Value::Null })),
        );
        err_msg
    })?;

    let output = child.wait_with_output().await.map_err(|e| {
        let err_msg = format!("Failed to wait for pip command for {}: {}", package_name_for_event, e);
        error!("{}", err_msg);
        emit_event(
            app_handle,
            "PackageInstallFailed",
            Some(json!({ "packageName": package_name_for_event, "error": err_msg.clone(), "osHint": serde_json::Value::Null })),
        );
        err_msg
    })?;

    if output.status.success() {
        info!("[PIP_INSTALL] Successfully installed/updated {}. Output: {}", package_name_for_event, String::from_utf8_lossy(&output.stdout));
        emit_event(
            app_handle,
            "PackageInstallSuccess",
            Some(json!({ "packageName": package_name_for_event })),
        );
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let err_msg = format!(
            "Pip command for {} failed. Status: {:?}. Stderr: {}. Stdout: {}",
            package_name_for_event, output.status.code(), stderr.trim(), stdout.trim()
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


/// Clones the ComfyUI_IPAdapter_plus custom node repository.
pub async fn clone_comfyui_ipadapter_plus(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    info!("[CUSTOM_NODE_SETUP] Attempting to clone {}...", IPADAPTER_PLUS_NODE_NAME);
    emit_custom_node_clone_start(app_handle, IPADAPTER_PLUS_NODE_NAME);

    let comfyui_base_path = get_comfyui_directory_path(app_handle)?;
    let custom_nodes_dir = comfyui_base_path.join("custom_nodes");
    let target_dir = custom_nodes_dir.join(IPADAPTER_PLUS_NODE_NAME);

    if !custom_nodes_dir.exists() {
        std_fs::create_dir_all(&custom_nodes_dir).map_err(|e| { // Changed to std_fs
            let err_msg = format!("Failed to create custom_nodes directory at {}: {}", custom_nodes_dir.display(), e);
            error!("[CUSTOM_NODE_SETUP] {}", err_msg);
            emit_custom_node_clone_failed(app_handle, IPADAPTER_PLUS_NODE_NAME, &err_msg);
            err_msg
        })?;
        info!("[CUSTOM_NODE_SETUP] Created custom_nodes directory: {}", custom_nodes_dir.display());
    }

    if target_dir.exists() {
        info!("[CUSTOM_NODE_SETUP] Target directory {} already exists. Skipping clone.", target_dir.display());
        emit_custom_node_already_exists(app_handle, IPADAPTER_PLUS_NODE_NAME);
        // After cloning, proceed to install dependencies
        // Call install_insightface_dependencies without the comfyui_base_path argument
        return install_insightface_dependencies(app_handle).await;
    }

    info!("[CUSTOM_NODE_SETUP] Cloning {} into {}", IPADAPTER_PLUS_REPO_URL, target_dir.display());

    // Using tokio::process::Command for async git clone
    let mut command = TokioCommand::new("git");
    // Prepare the target directory path for the git command.
    // On Windows, git might not handle the `\\?\` prefix, so we remove it.
    // The `target_dir` PathBuf itself is fine for fs operations, but for Command::arg,
    // we pass a string that git can reliably interpret.
    let git_target_path_arg_string;
    if cfg!(windows) {
        let path_str_cow = target_dir.to_string_lossy();
        if path_str_cow.starts_with("\\\\?\\") {
            // Create a new string without the prefix
            git_target_path_arg_string = path_str_cow.trim_start_matches("\\\\?\\").to_string();
            debug!("[CUSTOM_NODE_SETUP] Using cleaned path for git clone (Windows): {}", git_target_path_arg_string);
        } else {
            git_target_path_arg_string = path_str_cow.into_owned();
        }
    } else {
        // On non-Windows, or if no prefix, use the path as is.
        // Convert OsStr to String for Command::arg consistency if needed,
        // though Command::arg can often take &Path or &OsStr directly.
        // Here, to_string_lossy().into_owned() ensures a String.
        git_target_path_arg_string = target_dir.to_string_lossy().into_owned();
    }

    command.arg("clone").arg(IPADAPTER_PLUS_REPO_URL).arg(git_target_path_arg_string)
        .stdout(ProcessStdio::piped())
        .stderr(ProcessStdio::piped());

    let child = command.spawn().map_err(|e| {
        let err_msg = if e.kind() == std::io::ErrorKind::NotFound {
            "Git command not found. Please ensure Git is installed and in your system's PATH.".to_string()
        } else {
            format!("Failed to execute git clone command for {}: {}", IPADAPTER_PLUS_NODE_NAME, e)
        };
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, IPADAPTER_PLUS_NODE_NAME, &err_msg);
        err_msg
    })?;

    let output = child.wait_with_output().await.map_err(|e| {
        let err_msg = format!("Failed to wait for git clone command for {}: {}", IPADAPTER_PLUS_NODE_NAME, e);
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, IPADAPTER_PLUS_NODE_NAME, &err_msg);
        err_msg
    })?;

    if output.status.success() {
        info!("[CUSTOM_NODE_SETUP] Successfully cloned {}. Output: {}", IPADAPTER_PLUS_NODE_NAME, String::from_utf8_lossy(&output.stdout));
        emit_custom_node_clone_success(app_handle, IPADAPTER_PLUS_NODE_NAME);
        // After successful clone, proceed to install dependencies
        // Call install_insightface_dependencies without the comfyui_base_path argument
        install_insightface_dependencies(app_handle).await
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let err_msg = format!(
            "Failed to clone {}. Git command exited with error. Status: {}. Stderr: {}. Stdout: {}",
            IPADAPTER_PLUS_NODE_NAME, output.status, stderr.trim(), stdout.trim()
        );
        error!("[CUSTOM_NODE_SETUP] {}", err_msg);
        emit_custom_node_clone_failed(app_handle, IPADAPTER_PLUS_NODE_NAME, &err_msg);
        Err(err_msg)
    }
}

/// Installs insightface and its dependencies (onnxruntime).
// Removed comfyui_base_path_arg as it's unused; comfyui_base_path is fetched internally.
pub async fn install_insightface_dependencies(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    let comfyui_base_path = get_comfyui_directory_path(app_handle)?;
    info!("[INSIGHTFACE_SETUP] Starting insightface dependency installation in {}...", comfyui_base_path.display());
    
    // Use the utility function to get the venv python executable
    let python_executable = get_venv_python_executable_path(app_handle)?;

    if !python_executable.exists() {
        let err_msg = format!("Python executable not found at {}", python_executable.display());
        error!("{}", err_msg);
        emit_event(app_handle, "PackageInstallFailed", Some(json!({"packageName": "insightface_dependencies", "error": err_msg.clone() })));
        return Err(err_msg);
    }
    info!("Using Python executable: {}", python_executable.display());

    // 1. Update pip
    run_pip_command(app_handle, &python_executable, &["install", "-U", "pip"], "pip", "pip").await?;

    // 2. Check and install ONNX Runtime
    let onnx_script_content = "import onnxruntime; print(onnxruntime.__version__)";
    let is_onnx_installed = execute_python_script_check(&python_executable, onnx_script_content, CHECK_ONNX_SCRIPT_NAME, &comfyui_base_path).await?;
    if is_onnx_installed {
        info!("[INSIGHTFACE_SETUP] {} already installed.", ONNXRUNTIME_PACKAGE);
        emit_event(app_handle, "PackageAlreadyInstalled", Some(json!({ "packageName": ONNXRUNTIME_PACKAGE })));
    } else {
        info!("[INSIGHTFACE_SETUP] Installing {}...", ONNXRUNTIME_PACKAGE);
        run_pip_command(app_handle, &python_executable, &["install", "onnxruntime"], ONNXRUNTIME_PACKAGE, "pip").await?;
    }

    // 3. Check and install Insightface
    let insightface_script_content = "import insightface; print(insightface.__version__)";
    let is_insightface_installed = execute_python_script_check(&python_executable, insightface_script_content, CHECK_INSIGHTFACE_SCRIPT_NAME, &comfyui_base_path).await?;
    if is_insightface_installed {
        info!("[INSIGHTFACE_SETUP] {} already installed.", INSIGHTFACE_PACKAGE);
        emit_event(app_handle, "PackageAlreadyInstalled", Some(json!({ "packageName": INSIGHTFACE_PACKAGE })));
        return Ok(());
    }

    info!("[INSIGHTFACE_SETUP] Installing {}...", INSIGHTFACE_PACKAGE);
    let os_type = env::consts::OS;
    match os_type {
        "windows" => {
            let py_version = get_python_version(app_handle, &python_executable).await.map_err(|e| {
                let err_msg = format!("Failed to get Python version for Windows wheel selection: {}", e);
                error!("{}", err_msg);
                // get_python_version emits PythonVersionDetected on success,
                // so we only need to emit PackageInstallFailed here.
                emit_event(app_handle, "PackageInstallFailed", Some(json!({"packageName": INSIGHTFACE_PACKAGE, "error": err_msg.clone(), "detail": "Python version detection failed."})));
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
                    emit_event(app_handle, "PackageInstallFailed", Some(json!({"packageName": INSIGHTFACE_PACKAGE, "error": err_msg.clone()})));
                    return Err(err_msg);
                }
            };
            let wheel_name = Path::new(wheel_url).file_name().unwrap_or_default().to_str().unwrap_or("insightface_wheel.whl");
            
            let temp_dir = comfyui_base_path.join("temp_downloads"); // Or use app_handle.path().app_cache_dir()
            fs::create_dir_all(&temp_dir).await.map_err(|e| format!("Failed to create temp dir for wheel: {}", e))?;

            let wheel_path = download_file(wheel_url, &temp_dir, wheel_name, app_handle).await?;
            run_pip_command(app_handle, &python_executable, &["install", wheel_path.to_str().unwrap()], INSIGHTFACE_PACKAGE, "wheel").await?;
            fs::remove_file(&wheel_path).await.map_err(|e| format!("Failed to remove temporary wheel file {}: {}", wheel_path.display(), e))?;
            info!("[INSIGHTFACE_SETUP] Successfully installed {} from wheel.", INSIGHTFACE_PACKAGE);
        }
        "macos" | "linux" => {
            let install_result = run_pip_command(app_handle, &python_executable, &["install", "-U", "insightface"], INSIGHTFACE_PACKAGE, "pip").await;
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
                    app_handle,
                    "PackageInstallFailed",
                    Some(json!({ "packageName": INSIGHTFACE_PACKAGE, "error": e, "osHint": os_hint })),
                );
                return Err(err_msg);
            }
        }
        _ => {
            let err_msg = format!("Unsupported OS for Insightface installation: {}", os_type);
            error!("{}", err_msg);
            emit_event(app_handle, "PackageInstallFailed", Some(json!({"packageName": INSIGHTFACE_PACKAGE, "error": err_msg.clone()})));
            return Err(err_msg);
        }
    }

    info!("[INSIGHTFACE_SETUP] Insightface dependency installation completed.");
    Ok(())
}