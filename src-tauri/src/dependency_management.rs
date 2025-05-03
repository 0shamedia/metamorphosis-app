use std::fs;
use std::io::{prelude::*, BufReader};
use std::env;
use std::path::PathBuf;
use log::{info, error};
use uuid::Uuid;
use scopeguard;
use crate::gpu_detection::{GpuInfo, GpuType, get_gpu_info}; // Import from the gpu_detection module
use tauri::{AppHandle, Manager, Wry, Emitter}; // Import AppHandle, Manager, Wry, and Emitter
use serde::Serialize;
use std::process::{Command, Stdio};
use std::thread;
use fs2::available_space; // Import available_space
use tauri::path::BaseDirectory; // Import BaseDirectory

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum InstallationStep {
    CheckingDependencies,
    CreatingVirtualEnvironment,
    InstallingDependencies,
    InstallingTorch,
    InstallationComplete,
    Error,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstallationStatus {
    step: InstallationStep,
    message: String,
    is_error: bool,
}

// Function to emit installation status events
fn emit_status(app_handle: &AppHandle<Wry>, step: InstallationStep, message: String, is_error: bool) {
    let status = InstallationStatus {
        step,
        message,
        is_error,
    };
    if let Err(e) = app_handle.emit("installation-status", status) {
        error!("Failed to emit installation status event: {}", e);
    }
}

// This function executes a command and streams its stdout and stderr,
// logging each line with 'info!' for stdout and 'error!' for stderr.
// The command itself is logged before execution.
// Helper function to run a command and stream output
fn run_command_with_progress(
    app_handle: &AppHandle<Wry>,
    step: InstallationStep,
    command_path: &PathBuf,
    args: &[&str],
    current_dir: &PathBuf,
    initial_message: &str,
    success_message: &str,
    error_message_prefix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Executing command: {:?} {:?}", command_path, args);
    emit_status(app_handle, step.clone(), initial_message.to_string(), false);

    let mut child = Command::new(command_path)
        .current_dir(current_dir)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().ok_or(format!("{} - Failed to capture stdout", error_message_prefix))?;
    let stderr = child.stderr.take().ok_or(format!("{} - Failed to capture stderr", error_message_prefix))?;

    let app_handle_clone_stdout = app_handle.clone();
    let step_clone_stdout = step.clone();
    let stdout_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    info!("Stdout: {}", line);
                    emit_status(&app_handle_clone_stdout, step_clone_stdout.clone(), line, false);
                }
                Err(e) => {
                    error!("Error reading stdout: {}", e);
                    emit_status(&app_handle_clone_stdout, InstallationStep::Error, format!("Error reading stdout: {}", e), true);
                }
            }
        }
    });

    let app_handle_clone_stderr = app_handle.clone();
    let step_clone_stderr = step.clone();
    let stderr_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    error!("Stderr: {}", line);
                    // Treat stderr lines as progress updates but mark them as errors for potential UI highlighting
                    emit_status(&app_handle_clone_stderr, step_clone_stderr.clone(), line, true);
                }
                Err(e) => {
                    error!("Error reading stderr: {}", e);
                    emit_status(&app_handle_clone_stderr, InstallationStep::Error, format!("Error reading stderr: {}", e), true);
                }
            }
        }
    });

    let status = child.wait()?;

    stdout_thread.join().map_err(|e| format!("Stdout thread panicked: {:?}", e))?;
    stderr_thread.join().map_err(|e| format!("Stderr thread panicked: {:?}", e))?;

    if !status.success() {
        let error_msg = format!("{} failed with status: {:?}", error_message_prefix, status);
        error!("{}", error_msg);
        emit_status(app_handle, InstallationStep::Error, error_msg.clone(), true);
        return Err(error_msg.into());
    }

    info!("{}", success_message);
    emit_status(app_handle, step, success_message.to_string(), false);
    Ok(())
}


// Function to install Python dependencies
// Estimated required disk space for ComfyUI dependencies (20 GB)
const REQUIRED_DISK_SPACE: u64 = 20 * 1024 * 1024 * 1024; // in bytes

