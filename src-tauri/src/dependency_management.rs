use std::path::Path;
use std::fs;
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
// use std::env; // No longer needed directly here for path resolution
use std::path::PathBuf; // Still needed for PathBuf type
use log::{info, error};
use crate::gpu_detection::{GpuType, get_gpu_info};
use tauri::{AppHandle, Wry}; // Removed unused Manager, Emitter
// use serde::Serialize; // Unused
use tokio::process::Command; // Replaced std::process::Command
use std::process::Stdio; // Stdio is still needed
use tokio::task; // Replaced std::thread
use fs2::available_space; // Import available_space
// use std::io::Write; // Unused
use crate::setup;
// Import new python_utils functions
use crate::setup_manager::python_utils::{
    get_comfyui_directory_path,
    get_bundled_python_executable_path,
    get_venv_python_executable_path,
};

// Unused InstallationStep enum, InstallationStatus struct, and emit_installation_status function are fully removed.

// This function executes a command and streams its stdout and stderr,
// logging each line with 'info!' for stdout and 'error!' for stderr.
// The command itself is logged before execution.
// It now emits the new `setup-progress` event.
async fn run_command_for_setup_progress(
    app_handle: &AppHandle<Wry>,
    phase: &str, // e.g., "python_setup"
    current_step_base: &str, // e.g., "Creating virtual environment"
    mut progress_current_phase: u8, // Current progress within this phase (0-100)
    progress_weight_of_this_command: u8, // How much this command contributes to the phase's 100%
    command_path: &PathBuf,
    args: &[&str],
    current_dir: &PathBuf,
    initial_message: &str, // Will be part of current_step
    // success_message: &str, // Will be part of current_step
    error_message_prefix: &str,
) -> Result<u8, Box<dyn std::error::Error>> { // Returns updated phase progress
    info!("Executing command for setup: {:?} {:?}", command_path, args);
    
    let step_name_initial = format!("{}: {}", current_step_base, initial_message);
    setup::emit_setup_progress(app_handle, phase, &step_name_initial, progress_current_phase, Some(initial_message.to_string()), None);

    let mut cmd = Command::new(command_path);
    cmd.current_dir(current_dir)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Specifically for venv creation, clear potentially problematic env vars
    if current_step_base == "Creating virtual environment" {
        cmd.env_remove("PYTHONHOME");
        cmd.env_remove("PYTHONPATH");
        info!("Cleared PYTHONHOME and PYTHONPATH for venv creation command.");
    }

    // --- Start Debug Logging for Environment ---
    info!("Debug: About to spawn command for setup.");
    info!("Debug: Current Directory: {:?}", current_dir);
    info!("Debug: Command Path: {:?}", command_path);
    info!("Debug: Arguments: {:?}", args);

    // Log environment variables - Be cautious not to log sensitive info in production
    #[cfg(debug_assertions)] // Only log in debug builds
    {
        info!("Debug: Environment Variables:");
        for (key, value) in std::env::vars() {
            // Filter out potentially sensitive variables if necessary, e.g., API keys, passwords
            let key_str = key;
            if !key_str.contains("API_KEY") && !key_str.contains("PASSWORD") {
                 info!("  {}: {:?}", key_str, value);
            } else {
                 info!("  {}: [REDACTED]", key_str);
            }
        }
    }
    info!("Debug: Finished logging environment.");
    // --- End Debug Logging for Environment ---

    // --- Start Modify PATH for Bundled Python and Venv ---
    info!("Modifying PATH to prioritize bundled Python and Venv...");

    let bundled_python_exe = get_bundled_python_executable_path(app_handle)?;
    let bundled_python_dir = bundled_python_exe.parent()
        .ok_or_else(|| "Failed to get bundled Python directory".to_string())?;

    // Get the venv bin/Scripts directory
    let venv_python_exe = get_venv_python_executable_path(app_handle)?;
    let venv_bin_dir = venv_python_exe.parent()
        .ok_or_else(|| "Failed to get venv bin directory".to_string())?;


    let current_system_path = std::env::var("PATH")
        .unwrap_or_else(|_| "".to_string()); // Get existing PATH or empty string

    let system_path_buf = PathBuf::from(current_system_path); // Create a longer-lived PathBuf

    let new_path_dirs: Vec<&Path> = vec![
        venv_bin_dir.as_ref(), // Venv bin/Scripts first
        bundled_python_dir.as_ref(), // Bundled Python dir second
        system_path_buf.as_ref(), // Use the longer-lived reference
    ];

    let new_path = std::env::join_paths(new_path_dirs)
        .map_err(|e| format!("Failed to join paths for new PATH: {}", e))?;

    cmd.env("PATH", &new_path);

    info!("Debug: Set new PATH for spawned command: {:?}", new_path);
    // --- End Modify PATH ---

    let mut child = cmd.spawn()?;

    let stdout = child.stdout.take().ok_or(format!("{} - Failed to capture stdout", error_message_prefix))?;
    let stderr = child.stderr.take().ok_or(format!("{} - Failed to capture stderr", error_message_prefix))?;

    let app_handle_clone_stdout = app_handle.clone();
    let phase_clone_stdout = phase.to_string();
    let current_step_base_clone_stdout = current_step_base.to_string();
    let stdout_task = task::spawn(async move {
        let mut reader = TokioBufReader::new(stdout);
        let mut line_buf = String::new();
        while let Ok(n) = reader.read_line(&mut line_buf).await {
            if n == 0 { break; } // EOF
            let line_to_emit = line_buf.trim_end().to_string();
            info!("Stdout (setup): {}", line_to_emit);
            let step_name = format!("{}: {}", current_step_base_clone_stdout, line_to_emit);
            // Progress doesn't change per line here, just detail_message
            setup::emit_setup_progress(&app_handle_clone_stdout, &phase_clone_stdout, &step_name, progress_current_phase, Some(line_to_emit), None);
            line_buf.clear();
        }
    });

    let app_handle_clone_stderr = app_handle.clone();
    let phase_clone_stderr = phase.to_string();
    let current_step_base_clone_stderr = current_step_base.to_string();
    let stderr_task = task::spawn(async move {
        let mut reader = TokioBufReader::new(stderr);
        let mut line_buf = String::new();
        while let Ok(n) = reader.read_line(&mut line_buf).await {
            if n == 0 { break; } // EOF
            let line_to_emit = line_buf.trim_end().to_string();
            error!("Stderr (setup): {}", line_to_emit);
            let step_name = format!("{}: {}", current_step_base_clone_stderr, line_to_emit);
            setup::emit_setup_progress(&app_handle_clone_stderr, &phase_clone_stderr, &step_name, progress_current_phase, Some(line_to_emit.clone()), Some(line_to_emit));
            line_buf.clear();
        }
    });

    let status = child.wait().await?;

    // Debug: Log the raw exit status immediately after waiting
    info!("Debug: Command exit status: {:?}", status);

    stdout_task.await.map_err(|e| format!("Stdout task (setup) panicked: {:?}", e))?;
    stderr_task.await.map_err(|e| format!("Stderr task (setup) panicked: {:?}", e))?;

    if !status.success() {
        // Enhance error message to include the full command string
        let command_string = format!("{:?} {:?}", command_path, args);
        let error_msg = format!("{} failed with status: {:?}. Command: {}", error_message_prefix, status, command_string);
        error!("{}", error_msg);
        setup::emit_setup_progress(app_handle, phase, error_message_prefix, progress_current_phase, Some(error_msg.clone()), Some(error_msg.clone()));
        return Err(error_msg.into());
    }
    
    progress_current_phase += progress_weight_of_this_command;
    let success_step_name = format!("{}: Completed successfully.", current_step_base);
    info!("{}", success_step_name);
    setup::emit_setup_progress(app_handle, phase, &success_step_name, progress_current_phase.min(100), None, None);
    Ok(progress_current_phase.min(100))
}

