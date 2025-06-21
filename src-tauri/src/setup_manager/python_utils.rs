use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use log::{info, error, debug};
use crate::setup_manager::event_utils::emit_event;
use tauri::{AppHandle, Wry, Manager};
use serde_json::json;
use std::env; // Added for env! macro

// Helper function to get the application's base resource directory path.
// In release mode, this is where bundled assets (like 'vendor') are.
// In debug mode, we construct a path relative to the manifest dir to point to `target/debug/`.
fn get_base_resource_path(app_handle: &AppHandle<Wry>) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")) // Should be .../src-tauri
            .parent()                             // Should be .../metamorphosis-app
            .ok_or_else(|| "Failed to get parent of CARGO_MANIFEST_DIR for debug base path".to_string())?
            .join("target")                       // .../metamorphosis-app/target
            .join("debug")                        // .../metamorphosis-app/target/debug
        // Canonicalization is removed/deferred for debug paths as they should be concrete
        // and to avoid issues if build.rs output isn't immediately visible for canonicalization.
            .into_ok() // Convert PathBuf to Result<PathBuf, String>
    } else {
        // Release mode needs canonicalize because resource_dir() can be tricky (e.g. inside ASAR)
        app_handle.path().resource_dir()
            .map_err(|e| format!("Tauri error getting resource directory: {}", e))
            .and_then(|p| p.canonicalize().map_err(|e| format!("Failed to canonicalize release resource path: {}", e)))
    }
}

trait IntoOk<T, E> {
    fn into_ok(self) -> Result<T, E>;
}

impl<T, E: Default> IntoOk<T, E> for T {
    fn into_ok(self) -> Result<T, E> {
        Ok(self)
    }
}

/// Returns the absolute path to the 'vendor' directory.
pub fn get_vendor_path(app_handle: &AppHandle<Wry>) -> Result<PathBuf, String> {
    let base_path = get_base_resource_path(app_handle)?;
    let vendor_path = base_path.join("vendor");

    if cfg!(debug_assertions) {
        // For debug, trust the path construction. Existence is verified by build.rs.
        // Canonicalization here can fail at runtime if the OS hasn't "settled" the path.
        Ok(vendor_path)
    } else {
        // For release, canonicalize as it's coming from resource_dir()
        vendor_path.canonicalize()
            .map_err(|e| format!("Failed to canonicalize release vendor path (from base {}): {}", base_path.display(), e))
    }
}

/// Returns the absolute path to the ComfyUI directory within the 'vendor' directory.
pub fn get_comfyui_directory_path(app_handle: &AppHandle<Wry>) -> Result<PathBuf, String> {
    let vendor_path = get_vendor_path(app_handle)?; // Will be non-canonicalized in debug
    let comfyui_path = vendor_path.join("comfyui");

    if cfg!(debug_assertions) {
        // For debug, trust the path construction.
        Ok(comfyui_path)
    } else {
        comfyui_path.canonicalize()
            .map_err(|e| format!("Failed to canonicalize release comfyui directory path (from vendor {}): {}", vendor_path.display(), e))
    }
}

/// Returns the absolute path to the bundled Python executable.
pub fn get_bundled_python_executable_path(app_handle: &AppHandle<Wry>) -> Result<PathBuf, String> {
    let vendor_path = get_vendor_path(app_handle)?; // Will be non-canonicalized in debug
    let python_exe_path = vendor_path
        .join("python")
        .join(if cfg!(windows) { "python.exe" } else { "python" });
    
    if cfg!(debug_assertions) {
        // For debug, trust the path construction.
        Ok(python_exe_path)
    } else {
        python_exe_path.canonicalize()
            .map_err(|e| format!("Failed to canonicalize release bundled python executable path: {}", e))
    }
}

