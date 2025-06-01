// metamorphosis-app/src-tauri/src/setup_manager/dependency_manager/python_env.rs
use std::path::PathBuf;
use std::fs;
use log::{info, error};
use tauri::{AppHandle, Wry};
use fs2::available_space; // For disk space check

use crate::gpu_detection::{GpuType, get_gpu_info};
use crate::setup; // For emit_setup_progress
use crate::setup_manager::python_utils::{
    get_comfyui_directory_path,
    get_bundled_python_executable_path,
    get_venv_python_executable_path,
};

// Import from sibling modules
use super::command_runner::run_command_for_setup_progress;
use super::disk_utils::REQUIRED_DISK_SPACE;


// New function for SetupScreen with detailed progress
pub async fn install_python_dependencies_with_progress(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    let phase_name = "python_setup"; // Or "installing_comfyui" - needs consistency with SetupScreen
    let mut current_phase_progress: u8 = 0;

    info!("Checking disk space (with progress)...");
    setup::emit_setup_progress(app_handle, phase_name, "Checking available disk space...", current_phase_progress, None, None);
    
    let comfyui_dir = get_comfyui_directory_path(app_handle)?;

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

    let internal_deps_marker_path = comfyui_dir.join(".venv_deps_installed.marker");

    let python_executable = get_bundled_python_executable_path(app_handle)?;
    
    if !python_executable.exists() {
        let err_msg = format!("Bundled Python executable not found at {}", python_executable.display());
        error!("{}", err_msg);
        setup::emit_setup_progress(app_handle, "error", "Python Executable Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
        return Err(err_msg);
    }

    let venv_dir = comfyui_dir.join(".venv");
    let venv_python_executable = get_venv_python_executable_path(app_handle)?;

    let requirements_path = comfyui_dir.join("requirements.txt");
     info!("Attempting to access requirements.txt at: {}", requirements_path.display());
    if !requirements_path.exists() {
        let err_msg = format!("ComfyUI requirements.txt not found at {}", requirements_path.display());
        error!("{}", err_msg);
        setup::emit_setup_progress(app_handle, "error", "Requirements File Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
        return Err(err_msg);
    }

    if venv_dir.exists() && venv_python_executable.exists() && internal_deps_marker_path.exists() {
        info!("Python virtual environment and dependencies appear to be installed (marker found). Skipping pip install.");
        current_phase_progress = 100;
        setup::emit_setup_progress(app_handle, phase_name, "Python environment already set up.", current_phase_progress, None, None);
        return Ok(());
    }

    info!("Python dependencies need installation or verification. Starting process...");

    if !venv_dir.exists() {
        info!("Virtual environment not found at {}. Creating...", venv_dir.display());
        current_phase_progress = run_command_for_setup_progress(
            app_handle, phase_name, "Creating virtual environment", current_phase_progress, 15,
            &python_executable, &["-m", "venv", "--without-pip", venv_dir.to_str().unwrap()], &comfyui_dir,
            "Initializing...", "Virtual environment created."
        ).await.map_err(|e| e.to_string())?;
    } else {
        info!("Virtual environment found at {}. Skipping creation.", venv_dir.display());
        current_phase_progress = std::cmp::max(current_phase_progress, 25);
        setup::emit_setup_progress(app_handle, phase_name, "Virtual environment already exists.", current_phase_progress, None, None);
    }
    
    if internal_deps_marker_path.exists() {
        info!("Removing existing internal dependency marker: {}", internal_deps_marker_path.display());
        fs::remove_file(&internal_deps_marker_path).map_err(|e| format!("Failed to remove old internal marker: {}", e))?;
    }

    info!("Downloading get-pip.py...");
    let get_pip_url = "https://bootstrap.pypa.io/get-pip.py";
    let get_pip_path = comfyui_dir.join("get-pip.py");
    
    setup::emit_setup_progress(app_handle, phase_name, "Downloading get-pip.py", current_phase_progress, Some("Fetching installation script...".to_string()), None);

    match reqwest::get(get_pip_url).await {
        Ok(response) => {
            if response.status().is_success() {
                let mut file = match tokio::fs::File::create(&get_pip_path).await {
                    Ok(f) => f,
                    Err(e) => {
                        let err_msg = format!("Failed to create get-pip.py file at {}: {}", get_pip_path.display(), e);
                        error!("{}", err_msg);
                        setup::emit_setup_progress(app_handle, "error", "File Creation Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
                        return Err(err_msg);
                    }
                };
                let content = match response.bytes().await {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        let err_msg = format!("Failed to read get-pip.py response bytes: {}", e);
                        error!("{}", err_msg);
                        setup::emit_setup_progress(app_handle, "error", "Download Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
                        return Err(err_msg);
                    }
                };
                match tokio::io::AsyncWriteExt::write_all(&mut file, &content).await {
                    Ok(_) => {
                        info!("Successfully downloaded get-pip.py to {}", get_pip_path.display());
                        current_phase_progress += 5;
                        setup::emit_setup_progress(app_handle, phase_name, "get-pip.py downloaded.", current_phase_progress, None, None);
                    },
                    Err(e) => {
                        let err_msg = format!("Failed to write get-pip.py content to file: {}", e);
                        error!("{}", err_msg);
                        setup::emit_setup_progress(app_handle, "error", "File Write Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
                        return Err(err_msg);
                    }
                }
            } else {
                let err_msg = format!("Failed to download get-pip.py: HTTP status {}", response.status());
                error!("{}", err_msg);
                setup::emit_setup_progress(app_handle, "error", "Download Failed", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
                return Err(err_msg);
            }
        },
        Err(e) => {
            let err_msg = format!("Failed to send request to download get-pip.py: {}", e);
            error!("{}", err_msg);
            setup::emit_setup_progress(app_handle, "error", "Network Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
            return Err(err_msg);
        }
    }

    info!("Executing get-pip.py...");
    current_phase_progress = run_command_for_setup_progress(
        app_handle, phase_name, "Installing pip using get-pip.py", current_phase_progress, 10,
        &venv_python_executable, &[get_pip_path.to_str().unwrap()], &comfyui_dir,
        "Running get-pip.py...", "pip installed via get-pip.py."
    ).await.map_err(|e| e.to_string())?;

    info!("Cleaning up get-pip.py...");
    match tokio::fs::remove_file(&get_pip_path).await {
        Ok(_) => info!("Successfully removed get-pip.py"),
        Err(e) => error!("Failed to remove get-pip.py at {}: {}", get_pip_path.display(), e),
    }

    info!("Attempting to install PyTorch, Torchvision, and Torchaudio explicitly...");
    let torch_packages = vec![
        "torch==2.3.1".to_string(),
        "torchvision==0.18.1".to_string(),
        "torchaudio==2.3.1".to_string(),
    ];
    let mut pip_torch_args: Vec<String> = vec![
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "-vvv".to_string(),
        "--no-cache-dir".to_string(),
    ];
    
    let gpu_info_for_torch_step = get_gpu_info();
    let torch_index_url_for_explicit_install: Option<String> = if gpu_info_for_torch_step.gpu_type == GpuType::Nvidia {
        if let Some(cuda_ver_str_ref) = gpu_info_for_torch_step.cuda_version.as_deref() {
            if !cuda_ver_str_ref.is_empty() {
                let cuda_ver_str = cuda_ver_str_ref.to_string();
                info!("(Explicit Torch Install) Detected NVIDIA GPU with CUDA version string: {}", cuda_ver_str);
                let suffix = if cuda_ver_str.starts_with("12.") { "cu121".to_string() }
                             else if cuda_ver_str.starts_with("11.") {
                                 let parts: Vec<&str> = cuda_ver_str.split('.').collect();
                                 if parts.len() >= 2 { format!("cu{}{}", parts[0], parts[1]) } else { "cpu".to_string() }
                             } else {
                                 error!("(Explicit Torch Install) Unsupported NVIDIA CUDA version prefix: {}. Falling back to CPU PyTorch.", cuda_ver_str);
                                 "cpu".to_string()
                             };
               if suffix != "cpu" {
                   Some(format!("https://download.pytorch.org/whl/{}", suffix))
               } else {
                   Some("https://download.pytorch.org/whl/cpu".to_string())
               }
           } else {
               error!("(Explicit Torch Install) NVIDIA GPU detected, but CUDA version string is empty. Falling back to CPU PyTorch for explicit install.");
               Some("https://download.pytorch.org/whl/cpu".to_string())
           }
       } else {
           error!("(Explicit Torch Install) NVIDIA GPU detected, but no CUDA version found. Falling back to CPU PyTorch for explicit install.");
           Some("https://download.pytorch.org/whl/cpu".to_string())
       }
   } else {
       info!("(Explicit Torch Install) Non-NVIDIA GPU detected ({:?}). Using CPU PyTorch for explicit install.", gpu_info_for_torch_step.gpu_type);
       Some("https://download.pytorch.org/whl/cpu".to_string())
   };

   pip_torch_args.extend(torch_packages);

   if let Some(url) = &torch_index_url_for_explicit_install {
       info!("Using PyTorch index URL for explicit torch install: {}", url);
       pip_torch_args.push("--index-url".to_string());
       pip_torch_args.push(url.clone());
       pip_torch_args.push("--extra-index-url".to_string());
       pip_torch_args.push("https://pypi.org/simple".to_string());
   } else {
       error!("Critical error: Could not determine PyTorch index URL for explicit install. Attempting with CPU fallback and PyPI.");
       pip_torch_args.push("--index-url".to_string());
       pip_torch_args.push("https://download.pytorch.org/whl/cpu".to_string());
       pip_torch_args.push("--extra-index-url".to_string());
       pip_torch_args.push("https://pypi.org/simple".to_string());
   }

   let pip_torch_args_refs: Vec<&str> = pip_torch_args.iter().map(|s| s.as_str()).collect();
   current_phase_progress = run_command_for_setup_progress(
        app_handle, phase_name, "Installing PyTorch, Torchvision, Torchaudio", current_phase_progress, 30,
        &venv_python_executable, &pip_torch_args_refs,
        &comfyui_dir,
        "Starting explicit PyTorch installation...", "PyTorch, Torchvision, Torchaudio installed."
    ).await.map_err(|e| e.to_string())?;

    info!("Attempting to install compatible NumPy version (1.x)...");
    let numpy_pkg = "numpy~=1.26.4";
    let pip_numpy_args: Vec<String> = vec![
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "-vvv".to_string(),
        "--no-cache-dir".to_string(),
        numpy_pkg.to_string(),
    ];
    let pip_numpy_args_refs: Vec<&str> = pip_numpy_args.iter().map(|s| s.as_str()).collect();
    current_phase_progress = run_command_for_setup_progress(
        app_handle, phase_name, "Installing NumPy 1.x", current_phase_progress, 5,
        &venv_python_executable, &pip_numpy_args_refs,
        &comfyui_dir,
        "Starting NumPy 1.x installation...", "NumPy 1.x installed."
    ).await.map_err(|e| e.to_string())?;


    info!("Installing remaining dependencies from requirements.txt...");
    let pip_args_combined: Vec<String> = vec![
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "-vvv".to_string(),
        "--no-cache-dir".to_string(),
        "-r".to_string(),
        requirements_path.to_str().ok_or_else(|| "Failed to convert requirements_path to string".to_string())?.to_string(),
    ];
    
    let script_args_combined_refs: Vec<&str> = pip_args_combined.iter().map(|s| s.as_str()).collect();
    current_phase_progress = run_command_for_setup_progress(
        app_handle, phase_name, "Installing remaining dependencies from requirements.txt", current_phase_progress, 20,
        &venv_python_executable, &script_args_combined_refs,
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

    let _ = run_command_for_setup_progress(
        app_handle, phase_name, "Verifying PyTorch CUDA Setup", current_phase_progress, 5,
        &venv_python_executable, &[check_torch_py_path.to_str().ok_or_else(|| "Failed to convert check_torch.py path to string".to_string())?],
        &comfyui_dir,
        "Running PyTorch verification script...", "PyTorch verification script finished."
    ).await.map_err(|e| e.to_string())?;
    
    current_phase_progress = 100;
    setup::emit_setup_progress(app_handle, phase_name, "Python environment setup complete.", current_phase_progress, None, None);
    fs::write(&internal_deps_marker_path, "installed_via_setup_screen_flow").map_err(|e| format!("Failed to write internal dependency marker: {}", e))?;
    info!("Created internal dependency marker: {}", internal_deps_marker_path.display());
    
    Ok(())
}

/// Installs Python dependencies from a requirements.txt file for a given custom node.
pub async fn install_custom_node_dependencies(
    app_handle: AppHandle<Wry>,
    pack_name: String,
    pack_dir: PathBuf,
) -> Result<(), String> {
    let phase_name = "custom_node_deps";
    let mut current_phase_progress: u8 = 0;

    info!("[CUSTOM_NODE_DEPS] Installing dependencies for {}: {}", pack_name, pack_dir.display());
    setup::emit_setup_progress(&app_handle, phase_name, &format!("Starting dependency installation for {}", pack_name), current_phase_progress, None, None);

    let requirements_path = pack_dir.join("requirements.txt");

    if !requirements_path.exists() {
        info!("[CUSTOM_NODE_DEPS] No requirements.txt found for {}. Skipping dependency installation.", pack_name);
        current_phase_progress = 100;
        setup::emit_setup_progress(&app_handle, phase_name, &format!("No requirements.txt for {}. Skipping.", pack_name), current_phase_progress, None, None);
        return Ok(());
    }

    let venv_python_executable = get_venv_python_executable_path(&app_handle)?;

    if !venv_python_executable.exists() {
        let err_msg = format!("Venv Python executable not found at {} for {} dependency installation.", venv_python_executable.display(), pack_name);
        error!("{}", err_msg);
        setup::emit_setup_progress(&app_handle, "error", &format!("Venv Python Missing for {}", pack_name), current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
        return Err(err_msg);
    }

    let step_base = format!("Installing dependencies for {}", pack_name);
    let pip_args = ["-m", "pip", "install", "-r", requirements_path.to_str().unwrap_or_default()];

    match run_command_for_setup_progress(
        &app_handle,
        phase_name,
        &step_base,
        0, 
        100, 
        &venv_python_executable,
        &pip_args,
        &pack_dir, 
        "Reading requirements.txt...",
        &format!("Failed to install dependencies for {}", pack_name),
    ).await {
        Ok(final_progress) => {
            info!("[CUSTOM_NODE_DEPS] Successfully installed dependencies for {}. Final progress: {}", pack_name, final_progress);
            Ok(())
        }
        Err(e) => {
            let err_msg = format!("[CUSTOM_NODE_DEPS] Error installing dependencies for {}: {}", pack_name, e);
            error!("{}", err_msg);
            Err(err_msg)
        }
    }
}