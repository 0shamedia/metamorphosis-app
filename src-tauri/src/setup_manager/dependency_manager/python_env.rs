// metamorphosis-app/src-tauri/src/setup_manager/dependency_manager/python_env.rs
use std::fs;
use log::{info, error, warn};
use tauri::{AppHandle, Wry};
use fs2::available_space; // For disk space check
use tokio::time::{sleep, Duration}; // Added for retry mechanism

use crate::gpu_detection::{GpuType, get_gpu_info};
use crate::setup; // For emit_setup_progress
use crate::setup_manager::python_utils::{
    get_comfyui_directory_path,
    get_conda_env_python_executable_path,
    wait_for_file_to_exist, // Added this import
};

// Import from sibling modules
use super::command_runner::run_command_for_setup_progress;
use crate::setup_manager::python_utils::execute_command_to_string;
use super::disk_utils::REQUIRED_DISK_SPACE;


// New function for SetupScreen with detailed progress
pub async fn install_python_dependencies_with_progress(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    let phase_name = "python_setup"; // Or "installing_comfyui" - needs consistency with SetupScreen
    let mut current_phase_progress: u8 = 0;

    info!("Checking disk space (with progress)...");
    setup::emit_setup_progress(app_handle, phase_name, "Checking available disk space...", current_phase_progress, None, None);
    
    let comfyui_dir_raw = get_comfyui_directory_path(app_handle)?;
    
    let comfyui_dir = comfyui_dir_raw.canonicalize().map_err(|e| {
        let err_msg = format!("Failed to canonicalize ComfyUI directory path {}: {}", comfyui_dir_raw.display(), e);
        error!("{}", err_msg);
        setup::emit_setup_progress(app_handle, "error", "ComfyUI Path Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
        err_msg
    })?;
    info!("Canonicalized ComfyUI directory path: {}", comfyui_dir.display());

    match available_space(&comfyui_dir) {
        Ok(available) => {
            info!("Available disk space at {}: {} bytes", comfyui_dir.display(), available);
            if available < REQUIRED_DISK_SPACE {
                let err_msg = format!("Insufficient disk space. Required: {:.2} GB, Available: {:.2} GB.", REQUIRED_DISK_SPACE as f64 / (1024.0 * 1024.0 * 1024.0), available as f64 / (1024.0 * 1024.0 * 1024.0));
                error!("{}", err_msg);
                setup::emit_setup_progress(app_handle, "error", "Disk Space Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
                return Err(err_msg);
            }
            current_phase_progress = 10; // e.g., 10% for disk space check
            setup::emit_setup_progress(app_handle, phase_name, "Sufficient disk space available.", current_phase_progress, None, None);
        }
        Err(e) => {
            let err_msg = format!("Failed to check disk space at {}: {}", comfyui_dir.display(), e);
            error!("{}", err_msg);
            setup::emit_setup_progress(app_handle, "error", "Disk Space Check Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
            return Err(err_msg);
        }
    }

    info!("Checking if Python dependencies are installed (with progress)...");
    current_phase_progress = 15; // Progress after disk check
    setup::emit_setup_progress(app_handle, phase_name, "Checking existing Python installation...", current_phase_progress, None, None);

    let internal_deps_marker_path = comfyui_dir.join(".conda_env_deps_installed.marker");
    let env_name = "comfyui_env";
    let conda_python_executable_path_result = crate::setup_manager::python_utils::get_conda_env_python_executable_path(app_handle, env_name).await;

    let comfyui_env_python_exists = if let Ok(ref path) = conda_python_executable_path_result {
        path.exists() && path.is_file()
    } else {
        false
    };

    if internal_deps_marker_path.exists() {
        if comfyui_env_python_exists {
            // Marker exists AND Python executable in environment exists, perform integrity check
            info!("Python dependencies marker found and Conda environment Python executable exists. Verifying integrity...");
            setup::emit_setup_progress(app_handle, phase_name, "Verifying Python environment integrity...", current_phase_progress, None, None);
            
            match crate::setup_manager::verification::check_python_environment_integrity(app_handle).await {
                Ok(true) => {
                    info!("Python environment integrity verified. Skipping installation.");
                    current_phase_progress = 100;
                    setup::emit_setup_progress(app_handle, phase_name, "Python environment already set up.", current_phase_progress, None, None);
                    return Ok(());
                },
                Ok(false) => {
                    // Integrity check failed, proceed with re-installation
                    warn!("Python environment integrity check failed. Removing marker and proceeding with re-installation.");
                    fs::remove_file(&internal_deps_marker_path).map_err(|e| format!("Failed to remove old internal marker after failed integrity check: {}", e))?;
                },
                Err(e) => {
                    // Error during integrity check, treat as failure and proceed with re-installation
                    error!("Error during Python environment integrity check: {}. Removing marker and proceeding with re-installation.", e);
                    fs::remove_file(&internal_deps_marker_path).map_err(|e| format!("Failed to remove old internal marker after error during integrity check: {}", e))?;
                }
            }
        } else {
            // Marker exists but Python executable in environment does NOT exist, marker is stale. Remove it.
            warn!("Python dependencies marker found but Conda environment Python executable does not exist. Removing stale marker: {}", internal_deps_marker_path.display());
            fs::remove_file(&internal_deps_marker_path).map_err(|e| format!("Failed to remove stale internal marker: {}", e))?;
        }
    }

    info!("Python dependencies need installation or verification. Starting process...");

    // Determine the path to the conda executable
    let conda_executable = crate::setup_manager::python_utils::get_conda_executable_path(app_handle).await?;

    info!("Conda executable path: {}", conda_executable.display());

    // Check if the conda environment already exists
    let env_name = "comfyui_env";
    let check_env_args = vec!["env", "list"];
    setup::emit_setup_progress(app_handle, phase_name, "Checking existing Conda environments", current_phase_progress, Some("Listing Conda environments...".to_string()), None);
    let env_list_output = execute_command_to_string(
        &conda_executable,
        &check_env_args,
        Some(&comfyui_dir),
    ).await?;
    current_phase_progress += 5; // Manually update progress
    setup::emit_setup_progress(app_handle, phase_name, "Conda environment list retrieved.", current_phase_progress, None, None);

    let env_exists = env_list_output.contains(&format!(" {}", env_name));

    if !env_exists {
        info!("Conda environment '{}' does not exist. Creating it...", env_name);
        let create_env_args = vec!["create", "-n", env_name, "python=3.10", "-y"];
        info!("Executing command: {} {}", conda_executable.display(), create_env_args.join(" "));
        current_phase_progress = run_command_for_setup_progress(
            app_handle, phase_name, &format!("Creating Conda environment '{}'", env_name), current_phase_progress, 15,
            &conda_executable, &create_env_args,
            &comfyui_dir,
            &format!("Starting creation of Conda environment '{}'...", env_name), &format!("Conda environment '{}' created.", env_name)
        ).await.map_err(|e| e.to_string())?;
    } else {
        info!("Conda environment '{}' already exists. Skipping creation.", env_name);
        current_phase_progress = 15; // Adjust progress to reflect skipping creation
        setup::emit_setup_progress(app_handle, phase_name, &format!("Conda environment '{}' already exists.", env_name), current_phase_progress, None, None);
    }

    // Now that the environment is created/confirmed, get the python executable path and wait for it
    let conda_python_executable = get_conda_env_python_executable_path(app_handle, env_name).await?;
    info!("Waiting for Conda environment Python executable to exist at: {}", conda_python_executable.display());
    wait_for_file_to_exist(
        app_handle,
        &conda_python_executable,
        120, // Timeout after 120 seconds
        1000, // Check every 1000 milliseconds
        "Conda environment Python executable",
    ).await?;
    info!("Conda environment Python executable found at: {}", conda_python_executable.display());

    let requirements_path = comfyui_dir.join("requirements.txt");
    info!("Attempting to access requirements.txt at: {}", requirements_path.display());
    if !requirements_path.exists() {
        let err_msg = format!("ComfyUI requirements.txt not found at {}", requirements_path.display());
        error!("{}", err_msg);
        setup::emit_setup_progress(app_handle, "error", "Requirements File Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
        return Err(err_msg);
    }

    info!("Attempting to install PyTorch, Torchvision, and Torchaudio via Conda...");
    let gpu_info_for_torch_step = get_gpu_info();
    let mut conda_torch_args: Vec<String> = vec![
        "install".to_string(),
        "-n".to_string(),
        env_name.to_string(),
        "pytorch".to_string(),
        "torchvision".to_string(),
        "torchaudio".to_string(),
    ];
    info!("Executing command: {} {}", conda_executable.display(), conda_torch_args.join(" "));

    if gpu_info_for_torch_step.gpu_type == GpuType::Nvidia {
        if let Some(cuda_ver_str_ref) = gpu_info_for_torch_step.cuda_version.as_deref() {
            if !cuda_ver_str_ref.is_empty() {
                let cuda_ver_str = cuda_ver_str_ref.to_string();
                info!("Detected NVIDIA GPU with CUDA version string: {}", cuda_ver_str);
                let cuda_package_suffix = if cuda_ver_str.starts_with("12.") {
                    "pytorch-cuda=12.4".to_string() // Align with the expected 12.4 DLL and 12.9 driver
                } else if cuda_ver_str.starts_with("11.") {
                    format!("pytorch-cuda={}", cuda_ver_str)
                } else {
                    error!("Unsupported NVIDIA CUDA version prefix: {}. Falling back to CPU PyTorch.", cuda_ver_str);
                    "cpuonly".to_string()
                };
                conda_torch_args.push(cuda_package_suffix);
                conda_torch_args.push("-c".to_string());
                conda_torch_args.push("pytorch".to_string());
                conda_torch_args.push("-c".to_string());
                conda_torch_args.push("nvidia".to_string());
            } else {
                error!("NVIDIA GPU detected, but CUDA version string is empty. Falling back to CPU PyTorch.");
                conda_torch_args.push("cpuonly".to_string());
                conda_torch_args.push("-c".to_string());
                conda_torch_args.push("pytorch".to_string());
            }
        } else {
            error!("NVIDIA GPU detected, but no CUDA version found. Falling back to CPU PyTorch.");
            conda_torch_args.push("cpuonly".to_string());
            conda_torch_args.push("-c".to_string());
            conda_torch_args.push("pytorch".to_string());
        }
    } else {
        info!("Non-NVIDIA GPU detected ({:?}). Using CPU PyTorch.", gpu_info_for_torch_step.gpu_type);
        conda_torch_args.push("cpuonly".to_string());
        conda_torch_args.push("-c".to_string());
        conda_torch_args.push("pytorch".to_string());
    }
    conda_torch_args.push("-y".to_string()); // Auto-approve

    let conda_torch_args_refs: Vec<&str> = conda_torch_args.iter().map(|s| s.as_str()).collect();
    
    let max_retries = 3;
    let mut attempt = 0;
    let mut torch_install_success = false;

    while attempt < max_retries {
        attempt += 1;
        info!("Attempt {} of {} to install PyTorch, Torchvision, Torchaudio...", attempt, max_retries);
        setup::emit_setup_progress(app_handle, phase_name, &format!("Attempt {} of {} to install PyTorch...", attempt, max_retries), current_phase_progress, None, None);

        match run_command_for_setup_progress(
            app_handle, phase_name, "Installing PyTorch, Torchvision, Torchaudio", current_phase_progress, 30,
            &conda_executable, &conda_torch_args_refs,
            &comfyui_dir,
            "Starting PyTorch installation...", "PyTorch, Torchvision, Torchaudio installed."
        ).await {
            Ok(progress) => {
                current_phase_progress = progress;
                torch_install_success = true;
                info!("PyTorch, Torchvision, Torchaudio installed successfully on attempt {}.", attempt);
                break;
            },
            Err(e) => {
                error!("PyTorch installation failed on attempt {}: {}", attempt, e);
                if attempt < max_retries {
                    let retry_msg = format!("Retrying PyTorch installation in 5 seconds (attempt {}/{})", attempt, max_retries);
                    warn!("{}", retry_msg);
                    setup::emit_setup_progress(app_handle, phase_name, &retry_msg, current_phase_progress, Some(e.to_string()), None);
                    sleep(Duration::from_secs(5)).await;
                } else {
                    let final_err_msg = format!("Failed to install PyTorch after {} attempts. Last error: {}", max_retries, e);
                    error!("{}", final_err_msg);
                    setup::emit_setup_progress(app_handle, "error", "PyTorch Installation Failed", current_phase_progress, Some(final_err_msg.clone()), Some(final_err_msg.clone()));
                    return Err(final_err_msg);
                }
            }
        }
    }

    if !torch_install_success {
        let err_msg = "PyTorch installation did not succeed after multiple attempts.".to_string();
        error!("{}", err_msg);
        return Err(err_msg);
    }

    info!("Attempting to install NumPy via Conda...");
    let conda_numpy_args: Vec<String> = vec![
        "install".to_string(),
        "-n".to_string(),
        env_name.to_string(),
        "numpy".to_string(),
        "-y".to_string(),
    ];
    let conda_numpy_args_refs: Vec<&str> = conda_numpy_args.iter().map(|s| s.as_str()).collect();
    info!("Executing command: {} {}", conda_executable.display(), conda_numpy_args.join(" "));
    current_phase_progress = run_command_for_setup_progress(
        app_handle, phase_name, "Installing NumPy", current_phase_progress, 5,
        &conda_executable, &conda_numpy_args_refs,
        &comfyui_dir,
        "Starting NumPy installation...", "NumPy installed."
    ).await.map_err(|e| e.to_string())?;

    // Install onnxruntime with appropriate acceleration provider
    info!("Attempting to install onnxruntime with appropriate acceleration provider via Conda...");
    info!("Attempting to install onnxruntime from conda-forge.");
    let onnxruntime_pkg = "onnxruntime"; // Install base onnxruntime, Conda will handle GPU variant if CUDA is available

    let conda_onnxruntime_args: Vec<String> = vec![
        "install".to_string(),
        "-n".to_string(),
        env_name.to_string(),
        onnxruntime_pkg.to_string(),
        "-c".to_string(),
        "conda-forge".to_string(),
        "-y".to_string(),
    ];
    let conda_onnxruntime_args_refs: Vec<&str> = conda_onnxruntime_args.iter().map(|s| s.as_str()).collect();
    info!("Executing command: {} {}", conda_executable.display(), conda_onnxruntime_args.join(" "));
    current_phase_progress = run_command_for_setup_progress(
        app_handle, phase_name, &format!("Installing {}", onnxruntime_pkg), current_phase_progress, 10,
        &conda_executable, &conda_onnxruntime_args_refs,
        &comfyui_dir,
        &format!("Starting {} installation...", onnxruntime_pkg), &format!("{} installed.", onnxruntime_pkg)
    ).await.map_err(|e| e.to_string())?;
    info!("Successfully installed {}.", onnxruntime_pkg);

    // Install cffi via Conda first to avoid wheel compatibility issues with pip
    info!("Attempting to install cffi via Conda...");
    let conda_cffi_args: Vec<String> = vec![
        "install".to_string(),
        "-n".to_string(),
        env_name.to_string(),
        "cffi".to_string(),
        "-y".to_string(),
    ];
    let conda_cffi_args_refs: Vec<&str> = conda_cffi_args.iter().map(|s| s.as_str()).collect();
    info!("Executing command: {} {}", conda_executable.display(), conda_cffi_args.join(" "));
    current_phase_progress = run_command_for_setup_progress(
        app_handle, phase_name, "Installing cffi", current_phase_progress, 5,
        &conda_executable, &conda_cffi_args_refs,
        &comfyui_dir,
        "Starting cffi installation...", "cffi installed."
    ).await.map_err(|e| e.to_string())?;

    info!("Installing remaining dependencies from requirements.txt using 'conda run'...");
    let mut conda_run_args: Vec<String> = vec![
        "run".to_string(),
        "-n".to_string(),
        env_name.to_string(),
        "python".to_string(),
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "-vvv".to_string(),
        "--no-cache-dir".to_string(),
        "-r".to_string(),
        requirements_path.to_str().ok_or_else(|| "Failed to convert requirements_path to string".to_string())?.to_string(),
    ];

    // Conditionally add --extra-index-url for onnxruntime-gpu if CUDA 12.x is detected
    let gpu_info_for_pip_step = get_gpu_info();
    if gpu_info_for_pip_step.gpu_type == GpuType::Nvidia {
        if let Some(cuda_ver_str_ref) = gpu_info_for_pip_step.cuda_version.as_deref() {
            if cuda_ver_str_ref.starts_with("12.") {
                info!("Detected NVIDIA CUDA 12.x. Adding --extra-index-url for onnxruntime-gpu to pip install command.");
                conda_run_args.push("--extra-index-url".to_string());
                conda_run_args.push("https://aiinfra.pkgs.visualstudio.com/PublicPackages/_packaging/onnxruntime-cuda-12/pypi/simple/".to_string());
            }
        }
    }
    
    let conda_run_args_refs: Vec<&str> = conda_run_args.iter().map(|s| s.as_str()).collect();
    
    info!("Executing command: {} {}", conda_executable.display(), conda_run_args.join(" "));
    current_phase_progress = run_command_for_setup_progress(
        app_handle, phase_name, "Installing remaining dependencies from requirements.txt", current_phase_progress, 10,
        &conda_executable, &conda_run_args_refs,
        &comfyui_dir,
        "Starting installation of remaining dependencies...", "Remaining dependencies installed."
    ).await.map_err(|e| e.to_string())?;

    info!("Verifying PyTorch CUDA installation using check_torch.py...");
    let check_torch_py_path = comfyui_dir.join("check_torch.py");
    if !check_torch_py_path.exists() {
        let err_msg = format!("check_torch.py not found at {}", check_torch_py_path.display());
        error!("{}", err_msg);
        setup::emit_setup_progress(app_handle, "error", "Verification Script Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
        return Err(err_msg);
    }

    let check_torch_args: Vec<String> = vec![
        "run".to_string(),
        "-n".to_string(),
        env_name.to_string(),
        "python".to_string(),
        check_torch_py_path.to_str().ok_or_else(|| "Failed to convert check_torch.py path to string".to_string())?.to_string(),
    ];
    let check_torch_args_refs: Vec<&str> = check_torch_args.iter().map(|s| s.as_str()).collect();

    let _ = run_command_for_setup_progress(
        app_handle, phase_name, "Verifying PyTorch CUDA Setup", current_phase_progress, 5,
        &conda_executable, &check_torch_args_refs,
        &comfyui_dir,
        "Running PyTorch verification script...", "PyTorch verification script finished."
    ).await.map_err(|e| e.to_string())?;
    
    current_phase_progress = 100;
    setup::emit_setup_progress(app_handle, phase_name, "Python environment setup complete.", current_phase_progress, None, None);
    fs::write(&internal_deps_marker_path, "installed_via_setup_screen_flow").map_err(|e| format!("Failed to write internal dependency marker: {}", e))?;
    info!("Created internal dependency marker: {}", internal_deps_marker_path.display());
    
    Ok(())
}