// Estimated required disk space for ComfyUI dependencies (20 GB)
const REQUIRED_DISK_SPACE: u64 = 20 * 1024 * 1024 * 1024; // in bytes

// Unused function install_python_dependencies removed.
// pub async fn install_python_dependencies(app_handle: &AppHandle<Wry>) -> Result<(), Box<dyn std::error::Error>> {
//     ...
// }


// New function for SetupScreen with detailed progress
pub async fn install_python_dependencies_with_progress(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    let phase_name = "python_setup"; // Or "installing_comfyui" - needs consistency with SetupScreen
    let mut current_phase_progress: u8 = 0;

    info!("Checking disk space (with progress)...");
    setup::emit_setup_progress(app_handle, phase_name, "Checking available disk space...", current_phase_progress, None, None);

    // let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get current exe path: {}", e))?; // Replaced
    // let exe_dir = exe_path.parent().ok_or_else(|| "Failed to get executable directory".to_string())?; // Replaced
    
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

    // Marker for this specific function's full completion (Python deps + venv)
    // This is different from the Master Installation Marker.
    let internal_deps_marker_path = comfyui_dir.join(".venv_deps_installed.marker");

    let python_executable = get_bundled_python_executable_path(app_handle)?;
    
    if !python_executable.exists() {
        let err_msg = format!("Bundled Python executable not found at {}", python_executable.display());
        error!("{}", err_msg);
        setup::emit_setup_progress(app_handle, "error", "Python Executable Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
        return Err(err_msg);
    }

    let venv_dir = comfyui_dir.join(".venv"); // Define venv_dir as it's used directly
    let venv_python_executable = get_venv_python_executable_path(app_handle)?;

    let requirements_path = comfyui_dir.join("requirements.txt");
     info!("Attempting to access requirements.txt at: {}", requirements_path.display());
    if !requirements_path.exists() {
        let err_msg = format!("ComfyUI requirements.txt not found at {}", requirements_path.display());
        error!("{}", err_msg);
        setup::emit_setup_progress(app_handle, "error", "Requirements File Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
        return Err(err_msg);
    }

    // Check if venv exists and internal marker is valid
    if venv_dir.exists() && venv_python_executable.exists() && internal_deps_marker_path.exists() {
        // TODO: Add hash check for requirements.txt if marker exists
        info!("Python virtual environment and dependencies appear to be installed (marker found). Skipping pip install.");
        current_phase_progress = 100; // Mark as complete if marker exists
        setup::emit_setup_progress(app_handle, phase_name, "Python environment already set up.", current_phase_progress, None, None);
        return Ok(());
    }

    info!("Python dependencies need installation or verification. Starting process...");

    // Step: Create Virtual Environment if it doesn't exist
    if !venv_dir.exists() {
        info!("Virtual environment not found at {}. Creating...", venv_dir.display());
        current_phase_progress = run_command_for_setup_progress(
            app_handle, phase_name, "Creating virtual environment", current_phase_progress, 15, // Weight: 15%
            &python_executable, &["-m", "venv", "--without-pip", venv_dir.to_str().unwrap()], &comfyui_dir, // Removed --upgrade-deps
            "Initializing...", "Virtual environment created."
        ).await.map_err(|e| e.to_string())?; // After venv creation, progress is 25 (15 base + 10 weight)
    } else {
        info!("Virtual environment found at {}. Skipping creation.", venv_dir.display());
        current_phase_progress = std::cmp::max(current_phase_progress, 25); // Assume venv creation part is done
        setup::emit_setup_progress(app_handle, phase_name, "Virtual environment already exists.", current_phase_progress, None, None);
    }
    
    // If we reached here, pip install is necessary (either no venv, or no marker)
    // Remove old marker if it exists, as we are about to reinstall
    if internal_deps_marker_path.exists() {
        info!("Removing existing internal dependency marker: {}", internal_deps_marker_path.display());
        fs::remove_file(&internal_deps_marker_path).map_err(|e| format!("Failed to remove old internal marker: {}", e))?;
    }

    // Step: Download get-pip.py
    info!("Downloading get-pip.py...");
    let get_pip_url = "https://bootstrap.pypa.io/get-pip.py";
    let get_pip_path = comfyui_dir.join("get-pip.py");
    
    setup::emit_setup_progress(app_handle, phase_name, "Downloading get-pip.py", current_phase_progress, Some(format!("From: {}", get_pip_url)), None);

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
                        current_phase_progress += 5; // Weight: 5% for download
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

    // Step: Execute get-pip.py
    info!("Executing get-pip.py...");
    current_phase_progress = run_command_for_setup_progress(
        app_handle, phase_name, "Installing pip using get-pip.py", current_phase_progress, 10, // Weight: 10% for execution
        &venv_python_executable, &[get_pip_path.to_str().unwrap()], &comfyui_dir,
        "Running get-pip.py...", "pip installed via get-pip.py."
    ).await.map_err(|e| e.to_string())?; // After get-pip.py execution, progress is 40 (25 base + 5 download + 10 exec)

    // Step: Clean up get-pip.py
    info!("Cleaning up get-pip.py...");
    match tokio::fs::remove_file(&get_pip_path).await {
        Ok(_) => info!("Successfully removed get-pip.py"),
        Err(e) => error!("Failed to remove get-pip.py at {}: {}", get_pip_path.display(), e),
    }
    // No progress update for cleanup, it's a minor step

    // Step: Install PyTorch, Torchvision, Torchaudio explicitly
    info!("Attempting to install PyTorch, Torchvision, and Torchaudio explicitly...");
    let torch_packages = vec![
        "torch==2.3.1".to_string(), // Pin to a known stable version compatible with cu121 and Python 3.12
        "torchvision==0.18.1".to_string(),
        "torchaudio==2.3.1".to_string(),
    ];
    let mut pip_torch_args: Vec<String> = vec![ // This variable will be populated with torch_packages and index urls
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "-vvv".to_string(),
        "--no-cache-dir".to_string(),
    ];
    // pip_torch_args.extend(torch_packages); // Will extend after determining index URL
    
    // Determine torch_index_url_to_use
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

   // Now build pip_torch_args using the determined torch_index_url_for_explicit_install
   pip_torch_args.extend(torch_packages); // Add the torch packages first

   if let Some(url) = &torch_index_url_for_explicit_install {
       info!("Using PyTorch index URL for explicit torch install: {}", url);
       pip_torch_args.push("--index-url".to_string());
       pip_torch_args.push(url.clone());
       pip_torch_args.push("--extra-index-url".to_string()); // Add PyPI as extra for torch's own dependencies
       pip_torch_args.push("https://pypi.org/simple".to_string());
   } else {
       // This case should ideally not be reached if the logic above always sets a URL (even if CPU)
       error!("Critical error: Could not determine PyTorch index URL for explicit install. Attempting with CPU fallback and PyPI.");
       // Fallback to CPU and PyPI if no URL was determined (should not happen with current logic)
       pip_torch_args.push("--index-url".to_string());
       pip_torch_args.push("https://download.pytorch.org/whl/cpu".to_string());
       pip_torch_args.push("--extra-index-url".to_string());
       pip_torch_args.push("https://pypi.org/simple".to_string());
   }

   let pip_torch_args_refs: Vec<&str> = pip_torch_args.iter().map(|s| s.as_str()).collect();
   current_phase_progress = run_command_for_setup_progress(
        app_handle, phase_name, "Installing PyTorch, Torchvision, Torchaudio", current_phase_progress, 30, // Weight: 30% (40 base + 30 weight = 70)
        &venv_python_executable, &pip_torch_args_refs,
        &comfyui_dir,
        "Starting explicit PyTorch installation...", "PyTorch, Torchvision, Torchaudio installed."
    ).await.map_err(|e| e.to_string())?;

    // Step: Explicitly install a compatible NumPy version (1.x)
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
        app_handle, phase_name, "Installing NumPy 1.x", current_phase_progress, 5, // Weight: 5% (70 base + 5 weight = 75)
        &venv_python_executable, &pip_numpy_args_refs,
        &comfyui_dir,
        "Starting NumPy 1.x installation...", "NumPy 1.x installed."
    ).await.map_err(|e| e.to_string())?;


    // Step: Install remaining dependencies from requirements.txt
    info!("Installing remaining dependencies from requirements.txt...");
    let pip_args_combined: Vec<String> = vec![
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "-vvv".to_string(),
        "--no-cache-dir".to_string(),
        // "--upgrade".to_string(), // Consider if upgrade is needed for other deps
        // "--force-reinstall".to_string(), // May not be needed for other deps
        // "--no-deps".to_string(), // If torch, torchvision, torchaudio are in reqs.txt and cause issues
        "-r".to_string(),
        requirements_path.to_str().ok_or_else(|| "Failed to convert requirements_path to string".to_string())?.to_string(),
    ];
    
    // For the second pass, we primarily rely on PyPI.
    // No explicit --index-url or --extra-index-url needed here unless specific other packages require it.
    // Pip will use its default (PyPI).

    let script_args_combined_refs: Vec<&str> = pip_args_combined.iter().map(|s| s.as_str()).collect();
    current_phase_progress = run_command_for_setup_progress(
        app_handle, phase_name, "Installing remaining dependencies from requirements.txt", current_phase_progress, 20, // Weight: 20% (75 base + 20 weight = 95)
        &venv_python_executable, &script_args_combined_refs,
        &comfyui_dir,
        "Starting installation of remaining dependencies...", "Remaining dependencies installed."
    ).await.map_err(|e| e.to_string())?;

    // Step: Verify PyTorch CUDA installation using check_torch.py
    info!("Verifying PyTorch CUDA installation using check_torch.py...");
    let check_torch_py_path = comfyui_dir.join("check_torch.py");
    if !check_torch_py_path.exists() {
        let err_msg = format!("check_torch.py not found at {}", check_torch_py_path.display());
        error!("{}", err_msg);
        setup::emit_setup_progress(app_handle, "error", "Verification Script Error", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
        return Err(err_msg);
    }

    let _ = run_command_for_setup_progress( // Assign to _ to mark as intentionally unused
        app_handle, phase_name, "Verifying PyTorch CUDA Setup", current_phase_progress, 5, // Weight: 5% (95 base + 5 weight = 100)
        &venv_python_executable, &[check_torch_py_path.to_str().ok_or_else(|| "Failed to convert check_torch.py path to string".to_string())?],
        &comfyui_dir, // Run check_torch.py from within comfyui_dir
        "Running PyTorch verification script...", "PyTorch verification script finished."
    ).await.map_err(|e| e.to_string())?;
    // current_phase_progress is updated internally by run_command_for_setup_progress and emitted
    
    // Final step: marker file
    current_phase_progress = 100; // Ensure it reaches 100
    setup::emit_setup_progress(app_handle, phase_name, "Python environment setup complete.", current_phase_progress, None, None);
    fs::write(&internal_deps_marker_path, "installed_via_setup_screen_flow").map_err(|e| format!("Failed to write internal dependency marker: {}", e))?;
    info!("Created internal dependency marker: {}", internal_deps_marker_path.display());
    
    Ok(())
}

// Unused function write_temp_python_script removed.
// pub fn write_temp_python_script(content: &str, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
//     info!("Writing temporary Python script to: {}", file_path.display());
//     let mut file = fs::File::create(file_path)?;
//     file.write_all(content.as_bytes())?;
//     info!("Temporary Python script written successfully.");
//     Ok(())
// }