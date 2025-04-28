use std::fs;
use std::io::Write;
use std::env;
use tauri::{AppHandle, Manager, Wry};
use tauri::async_runtime::{self};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use log::{info, error};
use tauri_plugin_shell::ShellExt; // Import ShellExt for sidecar spawning
use tauri_plugin_shell::process::{CommandChild, CommandEvent}; // Import Command, CommandChild, and CommandEvent from shell plugin

// Global static variable to hold the child process handle
static COMFYUI_CHILD_PROCESS: Lazy<Mutex<Option<CommandChild>>> = Lazy::new(|| Mutex::new(None));

// Function to install Python dependencies
fn install_python_dependencies(app_handle: &AppHandle<Wry>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Checking if Python dependencies are installed...");

    // Define the path for the dependency installed marker file
    let app_data_dir = app_handle.path().app_data_dir()?;
    let marker_file_path = app_data_dir.join("dependencies_installed_marker");

    // Check if dependencies are already installed
    if marker_file_path.exists() {
        info!("Python dependencies already installed (marker file found).");
        return Ok(());
    }

    info!("Dependencies not installed. Starting installation process...");

    // Get the path to the bundled ComfyUI directory
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
    let comfyui_dir = exe_dir.join("../vendor/comfyui");

    let requirements_path = comfyui_dir.join("requirements.txt");
    if !requirements_path.exists() {
        error!("ComfyUI requirements.txt not found at: {}", requirements_path.display());
        return Err("ComfyUI requirements.txt not found".into());
    }

    info!("Reading dependencies from: {}", requirements_path.display());
    let requirements_content = fs::read_to_string(&requirements_path)?;

    let python_executable = exe_dir.join("../vendor/python/python.exe"); // Get python executable path

    if !python_executable.exists() {
        error!("Python executable not found at: {}", python_executable.display());
        return Err("Python executable not found".into());
    }

    let comfyui_site_packages_path = comfyui_dir.join("site-packages");
    let comfyui_site_packages_str = comfyui_site_packages_path.to_string_lossy().into_owned();

    let venv_dir = comfyui_dir.join(".venv");
    let venv_python_executable = if cfg!(target_os = "windows") {
        venv_dir.join("Scripts").join("python.exe")
    } else {
        venv_dir.join("bin").join("python")
    };

    // 1. Create a virtual environment if it doesn't exist
    if !venv_dir.exists() {
        info!("Creating virtual environment at: {}", venv_dir.display());
        let create_venv_output = std::process::Command::new(&python_executable)
            .arg("-m")
            .arg("venv")
            .arg(&venv_dir)
            .current_dir(&comfyui_dir) // Run venv creation from comfyui_dir
            .output()?;

        info!("Create venv stdout:\n---");
        info!("{}", String::from_utf8_lossy(&create_venv_output.stdout));
        info!("---");
        info!("Create venv stderr:\n---");
        error!("{}", String::from_utf8_lossy(&create_venv_output.stderr)); // Log stderr as error
        info!("---");

        if !create_venv_output.status.success() {
            error!("Failed to create virtual environment with status: {:?}", create_venv_output.status);
            return Err("Failed to create virtual environment".into());
        }
        info!("Virtual environment created successfully.");
    } else {
        info!("Virtual environment already exists at: {}", venv_dir.display());
    }

    // 2. Install dependencies into the virtual environment
    info!("Installing Python dependencies into virtual environment...");
    let mut pip_args: Vec<String> = vec![
        "-m".to_string(), "pip".to_string(), "install".to_string(),
        "--no-cache-dir".to_string(),
        "--verbose".to_string(),
        "--upgrade".to_string(),
        "--force-reinstall".to_string(), // Add this to ensure a clean installation
    ];

    // Add dependencies from requirements.txt
    let dependencies: Vec<String> = requirements_content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .map(|line| line.trim().to_string())
        .collect();

    for dep in dependencies {
        pip_args.push(dep); // Push owned String
    }

    // Add the index URL for CUDA 12.4 wheels (compatible with Python 3.12)

    // Convert pip_args to Vec<&str> for the command execution
    let pip_args_refs: Vec<&str> = pip_args.iter().map(|s| s.as_str()).collect();

    info!("Executing pip install command in venv: {:?} {:?}", &venv_python_executable, pip_args_refs);

    let install_output = std::process::Command::new(&venv_python_executable)
        .current_dir(&comfyui_dir) // Run pip install from comfyui_dir
        .args(&pip_args_refs)
        .output()?;

    info!("Pip install stdout:\n---");
    info!("{}", String::from_utf8_lossy(&install_output.stdout));
    info!("---");
    info!("Pip install stderr:\n---");
    error!("{}", String::from_utf8_lossy(&install_output.stderr)); // Log stderr as error
    info!("---");

    if !install_output.status.success() {
        error!("Pip install command failed with status: {:?}", install_output.status);
        return Err("Pip install failed".into());
    }

    info!("Successfully installed Python dependencies.");

    // Set a flag indicating dependencies are installed by creating a marker file
    info!("Creating dependency installed marker file: {}", marker_file_path.display());
    // Ensure the parent directory exists
    if let Some(parent) = marker_file_path.parent() {
        if !parent.exists() {
            info!("Creating parent directory for marker file: {}", parent.display());
            if let Err(e) = fs::create_dir_all(parent) {
                error!("Failed to create parent directory for marker file: {}", e);
                return Err("Failed to create parent directory for marker file".into());
            }
            info!("Parent directory created successfully.");
        }
    }
    if let Err(e) = fs::File::create(&marker_file_path) {
        error!("Failed to create dependency marker file: {}", e);
        return Err("Failed to create dependency marker file".into());
    }
    info!("Dependency marker file created successfully.");

    info!("Python dependency installation complete.");
    Ok(())
}