pub fn install_python_dependencies(app_handle: &AppHandle<Wry>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Checking if Python dependencies are installed...");
    emit_status(app_handle, InstallationStep::CheckingDependencies, "Checking if dependencies are already installed...".into(), false);

    // Get the path to the bundled ComfyUI directory (where dependencies will be installed)
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
    // Determine the ComfyUI directory based on whether we are in development or bundled mode
    let comfyui_dir = if std::env::var("TAURI_DEV").is_ok() {
        // In development mode, derive the path from CARGO_MANIFEST_DIR (src-tauri)
        // to the workspace root and then to target/debug/vendor/comfyui.
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("debug").join("vendor").join("comfyui")
    } else {
        // In bundled mode, the vendor directory is relative to the executable.
        exe_dir.join("vendor/comfyui")
    };

    // Check available disk space
    match available_space(&comfyui_dir) {
        Ok(available) => {
            info!("Available disk space at {}: {} bytes", comfyui_dir.display(), available);
            if available < REQUIRED_DISK_SPACE {
                let required_gb = REQUIRED_DISK_SPACE as f64 / (1024.0 * 1024.0 * 1024.0);
                let available_gb = available as f64 / (1024.0 * 1024.0 * 1024.0);
                let error_msg = format!(
                    "Insufficient disk space. Required: {:.2} GB, Available: {:.2} GB. Please free up space and try again.",
                    required_gb, available_gb
                );
                error!("{}", error_msg);
                emit_status(app_handle, InstallationStep::Error, error_msg.clone(), true);
                return Err(error_msg.into());
            }
            info!("Sufficient disk space available.");
        }
        Err(e) => {
            let error_msg = format!("Failed to check disk space at {}: {}", comfyui_dir.display(), e);
            error!("{}", error_msg);
            emit_status(app_handle, InstallationStep::Error, error_msg.clone(), true);
            return Err(error_msg.into());
        }
    }

    // Define the path for the dependency installed marker file
    let app_data_dir = app_handle.path().app_data_dir()?;
    let marker_file_path = app_data_dir.join("dependencies_installed_marker");

    // Define the path for the dependency installed marker file
    let app_data_dir = app_handle.path().app_data_dir()?;
    let marker_file_path = app_data_dir.join("dependencies_installed_marker");

    // Check if dependencies are already installed
    if marker_file_path.exists() {
        info!("Python dependencies already installed (marker file found).");
        emit_status(app_handle, InstallationStep::InstallationComplete, "Dependencies already installed.".into(), false);
        return Ok(());
    }

    info!("Dependencies not installed. Starting installation process...");
    emit_status(app_handle, InstallationStep::CheckingDependencies, "Dependencies not installed. Starting installation process...".into(), false);


    // Get the path to the bundled ComfyUI directory
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
    // Get the path to the bundled ComfyUI directory (where dependencies will be installed)
    // Determine the ComfyUI directory based on whether we are in development or bundled mode
    let comfyui_dir = if std::env::var("TAURI_DEV").is_ok() {
        // In development mode, derive the path from CARGO_MANIFEST_DIR (src-tauri)
        // to the workspace root and then to target/debug/vendor/comfyui.
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or("Failed to get workspace root")?
            .to_path_buf(); // Convert the &Path to PathBuf
        workspace_root.join("target").join("debug").join("vendor").join("comfyui")
    } else {
        // In bundled mode, the vendor directory is relative to the executable.
        exe_dir.join("vendor/comfyui")
    };

    // Resolve the path to requirements.txt based on build profile
    let requirements_path = if cfg!(debug_assertions) {
        // In development mode, derive the path from CARGO_MANIFEST_DIR (src-tauri)
        // to the workspace root and then to target/debug/vendor/comfyui/requirements.txt.
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("debug").join("vendor").join("comfyui").join("requirements.txt");
        info!("DEBUG: Development mode detected. Resolved requirements.txt path: {}", dev_path.display());
        dev_path
    } else {
        // In release mode, requirements.txt is bundled as a resource.
        let release_path = app_handle.path().resolve("vendor/comfyui/requirements.txt", BaseDirectory::Resource)
            .map_err(|e| {
                let error_msg = format!("Failed to resolve path to vendor/comfyui/requirements.txt in release mode: {}. Ensure it's included in tauri.conf.json resources.", e);
                error!("{}", error_msg);
                emit_status(app_handle, InstallationStep::Error, error_msg.clone(), true);
                error_msg
            })?;
        info!("DEBUG: Release mode detected. Resolved requirements.txt path: {}", release_path.display());
        release_path
    };

    match std::env::current_dir() {
        Ok(cwd) => info!("DEBUG: App CWD: {}", cwd.display()),
        Err(e) => error!("DEBUG: Failed to get App CWD: {}", e),
    }
    info!("DEBUG: App checking for requirements.txt at: {}", requirements_path.display());

    if !requirements_path.exists() {
        let error_msg = format!("ComfyUI requirements.txt not found at resolved path: {}", requirements_path.display());
        error!("{}", error_msg);
        emit_status(app_handle, InstallationStep::Error, error_msg.clone(), true);
        return Err(error_msg.into());
    }

    info!("Reading dependencies from: {}", requirements_path.display());
    let requirements_content = fs::read_to_string(&requirements_path)?;

    let python_executable = if std::env::var("TAURI_DEV").is_ok() {
        // In development mode, derive the path from CARGO_MANIFEST_DIR (src-tauri)
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("debug").join("vendor").join("python").join("python.exe")
    } else {
        // In bundled mode, the vendor directory is relative to the executable.
        exe_dir.join("vendor/python/python.exe")
    };
    info!("DEBUG: Resolved Python executable path: {:?}", python_executable);

    if !python_executable.exists() {
        let error_msg = format!("Python executable not found at: {}", python_executable.display());
        error!("{}", error_msg);
        emit_status(app_handle, InstallationStep::Error, error_msg.clone(), true);
        return Err(error_msg.into());
    }

    let venv_dir = comfyui_dir.join(".venv");
    let venv_python_executable = if cfg!(target_os = "windows") {
        venv_dir.join("Scripts").join("python.exe")
    } else {
        venv_dir.join("bin").join("python")
    };

    // 1. Create a virtual environment if it doesn't exist
    // Ensure a clean virtual environment by removing it if it exists
    if venv_dir.exists() {
        info!("Removing existing virtual environment at: {}", venv_dir.display());
        match fs::remove_dir_all(&venv_dir) {
            Ok(_) => info!("Successfully removed existing virtual environment."),
            Err(e) => error!("Failed to remove existing virtual environment: {}. Proceeding with creation, but this might cause issues.", e),
        }
    }

    // 1. Create a virtual environment
    info!("Creating virtual environment at: {}", venv_dir.display());
    run_command_with_progress(
        app_handle,
        InstallationStep::CreatingVirtualEnvironment,
        &python_executable,
        &["-m", "venv", venv_dir.to_str().unwrap()],
        &comfyui_dir,
        &format!("Creating virtual environment at: {}", venv_dir.display()),
        "Virtual environment created successfully.",
        "Failed to create virtual environment",
    )?;

    // 2. Install dependencies into the virtual environment
    info!("Installing Python dependencies into virtual environment...");
    emit_status(app_handle, InstallationStep::InstallingDependencies, "Preparing to install Python dependencies...".into(), false);


    // Get detailed GPU information
    let gpu_info = crate::gpu_detection::get_gpu_info();
    info!("Detected GPU Info: {:?}", gpu_info);

    // Define the Python script content
    let python_script_content = r#"
import csv
import sys
import subprocess

# Set the CSV field size limit to handle potentially large fields in pip output
# Using a large integer value to avoid potential OverflowError with sys.maxsize
csv.field_size_limit(2147483647)

# Execute the pip command with the received arguments
# sys.argv[1:] contains the arguments passed after the script name
# We need to exclude the first argument which is the script name itself
try:
    subprocess.run([sys.executable, "-m", "pip"] + sys.argv[1:], check=True, capture_output=False, text=True)
except subprocess.CalledProcessError as e:
    print(f"Pip command failed with error: {e}", file=sys.stderr)
    sys.exit(e.returncode)
except Exception as e:
    print(f"An unexpected error occurred: {e}", file=sys.stderr)
    sys.exit(1)
"#;

    // Generate a unique temporary file path
    let temp_dir = env::temp_dir();
    let script_filename = format!("install_pip_{}.py", Uuid::new_v4());
    let temp_script_path = temp_dir.join(script_filename);

    // Write the Python script to the temporary file
    if let Err(e) = write_temp_python_script(python_script_content, &temp_script_path) {
        let error_msg = format!("Failed to write temporary Python script: {}", e);
        error!("{}", error_msg);
        emit_status(app_handle, InstallationStep::Error, error_msg.clone(), true);
        return Err(error_msg.into());
    }

    // Ensure the temporary file is cleaned up later
    let temp_script_path_clone = temp_script_path.clone();
    let _cleanup = scopeguard::guard(temp_script_path_clone, |path| {
        if let Err(e) = fs::remove_file(&path) {
            error!("Failed to delete temporary Python script {}: {}", path.display(), e);
        } else {
            info!("Successfully deleted temporary Python script: {}", path.display());
        }
    });


    // 2a. Install non-torch dependencies from requirements.txt using default index
    let non_torch_dependencies: Vec<String> = requirements_content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#') &&
            trimmed != "torch" &&
            trimmed != "torchvision" &&
            trimmed != "torchaudio"
        })
        .map(|line| line.trim().to_string())
        .collect();

    if !non_torch_dependencies.is_empty() {
        let mut pip_args_non_torch: Vec<String> = vec![
            "install".to_string(),
            "--no-cache-dir".to_string(),
            "--verbose".to_string(), // Keep verbose for detailed output
            "--upgrade".to_string(),
            // "--force-reinstall".to_string(), // Consider removing force-reinstall unless necessary
        ];
        pip_args_non_torch.extend(non_torch_dependencies);

        let mut script_args_non_torch: Vec<String> = vec![temp_script_path.to_string_lossy().into_owned()];
        script_args_non_torch.extend(pip_args_non_torch);

        let script_args_non_torch_refs: Vec<&str> = script_args_non_torch.iter().map(|s| s.as_str()).collect();

        // The command and its output are logged by the run_command_with_progress function.
        run_command_with_progress(
            app_handle,
            InstallationStep::InstallingDependencies,
            &venv_python_executable,
            &script_args_non_torch_refs,
            &comfyui_dir.parent().ok_or("Failed to get parent directory of comfyui_dir")?.to_path_buf(),
            &format!("Installing non-torch dependencies: {:?}", script_args_non_torch_refs),
            "Successfully installed non-torch Python dependencies.",
            "Pip install (non-torch)",
        )?;

    } else {
        info!("No non-torch dependencies to install.");
        emit_status(app_handle, InstallationStep::InstallingDependencies, "No non-torch dependencies to install.".into(), false);
    }

    // 2b. Install torch, torchvision, and torchaudio based on GPU info
    let mut pip_args_torch: Vec<String> = vec![
        "install".to_string(),
        "--no-cache-dir".to_string(),
        "--verbose".to_string(), // Keep verbose for detailed output
        "--upgrade".to_string(),
        // "--force-reinstall".to_string(), // Consider removing force-reinstall unless necessary
    ];

    let mut torch_packages = vec![
        "torch".to_string(),
        "torchvision".to_string(),
        "torchaudio".to_string(),
    ];

    let mut index_url: Option<String> = None;

    match gpu_info.gpu_type {
        GpuType::Nvidia => {
            if let Some(cuda_version) = gpu_info.cuda_version {
                let major_minor = cuda_version.split('.').take(2).collect::<Vec<&str>>().join("");
                let url = format!("https://download.pytorch.org/whl/cu{}", major_minor);
                info!("Using CUDA index URL: {}", url);
                index_url = Some(url);
            } else {
                info!("NVIDIA GPU detected but CUDA version unknown. Installing CPU-only torch.");
                index_url = Some("https://download.pytorch.org/whl/cpu".to_string());
            }
        },
        GpuType::Amd => {
            #[cfg(target_os = "linux")]
            {
                info!("Detected AMD GPU on Linux. Installing ROCm-enabled torch.");
                // Example ROCm version, might need adjustment based on detection or user config
                index_url = Some("https://download.pytorch.org/whl/rocm5.7".to_string());
            }
            #[cfg(not(target_os = "linux"))]
            {
                info!("Detected AMD GPU on non-Linux. Installing CPU-only torch.");
                index_url = Some("https://download.pytorch.org/whl/cpu".to_string());
            }
        },
        GpuType::Intel => {
            info!("Detected Intel GPU. Installing Intel-optimized torch (CPU).");
            torch_packages.push("intel-extension-for-pytorch".to_string());
            index_url = Some("https://download.pytorch.org/whl/cpu".to_string());
        },
        _ => {
            info!("Detected Other or Unknown GPU type. Installing CPU-only torch.");
            index_url = Some("https://download.pytorch.org/whl/cpu".to_string());
        }
    }

    pip_args_torch.extend(torch_packages);
    if let Some(url) = index_url {
        pip_args_torch.extend(vec!["--index-url".to_string(), url]);
    } else {
         // Fallback to default PyPI if no specific index URL was determined
         info!("No specific index URL determined, using default PyPI.");
         pip_args_torch.extend(vec!["--index-url".to_string(), "https://pypi.org/simple".to_string()]);
    }


    let mut script_args_torch: Vec<String> = vec![temp_script_path.to_string_lossy().into_owned()];
    script_args_torch.extend(pip_args_torch);

    let script_args_torch_refs: Vec<&str> = script_args_torch.iter().map(|s| s.as_str()).collect();

    // The command and its output are logged by the run_command_with_progress function.
    run_command_with_progress(
        app_handle,
        InstallationStep::InstallingTorch,
        &venv_python_executable,
        &script_args_torch_refs,
        &comfyui_dir.parent().ok_or("Failed to get parent directory of comfyui_dir")?.to_path_buf(),
        &format!("Installing torch dependencies: {:?}", script_args_torch_refs),
        "Successfully installed torch Python dependencies.",
        "Pip install (torch)",
    )?;


    // Set a flag indicating dependencies are installed by creating a marker file
    info!("Creating dependency installed marker file: {}", marker_file_path.display());
    // Ensure the parent directory exists
    if let Some(parent) = marker_file_path.parent() {
        if !parent.exists() {
            info!("Creating parent directory for marker file: {}", parent.display());
            if let Err(e) = fs::create_dir_all(parent) {
                let error_msg = format!("Failed to create parent directory for marker file: {}", e);
                error!("{}", error_msg);
                emit_status(app_handle, InstallationStep::Error, error_msg.clone(), true);
                return Err(error_msg.into());
            }
            info!("Parent directory created successfully.");
        }
    }
    if let Err(e) = fs::File::create(&marker_file_path) {
        let error_msg = format!("Failed to create dependency marker file: {}", e);
        error!("{}", error_msg);
        emit_status(app_handle, InstallationStep::Error, error_msg.clone(), true);
        return Err(error_msg.into());
    }
    info!("Dependency marker file created successfully.");

    info!("Python dependency installation complete.");
    emit_status(app_handle, InstallationStep::InstallationComplete, "Python dependency installation complete.".into(), false);

    Ok(())
}

// Function to write the temporary Python script
pub fn write_temp_python_script(content: &str, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    info!("Writing temporary Python script to: {}", file_path.display());
    let mut file = fs::File::create(file_path)?;
    file.write_all(content.as_bytes())?;
    info!("Temporary Python script written successfully.");
    Ok(())
}