use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use log::{info, error, debug};
use crate::setup_manager::event_utils::emit_event;
use tauri::{AppHandle, Wry, path::BaseDirectory, Manager}; // Added BaseDirectory and Manager
use serde_json::json;
use std::env; // Added for env! macro

// Helper function to get the application's base resource directory path.
// In release mode, this is where bundled assets (like 'vendor') are.
// In debug mode, we construct a path relative to the manifest dir to point to `target/debug/`.
fn get_base_resource_path(app_handle: &AppHandle<Wry>) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| "Failed to get parent of CARGO_MANIFEST_DIR".to_string())?
            .join("target")
            .join("debug")
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize debug base resource path: {}", e))
    } else {
        app_handle.path().resource_dir()
            .map_err(|e| format!("Tauri error getting resource directory: {}", e)) // Changed ok_or_else to map_err
            .and_then(|p| p.canonicalize().map_err(|e| format!("Failed to canonicalize release resource path: {}", e)))
    }
}

/// Returns the absolute path to the 'vendor' directory.
pub fn get_vendor_path(app_handle: &AppHandle<Wry>) -> Result<PathBuf, String> {
    get_base_resource_path(app_handle)?
        .join("vendor")
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize vendor path: {}", e))
}

/// Returns the absolute path to the ComfyUI directory within the 'vendor' directory.
pub fn get_comfyui_directory_path(app_handle: &AppHandle<Wry>) -> Result<PathBuf, String> {
    get_vendor_path(app_handle)?
        .join("comfyui")
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize comfyui directory path: {}", e))
}

/// Returns the absolute path to the bundled Python executable.
pub fn get_bundled_python_executable_path(app_handle: &AppHandle<Wry>) -> Result<PathBuf, String> {
    get_vendor_path(app_handle)?
        .join("python")
        .join(if cfg!(windows) { "python.exe" } else { "python" })
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize bundled python executable path: {}", e))
}

/// Returns the absolute path to the Python executable within the ComfyUI virtual environment.
pub fn get_venv_python_executable_path(app_handle: &AppHandle<Wry>) -> Result<PathBuf, String> {
    let comfyui_dir = get_comfyui_directory_path(app_handle)?;
    let venv_dir = comfyui_dir.join(".venv");
    let python_exe_name = if cfg!(windows) { "python.exe" } else { "python" };
    let script_folder = if cfg!(windows) { "Scripts" } else { "bin" };

    Ok(venv_dir.join(script_folder).join(python_exe_name))
}

/// Returns the absolute path to a script within the 'scripts' directory (bundled or in src-tauri/scripts for debug).
pub fn get_script_path(app_handle: &AppHandle<Wry>, script_name: &str) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scripts")
            .join(script_name)
            .canonicalize()
            .map_err(|e| format!("Failed to find script '{}' at {:?} in debug mode: {}", script_name, PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts"), e))
    } else {
        app_handle
            .path()
            .resolve(&format!("scripts/{}", script_name), BaseDirectory::Resource)
            .map_err(|e| format!("Failed to resolve script '{}' with BaseDirectory::Resource: {}", script_name, e))
            // .and_then(|p| p.canonicalize().map_err(|e| format!("Failed to canonicalize script path '{}' in release mode: {}", script_name, e))) // Canonicalize might fail if path is inside archive before extraction
    }
}


/// Executes a command and returns its stdout.
pub async fn execute_command_to_string(
    command_path: &Path,
    args: &[&str],
    working_dir: Option<&Path>,
) -> Result<String, String> {
    debug!(
        "Executing command: {} with args: {:?} in working_dir: {:?}",
        command_path.display(),
        args,
        working_dir
    );
    let mut cmd = Command::new(command_path);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    let child = cmd.spawn().map_err(|e| {
        error!("Failed to spawn command {}: {}", command_path.display(), e);
        format!("Failed to spawn command {}: {}", command_path.display(), e)
    })?;

    let output = child.wait_with_output().await.map_err(|e| {
        error!("Failed to wait for command {}: {}", command_path.display(), e);
        format!("Failed to wait for command {}: {}", command_path.display(), e)
    })?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        debug!("Command {} successful. Output: {}", command_path.display(), stdout);
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        error!(
            "Command {} failed with status: {:?}. Stderr: {}",
            command_path.display(),
            output.status.code(),
            stderr
        );
        Err(format!(
            "Command {} failed. Stderr: {}",
            command_path.display(),
            stderr
        ))
    }
}