// Function to detect CUDA GPU presence using nvidia-smi
fn detect_cuda_gpu() -> bool {
    info!("Detecting CUDA GPU presence using nvidia-smi...");
    match std::process::Command::new("nvidia-smi").output() {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                info!("nvidia-smi output:\n{}", stdout);
                // Simple check: if the output contains "CUDA Version", assume CUDA GPU is present.
                // A more robust check might parse the output more thoroughly.
                let cuda_present = stdout.contains("CUDA Version");
                if cuda_present {
                    info!("CUDA GPU detected.");
                } else {
                    info!("No CUDA GPU detected based on nvidia-smi output.");
                }
                cuda_present
            } else {
                error!("nvidia-smi command failed with status: {:?}", output.status);
                error!("nvidia-smi stderr:\n{}", String::from_utf8_lossy(&output.stderr));
                info!("Defaulting to CPU mode due to nvidia-smi command failure.");
                false // Default to CPU mode on failure
            }
        }
        Err(e) => {
            error!("Failed to execute nvidia-smi: {}", e);
            info!("Defaulting to CPU mode because nvidia-smi could not be executed.");
            false // Default to CPU mode if nvidia-smi is not found or fails to execute
        }
    }
}

// Function to start the sidecar
fn start_comfyui_sidecar(app_handle: AppHandle<Wry>) {
    async_runtime::spawn(async move {
        info!("Attempting to start ComfyUI sidecar process...");

        // Install Python dependencies if not already installed
        if let Err(e) = install_python_dependencies(&app_handle) {
            error!("Failed to install Python dependencies: {}", e);
            // Depending on the desired behavior, you might want to show an error to the user
            // or prevent the ComfyUI sidecar from starting. For now, we just log the error.
            return;
        }

        // Get the path to the directory containing the current executable
        let exe_path = match std::env::current_exe() {
            Ok(path) => path,
            Err(e) => {
                error!("Failed to get current executable path: {}", e);
                return;
            }
        };
        let exe_dir = match exe_path.parent() {
             Some(dir) => dir.to_path_buf(), // Parent exists, convert &Path to PathBuf
             None => {
                 // Handle the case where there is no parent (e.g., root directory)
                 error!("Failed to get parent directory of executable: {}", exe_path.display());
                 // Depending on desired behavior, you might return an error,
                 // use the current directory, or panic. Returning here as before.
                 return;
             }
        };
        info!("Executable directory: {}", exe_dir.display());

        // Construct the path relative to the executable directory's parent
        // build.rs copies 'vendor' to the executable's parent directory (e.g., target/vendor)
        // So from target/debug/, we go up one level (../) to target/ and then into vendor/
        let comfyui_dir = exe_dir.join("../vendor/comfyui"); // Also update comfyui_dir path

        // Get the path to the virtual environment's Python executable
        let venv_dir = comfyui_dir.join(".venv");
        let venv_python_executable = if cfg!(target_os = "windows") {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python")
        };


        // Check if the constructed paths exist
        if !venv_python_executable.exists() {
            error!("Venv Python executable not found at expected path: {}", venv_python_executable.display());
            return;
        }
         if !comfyui_dir.exists() {
             error!("ComfyUI directory not found at expected path: {}", comfyui_dir.display());
             return;
         }

        let main_script = comfyui_dir.join("main.py");

        if !main_script.exists() {
            error!("ComfyUI main.py not found at: {}", main_script.display());
            return;
        }
        if !comfyui_dir.exists() {
            error!("ComfyUI directory not found at: {}", comfyui_dir.display());
            return;
        }

        info!("Using Venv Python executable: {}", venv_python_executable.display());
        info!("Using ComfyUI script: {}", main_script.display());
        info!("Setting CWD to: {}", comfyui_dir.display());

        // Detect GPU
        let use_cpu = !detect_cuda_gpu();
        let mut args = vec![
            main_script.to_string_lossy().into_owned(),
            "--listen".to_string(), // Add --listen argument
            "--front-end-version".to_string(), // Add frontend version argument
            "Comfy-Org/ComfyUI_frontend@v1.18.2".to_string(), // Specify frontend version v1.18.2
        ];

        if use_cpu {
            info!("No CUDA GPU detected or detection failed, adding --cpu flag.");
            args.push("--cpu".to_string());
        } else {
            info!("CUDA GPU detected, launching in GPU mode.");
        }

        info!("ComfyUI args: {:?}", args);

        info!("Spawning command:");
        info!("  Executable: {}", venv_python_executable.to_string_lossy());
        info!("  Args: {:?}", args);
        info!("  Working Directory: {}", comfyui_dir.display());

        let (mut rx, child) = app_handle.shell().command(venv_python_executable.to_string_lossy().to_string())
            .args(args.clone())
            .current_dir(&comfyui_dir) // Pass as reference
            .spawn()
            .expect("Failed to spawn ComfyUI process");

        info!("ComfyUI process started successfully (PID: {}).", child.pid());

        // Asynchronous logging using CommandEvent
        async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        info!("[ComfyUI stdout] {}", String::from_utf8_lossy(&line));
                    }
                    CommandEvent::Stderr(line) => {
                        error!("[ComfyUI stderr] {}", String::from_utf8_lossy(&line)); // Log stderr as error
                    }
                    CommandEvent::Terminated(exit_status) => {
                        info!("[ComfyUI] Process terminated with status: {:?}", exit_status);
                    }
                    _ => {} // Ignore other events for now
                }
            }
            info!("[ComfyUI] Output stream processing finished.");
        });


        // Store the child handle
        *COMFYUI_CHILD_PROCESS.lock().unwrap() = Some(child);

    });
}