/// Determines the absolute path to the `conda` executable.
/// Initially, it tries to find `conda` in the system's PATH.
/// In a future iteration, this will be updated to point to a bundled Miniconda installation.
pub async fn get_conda_executable_path(app_handle: &AppHandle<Wry>) -> Result<PathBuf, String> {
    // Get the application's root path
    let app_root_path = crate::setup_manager::orchestration::get_app_root_path()?;
    // Construct the expected Miniconda installation path relative to the app root
    let miniconda_install_path = app_root_path.join(crate::setup_manager::orchestration::MINICONDA_INSTALL_DIR_NAME);

    let conda_exe_path_in_scripts = miniconda_install_path.join("Scripts").join("conda.exe");
    let conda_exe_path_direct = miniconda_install_path.join("conda.exe");
    let conda_exe_path_in_bin = miniconda_install_path.join("bin").join("conda"); // For non-Windows

    if cfg!(windows) {
        if conda_exe_path_in_scripts.exists() {
            info!("Found conda executable at expected path (Scripts): {}", conda_exe_path_in_scripts.display());
            Ok(conda_exe_path_in_scripts)
        } else if conda_exe_path_direct.exists() {
            info!("Found conda executable at expected path (Direct): {}", conda_exe_path_direct.display());
            Ok(conda_exe_path_direct)
        } else {
            let err_msg = format!("Conda executable not found at expected paths: {} or {}", conda_exe_path_in_scripts.display(), conda_exe_path_direct.display());
            error!("{}", err_msg);
            Err(err_msg)
        }
    } else { // Linux/macOS
        if conda_exe_path_in_bin.exists() {
            info!("Found conda executable at expected path (bin): {}", conda_exe_path_in_bin.display());
            Ok(conda_exe_path_in_bin)
        } else {
            let err_msg = format!("Conda executable not found at expected path: {}", conda_exe_path_in_bin.display());
            error!("{}", err_msg);
            Err(err_msg)
        }
    }
}

/// Returns the absolute path to the Python executable within the specified Conda environment.
/// This function assumes that the `conda` executable is found and uses its location
/// to infer the Miniconda installation root and then the environment's Python path.
pub async fn get_conda_env_python_executable_path(app_handle: &AppHandle<Wry>, env_name: &str) -> Result<PathBuf, String> {
    let app_root_path = crate::setup_manager::orchestration::get_app_root_path()?;
    let miniconda_install_path = app_root_path.join(crate::setup_manager::orchestration::MINICONDA_INSTALL_DIR_NAME);

    let python_exe_name = if cfg!(windows) { "python.exe" } else { "python" };

    // On Windows, the python executable is in <miniconda_root>/envs/<env_name>/python.exe
    // On Linux/macOS, it's in <miniconda_root>/envs/<env_name>/bin/python
    let env_python_path = if cfg!(windows) {
        miniconda_install_path.join("envs").join(env_name).join(python_exe_name)
    } else {
        miniconda_install_path.join("envs").join(env_name).join("bin").join(python_exe_name)
    };

    info!("Determined Conda environment Python executable path: {}", env_python_path.display());
    Ok(env_python_path)
}

// The get_script_path function has been removed as verification scripts are now created dynamically.

/// Waits for a directory to exist at the given path, with a timeout.
pub async fn wait_for_directory_to_exist(
    app_handle: &AppHandle<Wry>,
    dir_path: &Path,
    timeout_secs: u64,
    check_interval_millis: u64,
    dir_description: &str,
) -> Result<(), String> {
    info!("Waiting for {} to exist at: {}", dir_description, dir_path.display());
    let start_time = tokio::time::Instant::now();

    loop {
        if dir_path.exists() && dir_path.is_dir() {
            info!("{} found at: {}", dir_description, dir_path.display());
            return Ok(());
        }

        if start_time.elapsed().as_secs() >= timeout_secs {
            let err_msg = format!(
                "Timeout waiting for {} to appear at: {}",
                dir_description,
                dir_path.display()
            );
            error!("{}", err_msg);
            emit_event(app_handle, "SetupError", Some(serde_json::json!({
                "message": err_msg.clone(),
                "detail": format!("Waited for {} seconds.", timeout_secs)
            })));
            return Err(err_msg);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_millis)).await;
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

    let client = reqwest::Client::builder()
        .user_agent("Metamorphosis-App/1.0")
        .build()
        .map_err(|e| format!("Failed to build reqwest client: {}", e))?;

    let response = client.get(url).send().await.map_err(|e| {
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
/// Waits for a file to exist at the given path, with a timeout.
pub async fn wait_for_file_to_exist(
    app_handle: &AppHandle<Wry>,
    file_path: &Path,
    timeout_secs: u64,
    check_interval_millis: u64,
    file_description: &str,
) -> Result<(), String> {
    info!("Waiting for {} to exist at: {}", file_description, file_path.display());
    let start_time = tokio::time::Instant::now();

    loop {
        if file_path.exists() {
            info!("{} found at: {}", file_description, file_path.display());
            return Ok(());
        }

        if start_time.elapsed().as_secs() >= timeout_secs {
            let err_msg = format!(
                "Timeout waiting for {} to appear at: {}",
                file_description,
                file_path.display()
            );
            error!("{}", err_msg);
            emit_event(app_handle, "SetupError", Some(serde_json::json!({
                "message": err_msg.clone(),
                "detail": format!("Waited for {} seconds.", timeout_secs)
            })));
            return Err(err_msg);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_millis)).await;
    }
}