/// Gets the Python minor version (e.g., "3.10", "3.11").
pub async fn get_python_version(app_handle: &AppHandle<Wry>, python_executable: &PathBuf) -> Result<String, String> {
    info!("Detecting Python version for: {}", python_executable.display());
    let output = execute_command_to_string(python_executable, &["-V"], None).await?;

    // Output is typically "Python 3.X.Y"
    if let Some(version_part) = output.split_whitespace().nth(1) {
        // Version part is typically "3.X.Y" or "3.X.Yrc1" etc.
        // We want to extract "3.X"
        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() >= 2 {
            let major = parts[0];
            let minor = parts[1];
            // Ensure major looks like a number (e.g. "3") and minor looks like a number (e.g. "10")
            if major.chars().all(char::is_numeric) && minor.chars().all(char::is_numeric) {
                let minor_version = format!("{}.{}", major, minor);
                info!("Detected Python version: {}", minor_version);
                emit_event(app_handle, "PythonVersionDetected", Some(json!({ "version": minor_version })));
                return Ok(minor_version);
            }
        }
    }
    Err(format!("Could not parse Python version from output: {}", output))
}

/// Downloads a file from a URL to a temporary directory.
pub async fn download_file(
    url: &str,
    temp_dir: &PathBuf,
    dest_name: &str,
    app_handle: &AppHandle<Wry>, // Pass AppHandle for events
) -> Result<PathBuf, String> {
    info!("Downloading file from {} to {}/{}", url, temp_dir.display(), dest_name);
    emit_event(app_handle, "InsightfaceWheelDownloadStart", Some(json!({ "url": url.to_string() })));

    if !temp_dir.exists() {
        tokio::fs::create_dir_all(temp_dir).await.map_err(|e| {
            error!("Failed to create temp directory {}: {}", temp_dir.display(), e);
            format!("Failed to create temp directory {}: {}", temp_dir.display(), e)
        })?;
    }
    let dest_path = temp_dir.join(dest_name);

    let response = reqwest::get(url).await.map_err(|e| {
        let err_msg = format!("Failed to request file from {}: {}", url, e);
        error!("{}", err_msg);
        emit_event(app_handle, "PackageInstallFailed", Some(json!({ "packageName": "insightface_wheel", "error": err_msg.clone(), "osHint": serde_json::Value::Null })));
        err_msg
    })?;

    if !response.status().is_success() {
        let err_msg = format!("Download failed: {} status for URL {}", response.status(), url);
        error!("{}", err_msg);
        emit_event(app_handle, "PackageInstallFailed", Some(json!({ "packageName": "insightface_wheel", "error": err_msg.clone(), "osHint": serde_json::Value::Null })));
        return Err(err_msg);
    }

    let total_size = response.content_length();
    let mut file = File::create(&dest_path).await.map_err(|e| {
        let err_msg = format!("Failed to create file {}: {}", dest_path.display(), e);
        error!("{}", err_msg);
        emit_event(app_handle, "PackageInstallFailed", Some(json!({ "packageName": "insightface_wheel", "error": err_msg.clone(), "osHint": serde_json::Value::Null })));
        err_msg
    })?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(item) = futures_util::StreamExt::next(&mut stream).await {
        let chunk = item.map_err(|e| {
            let err_msg = format!("Error while downloading file chunk from {}: {}", url, e);
            error!("{}", err_msg);
            emit_event(app_handle, "PackageInstallFailed", Some(json!({ "packageName": "insightface_wheel", "error": err_msg.clone(), "osHint": serde_json::Value::Null })));
            err_msg
        })?;
        file.write_all(&chunk).await.map_err(|e| {
            let err_msg = format!("Error writing chunk to file {}: {}", dest_path.display(), e);
            error!("{}", err_msg);
            emit_event(app_handle, "PackageInstallFailed", Some(json!({ "packageName": "insightface_wheel", "error": err_msg.clone(), "osHint": serde_json::Value::Null })));
            err_msg
        })?;
        downloaded += chunk.len() as u64;
        if let Some(total) = total_size {
             debug!("Downloaded {} / {} bytes ({:.2}%)", downloaded, total, (downloaded as f64 / total as f64) * 100.0);
            emit_event(app_handle, "InsightfaceWheelDownloadProgress", Some(json!({ "downloaded": downloaded, "total": total })));
        } else {
            debug!("Downloaded {} bytes (total size unknown)", downloaded);
            emit_event(app_handle, "InsightfaceWheelDownloadProgress", Some(json!({ "downloaded": downloaded, "total": serde_json::Value::Null })));
        }
    }
    info!("Successfully downloaded {} to {}", url, dest_path.display());
    emit_event(app_handle, "InsightfaceWheelDownloadComplete", None::<serde_json::Value>);
    Ok(dest_path)
}

// TODO: Add idempotency check functions (e.g., for checking if a package is already installed at a specific version)
// TODO: Add pip update function (e.g., `pip install --upgrade pip`)
// TODO: Consider moving ONNX Runtime and Insightface installation logic here if they become more generic Python package installations.