// Function to stop the sidecar
fn stop_comfyui_sidecar() {
    info!("Attempting to stop ComfyUI sidecar process...");
    if let Ok(mut child_lock) = COMFYUI_CHILD_PROCESS.lock() {
        if let Some(child) = child_lock.take() { // Take ownership from the Option
            info!("Found ComfyUI process (PID: {}), attempting to kill...", child.pid());
            match child.kill() {
                Ok(_) => info!("ComfyUI process killed successfully."),
                Err(e) => error!("Failed to kill ComfyUI process: {}", e),
            }
        } else {
            info!("No active ComfyUI process found to stop.");
        }
    } else {
        error!("Failed to acquire lock for ComfyUI process handle.");
    }
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_log::Builder::default().build()) // Initialize log plugin first
    .plugin(tauri_plugin_fs::init()) // Initialize the FS plugin
    .plugin(tauri_plugin_shell::init()) // Initialize the Shell plugin
    .setup(|app| {
      // Logging should be configured via the plugin initialization above
      // You can still log here if needed after initialization
      info!("App setup started.");
      match app.handle().path().app_data_dir() {
          Ok(path) => info!("App data directory: {}", path.display()),
          Err(e) => error!("Failed to get app data directory: {}", e),
      }

      // Start the sidecar process
      start_comfyui_sidecar(app.handle().clone());

      Ok(())
    })
    .plugin(tauri_plugin_http::init()) // Register the HTTP plugin
    .on_window_event(|window, event| match event {
        tauri::WindowEvent::Destroyed => {
             // Ensure this only runs for the main window if multiple windows exist
             if window.label() == "main" { // Check label for main window
                 info!("Main window destroyed, stopping ComfyUI sidecar...");
                 stop_comfyui_sidecar();
             }
        }
        _ => {}
    })
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    .run(|_app_handle, event| match event { // Handle app exit events too
        tauri::RunEvent::ExitRequested { .. } => {
            info!("Exit requested, stopping ComfyUI sidecar...");
            stop_comfyui_sidecar();
            // Optionally prevent default exit and wait for cleanup
        }
        tauri::RunEvent::Exit => {
             info!("Application exiting.");
             // Sidecar should ideally be stopped by ExitRequested or WindowEvent::Destroyed
        }
        _ => {}
    });
}
