use std::path::PathBuf;
use std::error::Error; // Import the Error trait
use tauri::{AppHandle, Manager, Wry};
use tauri::path::BaseDirectory;
use tauri::async_runtime::{self};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use log::{info, error};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use scopeguard; // Import scopeguard for the cleanup guard
use tauri_plugin_http::reqwest; // For making HTTP requests
use tokio::time::{interval, Duration}; // For periodic health checks
use std::time::Instant; // To track restart times
use std::pin::Pin;
use std::future::Future;

// Import necessary modules from the crate
use crate::gpu_detection::{GpuInfo, GpuType, get_gpu_info};
use crate::dependency_management::{self, InstallationStep, InstallationStatus}; // Import Installation types
use crate::setup; // To use emit_setup_progress
use tauri::Emitter; // Import Emitter for sending events
use serde_json::json; // For creating JSON payloads

// Global static variable to hold the child process handle
static COMFYUI_CHILD_PROCESS: Lazy<Mutex<Option<CommandChild>>> = Lazy::new(|| Mutex::new(None));
static IS_ATTEMPTING_SPAWN: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false)); // New flag for initial spawn attempt
static RESTART_ATTEMPTS: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));
static LAST_RESTART_TIME: Lazy<Mutex<Option<Instant>>> = Lazy::new(|| Mutex::new(None));
const MAX_RESTARTS_PER_HOUR: u32 = 5; // Define a limit for restarts
const COMFYUI_PORT: u16 = 8188; // TODO: Make this configurable

// Helper to emit backend status
fn emit_backend_status(app_handle: &AppHandle<Wry>, status: &str, message: String, is_error: bool) {
    if let Err(e) = app_handle.emit("backend-status", json!({
        "status": status,
        "message": message,
        "isError": is_error,
    })) {
        error!("Failed to emit backend status event: {}", e);
    }
}


// Renamed: Internal function to actually spawn the sidecar process
// Assumes dependencies are already installed. Returns Result.
fn spawn_comfyui_process(app_handle: AppHandle<Wry>) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
    log::error!("[EARLY_SPAWN_DEBUG] INTERNAL spawn_comfyui_process INVOKED");
    
    { // Scope for the is_attempting_spawn_guard
        let mut is_attempting_spawn_guard = IS_ATTEMPTING_SPAWN.lock().unwrap();
        if *is_attempting_spawn_guard {
            info!("[GUARD] spawn_comfyui_process: Another spawn attempt is already in progress by a concurrent call. Skipping this one.");
            // Return an error or a specific Ok status to indicate skipping due to ongoing attempt
            return Box::pin(async { Err("Spawn attempt already in progress.".to_string()) });
        }
        // Check if a process is already running *before* marking this attempt
        if COMFYUI_CHILD_PROCESS.lock().unwrap().is_some() {
            info!("[GUARD] spawn_comfyui_process: ComfyUI process is already active. Skipping new spawn attempt.");
            return Box::pin(async { Ok(()) });
        }
        *is_attempting_spawn_guard = true; // Mark that this call is now attempting the spawn
        info!("[GUARD] spawn_comfyui_process: Marked IS_ATTEMPTING_SPAWN = true.");
    } // is_attempting_spawn_guard is dropped

    let app_handle_clone = app_handle.clone();
    Box::pin(async move {
        // Ensure IS_ATTEMPTING_SPAWN is reset when this async block finishes, regardless of outcome.
        let _reset_spawning_flag_guard = scopeguard::guard((), |_| {
            let mut guard = IS_ATTEMPTING_SPAWN.lock().unwrap();
            if *guard { // Only reset if this instance was the one that set it (though it always should be if it got this far)
                *guard = false;
                info!("[GUARD_CLEANUP] spawn_comfyui_process: IS_ATTEMPTING_SPAWN flag reset to false.");
            }
        });

        info!("Attempting to spawn ComfyUI process on port {}...", COMFYUI_PORT);
        emit_backend_status(&app_handle, "starting_sidecar", format!("Starting ComfyUI backend on port {}...", COMFYUI_PORT), false);

        // Get the path to the directory containing the current executable
        let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get current executable path: {}", e))?;
        // Removed dangling Ok/Err arms from previous match statement
        let exe_dir = exe_path.parent().ok_or_else(|| format!("Failed to get parent directory of executable: {}", exe_path.display()))?.to_path_buf();
        info!("Executable directory: {}", exe_dir.display());

        // Get the path to the target directory (parent of the executable directory)
        // This logic might need adjustment depending on the final build structure.
        let target_dir = exe_dir.parent().ok_or_else(|| format!("Failed to get target directory from executable path: {}", exe_dir.display()))?.to_path_buf();
        info!("DEBUG: Base directory for vendor resolved at runtime: {:?}", target_dir);

        // Construct the path to the ComfyUI directory based on build mode
        let comfyui_dir = if cfg!(debug_assertions) {
            // --- DEBUG MODE ---
            // `cfg!(debug_assertions)` is true for debug builds (e.g., `npm run tauri dev` or `cargo build`).
            // In debug mode, assets copied by `build.rs` are located in the workspace's `target/debug/` directory.
            // `env!("CARGO_MANIFEST_DIR")` points to `metamorphosis-app/src-tauri`.
            // We navigate up one level to `metamorphosis-app/` and then into `target/debug/vendor/comfyui`.
            // This logic is specific to debug builds.
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("Failed to get parent of CARGO_MANIFEST_DIR for debug path construction")
                .join("target")
                .join("debug")
                .join("vendor")
                .join("comfyui")
        } else {
            // --- RELEASE MODE ---
            // `cfg!(debug_assertions)` is false for release builds (e.g., `cargo tauri build` or `cargo build --release`).
            // In release builds, assets are bundled with the application executable.
            // `target_dir` is derived from `current_exe()`'s parent, which should be the root of the bundled app resources.
            // The path becomes relative to the executable's location.
            // This depends on how `build.rs` and Tauri package the application.
            target_dir.join("vendor").join("comfyui")
        };

        // Construct the path to the Python executable within the virtual environment
        // The venv path is relative to the `comfyui_dir` which is already resolved for debug/release.
        let venv_dir = comfyui_dir.join(".venv");
        let venv_python_executable = if cfg!(target_os = "windows") {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python")
        };

        // Check if the constructed paths exist
        if !venv_python_executable.exists() {
            let err_msg = format!("Virtual environment Python executable not found at expected path: {}", venv_python_executable.display());
            error!("{}", err_msg);
            emit_backend_status(&app_handle, "backend_error", err_msg.clone(), true);
            return Err(err_msg);
        }
         if !comfyui_dir.exists() {
             let err_msg = format!("ComfyUI directory not found at expected path: {}", comfyui_dir.display());
             error!("{}", err_msg);
             emit_backend_status(&app_handle, "backend_error", err_msg.clone(), true);
             return Err(err_msg);
         }

        let main_script = comfyui_dir.join("main.py");

        if !main_script.exists() {
            let err_msg = format!("ComfyUI main.py not found at: {}", main_script.display());
            error!("{}", err_msg);
            emit_backend_status(&app_handle, "backend_error", err_msg.clone(), true);
            return Err(err_msg);
        }
        // Redundant check for comfyui_dir removed

        info!("Using Python executable: {}", venv_python_executable.display());
        info!("Using ComfyUI script: {}", main_script.display());
        info!("Setting CWD to: {}", comfyui_dir.display());

        // Get detailed GPU information
        let gpu_info = get_gpu_info();
        info!("Detected GPU Info: {:?}", gpu_info);

        let mut args = vec![
            main_script.to_string_lossy().into_owned(),
            "--listen".to_string(), // Add --listen argument
            "--front-end-version".to_string(), // Add frontend version argument
            "Comfy-Org/ComfyUI_frontend@v1.18.2".to_string(), // Specify frontend version v1.18.2
            "--port".to_string(), // Add --port argument
            COMFYUI_PORT.to_string(), // Specify the port
        ];

        // Determine whether to use --cpu flag based on detected GPU type
        let use_cpu = match gpu_info.gpu_type {
            GpuType::Nvidia => {
                // For NVIDIA, we assume GPU mode unless CUDA version detection failed or is not ideal.
                // For now, we'll launch in GPU mode if NVIDIA is detected.
                info!("NVIDIA GPU detected, launching in GPU mode.");
                false
            }
            GpuType::Amd => {
                // TODO: Implement logic for AMD GPUs. ComfyUI supports ROCm.
                info!("AMD GPU detected. Currently defaulting to CPU mode. ROCm support needs to be implemented.");
                true // Default to CPU for now
            }
            GpuType::Intel => {
                // TODO: Implement logic for Intel GPUs. ComfyUI supports Intel.
                info!("Intel GPU detected. Currently defaulting to CPU mode. Intel GPU support needs to be implemented.");
                true // Default to CPU for now
            }
            GpuType::Other | GpuType::Unknown => {
                info!("Other or Unknown GPU type detected, adding --cpu flag.");
                true // Default to CPU for unknown or other types
            }
        };

        if use_cpu {
            args.push("--cpu".to_string());
        }

        info!("ComfyUI args: {:?}", args);

        // Log the final command being executed
        let final_command = format!("{} {}", venv_python_executable.to_string_lossy(), args.join(" "));
        info!("Final ComfyUI launch command: {}", final_command);
        info!("Working Directory: {}", comfyui_dir.display());
        info!("Command path being used: {}", venv_python_executable.to_string_lossy());
        info!("Working directory being used: {}", comfyui_dir.display());

        info!("Attempting to spawn ComfyUI process...");
        let (mut rx, child) = match app_handle.shell().command(&venv_python_executable)
            .args(args.clone())
            .current_dir(&comfyui_dir) // Change working directory to comfyui_dir
            .spawn() {
                Ok((rx, child)) => {
                    info!("ComfyUI process started successfully (PID: {}).", child.pid());
                    info!("Successfully spawned ComfyUI process."); // Added log
                    (rx, child)
                },
                Err(e) => {
                    let err_msg = format!("Failed to spawn ComfyUI process: {}", e);
                    error!("{}", err_msg);
                    emit_backend_status(&app_handle, "backend_error", err_msg.clone(), true);
                    return Err(err_msg);
                }
            };

        // Asynchronous logging using CommandEvent
        let app_handle_clone_for_logs = app_handle.clone(); // Clone for the logging task
        async_runtime::spawn(async move {
            let app_handle = app_handle_clone_for_logs; // Use the cloned handle
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
                        // Update the global child process handle to None
                        let mut child_lock = COMFYUI_CHILD_PROCESS.lock().unwrap();
                        if child_lock.is_some() {
                            *child_lock = None;
                            info!("ComfyUI child process handle cleared on termination.");
                            // Optionally emit a status update about termination
                            // emit_backend_status(&app_handle, "sidecar_terminated", format!("ComfyUI process terminated: {:?}", exit_status), false);
                        }
                    }
                    _ => {} // Ignore other events for now
                }
            }
            info!("[ComfyUI] Output stream processing finished.");
        });


        // Store the child handle
        *COMFYUI_CHILD_PROCESS.lock().unwrap() = Some(child);
        info!("ComfyUI child process handle stored successfully.");

        // Emit status indicating the process has spawned, and now we will perform a health check.
        emit_backend_status(&app_handle, "sidecar_spawned_checking_health", "ComfyUI process spawned. Performing initial health check...".to_string(), false);

        // Perform initial health check
        let app_handle_clone_for_health_check = app_handle.clone();
        let health_check_fut = async move {
            if let Err(e) = perform_comfyui_health_check(app_handle_clone_for_health_check).await {
                error!("Initial ComfyUI health check failed: {}", e);
                // Error status should be emitted within perform_comfyui_health_check
            }
        };
        async_runtime::spawn(Box::pin(health_check_fut) as Pin<Box<dyn Future<Output = ()> + Send>>);

        Ok(()) // Return Ok on successful spawn, health check is async
    })
}

// New function to perform initial health check after spawning
async fn perform_comfyui_health_check(app_handle: AppHandle<Wry>) -> Result<(), String> {
    info!("Performing initial ComfyUI health check...");
    let health_check_url = format!("http://localhost:{}/queue", COMFYUI_PORT); // or /object_info
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10)) // Add a timeout for the request
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let max_retries = 10; // e.g., 10 retries
    let retry_delay = Duration::from_secs(5); // e.g., 5 seconds delay

    for attempt in 1..=max_retries {
        info!("Health check attempt {}/{} to {}", attempt, max_retries, health_check_url);
        match client.get(&health_check_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("ComfyUI initial health check successful.");
                    if let Err(e) = app_handle.emit("comfyui-fully-healthy", ()) {
                         error!("Failed to emit comfyui-fully-healthy event: {}", e);
                    }
                    emit_backend_status(&app_handle, "backend_ready", "ComfyUI backend is fully operational.".to_string(), false);

                    // Start long-term health monitoring only after initial check passes
                    info!("Attempting to start long-term ComfyUI health monitor...");
                    let app_handle_for_monitor = app_handle.clone();
                    let monitor_fut = monitor_comfyui_health(app_handle_for_monitor);
                    async_runtime::spawn(Box::pin(monitor_fut) as Pin<Box<dyn Future<Output = ()> + Send>>);
                    info!("Long-term ComfyUI health monitor spawn initiated.");
                    return Ok(());
                } else {
                    error!("Health check attempt {} failed: Status {}", attempt, response.status());
                }
            }
            Err(e) => {
                error!("Health check attempt {} failed: Error {}", attempt, e);
            }
        }
        if attempt < max_retries {
            tokio::time::sleep(retry_delay).await;
        }
    }

    let err_msg = "ComfyUI failed initial health check after multiple attempts.".to_string();
    error!("{}", err_msg);
    emit_backend_status(&app_handle, "backend_error", err_msg.clone(), true);
    stop_comfyui_sidecar(); // Attempt to clean up
    Err(err_msg)
}


// Function to monitor the health of the ComfyUI sidecar
async fn monitor_comfyui_health(app_handle: AppHandle<Wry>) {
    let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds
    info!("Starting ComfyUI health monitoring...");
    tokio::time::sleep(Duration::from_secs(45)).await; // Initial delay before first health check
    info!("Initial delay complete, starting periodic health checks.");

    loop {
        interval.tick().await; // Wait for the next tick

        // Check if the process is running within a block to manage lock scope
        let is_running = {
            let child_process_guard = COMFYUI_CHILD_PROCESS.lock().unwrap();
            child_process_guard.is_some()
        }; // The lock is dropped here as child_process_guard goes out of scope

        if !is_running {
            info!("ComfyUI process is not running, attempting to restart...");
            // Check restart limits before attempting
            let mut attempts_lock = RESTART_ATTEMPTS.lock().unwrap();
            let mut last_restart_lock = LAST_RESTART_TIME.lock().unwrap();
            let now = Instant::now();
            if let Some(last_time) = *last_restart_lock {
                if now.duration_since(last_time) > Duration::from_secs(3600) {
                    *attempts_lock = 0; // Reset after an hour
                }
            }
            if *attempts_lock < MAX_RESTARTS_PER_HOUR {
                *attempts_lock += 1;
                *last_restart_lock = Some(now);
                let attempt_count = *attempts_lock;
                // Drop locks before await
                drop(attempts_lock);
                drop(last_restart_lock);

                info!("Restart attempt #{}", attempt_count);
                let app_handle_clone = app_handle.clone();
                // Spawn the restart attempt
                let fut = async move {
                    let app_handle_for_spawn = app_handle_clone.clone(); // Clone for spawn_comfyui_process
                    if let Err(e) = spawn_comfyui_process(app_handle_for_spawn).await {
                        error!("Restart attempt failed: {}", e);
                        // Optionally emit error status
                        // Use the original app_handle_clone for emit_backend_status as it's still valid in this scope
                        emit_backend_status(&app_handle_clone, "backend_error", format!("Restart attempt failed: {}", e), true);
                    }
                };
                async_runtime::spawn(Box::pin(fut) as Pin<Box<dyn Future<Output = ()> + Send>>);
            } else {
                error!("Maximum restart attempts reached. Not restarting ComfyUI.");
                // Optionally emit a persistent error status
                emit_backend_status(&app_handle, "backend_error", "Maximum restart attempts reached.".to_string(), true);
            }
            continue; // Skip health check for this interval
        }

        // Perform a simple HTTP GET request to the ComfyUI API endpoint using reqwest::Client
        let health_url = format!("http://localhost:{}/queue", COMFYUI_PORT); // A simple endpoint to check responsiveness
        let client = reqwest::Client::new();
        match client.get(&health_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("ComfyUI health check successful.");
                    // Reset restart attempts on successful health check after a failure
                    let mut attempts = RESTART_ATTEMPTS.lock().unwrap();
                    if *attempts > 0 {
                         info!("ComfyUI is healthy, resetting restart attempts.");
                        *attempts = 0;
                    }
                } else {
                    error!("ComfyUI health check failed: Received non-success status code: {}", response.status());
                    // Process might be running but unresponsive, attempt restart (logic moved to !is_running check)
                    error!("ComfyUI health check failed: Received non-success status code: {}", response.status());
                    // Consider killing the unresponsive process before restarting
                    stop_comfyui_sidecar(); // Attempt to kill the potentially hung process
                    // Restart logic will be handled by the next !is_running check
                }
            }
            Err(e) => {
                 let error_kind = if e.is_connect() { "Connection" }
                                else if e.is_timeout() { "Timeout" }
                                else if e.is_request() { "Request" }
                                else { "Other" };
                 let mut error_msg = format!("ComfyUI health check failed: {} error: {}", error_kind, e);
                 if let Some(source) = e.source() {
                     error_msg.push_str(&format!(" Source: {}", source));
                 }
                 error!("{}", error_msg);

                // Process is likely crashed or not listening, attempt restart (logic moved to !is_running check)
                // Ensure the process handle is cleared if it's likely crashed
                stop_comfyui_sidecar(); // Attempt to kill and clear handle
            }
        }
    }
}

// New command to ensure dependencies are installed and sidecar is started
#[tauri::command]
pub async fn ensure_backend_ready(app_handle: AppHandle<Wry>) -> Result<(), String> {
    log::error!("[EARLY_CALL_DEBUG] ensure_backend_ready INVOKED"); // Added for debugging
    info!("Ensuring backend is ready...");
    emit_backend_status(&app_handle, "checking_dependencies", "Checking backend dependencies...".to_string(), false);

    // 1. Install Dependencies
    match dependency_management::install_python_dependencies_with_progress(&app_handle).await {
        Ok(_) => {
            info!("Dependency check/installation complete.");
            // Don't emit success here, wait for sidecar start
        }
        Err(e) => {
            let err_msg = format!("Failed to install Python dependencies: {}", e);
            error!("{}", err_msg);
            emit_backend_status(&app_handle, "backend_error", err_msg.clone(), true);
            return Err(err_msg);
        }
    }

    // 2. Start Sidecar Process (using the refactored internal function)
    emit_backend_status(&app_handle, "starting_sidecar", "Starting ComfyUI backend...".to_string(), false);
    match spawn_comfyui_process(app_handle.clone()).await { // Clone app_handle here
        Ok(_) => {
            info!("ComfyUI sidecar process spawned successfully via ensure_backend_ready. Initial health check initiated.");
            // Health monitor is now started by perform_comfyui_health_check upon success.
            // The final "backend_ready" or "backend_error" status will be emitted by perform_comfyui_health_check.
            Ok(())
        }
        Err(e) => {
             let err_msg = format!("Failed to start ComfyUI sidecar process (spawn_comfyui_process failed): {}", e);
             error!("{}", err_msg);
             // Error status should have been emitted by spawn_comfyui_process
             Err(err_msg)
        }
    }
}

/// Spawns the ComfyUI process and performs an initial health check.
/// Emits `setup-progress` events.
pub async fn spawn_and_health_check_comfyui(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    log::error!("[EARLY_SPAWN_DEBUG] INTERNAL spawn_and_health_check_comfyui INVOKED");
    // Guard to prevent multiple concurrent spawns
    if COMFYUI_CHILD_PROCESS.lock().unwrap().is_some() {
        info!("[GUARD] spawn_and_health_check_comfyui: ComfyUI process is already considered active or starting. Skipping new spawn attempt.");
        // We still need to emit a success or ready status if it's already running and healthy.
        // For now, just returning Ok, assuming the existing process is being monitored.
        // This might need refinement to ensure the frontend gets the correct signals.
        // Let's emit a specific status for this case.
        emit_backend_status(app_handle, "already_running_quick_check", "ComfyUI appears to be already running.".to_string(), false);
        // Potentially, we could try a quick health check here without a full spawn.
        // For simplicity now, we assume if it's Some(), it's being handled.
        return Ok(());
    }
    let phase_name = "finalizing"; // Or "starting_services"
    let mut current_phase_progress: u8 = 0;

    info!("Attempting to spawn ComfyUI process for setup screen flow...");
    setup::emit_setup_progress(app_handle, phase_name, "Preparing to start ComfyUI services...", current_phase_progress, None, None);
    
    // --- Logic adapted from spawn_comfyui_process ---
    let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get current executable path: {}", e))?;
    let exe_dir = exe_path.parent().ok_or_else(|| format!("Failed to get parent directory of executable: {}", exe_path.display()))?.to_path_buf();
    let target_dir = exe_dir.parent().ok_or_else(|| format!("Failed to get target directory from executable path: {}", exe_dir.display()))?.to_path_buf();

    let comfyui_dir = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().expect("Failed to get parent of CARGO_MANIFEST_DIR").join("target").join("debug").join("vendor").join("comfyui")
    } else {
        target_dir.join("vendor").join("comfyui")
    };
    let venv_dir = comfyui_dir.join(".venv");
    let venv_python_executable = if cfg!(target_os = "windows") { venv_dir.join("Scripts").join("python.exe") } else { venv_dir.join("bin").join("python") };
    let main_script = comfyui_dir.join("main.py");

    if !venv_python_executable.exists() { /* ... error handling ... */
        let err_msg = format!("Venv Python not found for setup: {}", venv_python_executable.display());
        setup::emit_setup_progress(app_handle, "error", "Venv Python Missing", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
        return Err(err_msg);
    }
    if !main_script.exists() { /* ... error handling ... */
        let err_msg = format!("ComfyUI main.py not found for setup: {}", main_script.display());
        setup::emit_setup_progress(app_handle, "error", "ComfyUI main.py Missing", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
        return Err(err_msg);
    }

    let gpu_info = get_gpu_info();
    let mut args = vec![
        main_script.to_string_lossy().into_owned(),
        "--listen".to_string(),
        "--front-end-version".to_string(), "Comfy-Org/ComfyUI_frontend@v1.18.2".to_string(),
        "--port".to_string(), COMFYUI_PORT.to_string(),
    ];
    if matches!(gpu_info.gpu_type, GpuType::Amd | GpuType::Intel | GpuType::Other | GpuType::Unknown) { args.push("--cpu".to_string()); }

    current_phase_progress = 10; // Progress after path checks and arg setup
    setup::emit_setup_progress(app_handle, phase_name, "Spawning ComfyUI process...", current_phase_progress, Some(format!("Args: {:?}", args)), None);

    let (mut rx, child) = match app_handle.shell().command(&venv_python_executable).args(args.clone()).current_dir(&comfyui_dir).spawn() {
        Ok((rx, child)) => {
            info!("ComfyUI process started for setup (PID: {}).", child.pid());
            current_phase_progress = 30; // Progress after successful spawn
            setup::emit_setup_progress(app_handle, phase_name, "ComfyUI process spawned.", current_phase_progress, None, None);
            (rx, child)
        },
        Err(e) => {
            let err_msg = format!("Failed to spawn ComfyUI process for setup: {}", e);
            error!("{}", err_msg);
            setup::emit_setup_progress(app_handle, "error", "ComfyUI Spawn Failed", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
            return Err(err_msg);
        }
    };
    
    // Simplified async logging for this flow
    let app_handle_clone_for_logs = app_handle.clone();
    async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => info!("[ComfyUI Setup stdout] {}", String::from_utf8_lossy(&line)),
                CommandEvent::Stderr(line) => {
                    let err_line = String::from_utf8_lossy(&line).to_string();
                    error!("[ComfyUI Setup stderr] {}", err_line);
                    // Optionally emit stderr as a detail message for the current step
                    // setup::emit_setup_progress(&app_handle_clone_for_logs, phase_name, "ComfyUI Process Output", current_phase_progress, Some(err_line.clone()), Some(err_line));
                }
                CommandEvent::Terminated(status) => {
                    info!("[ComfyUI Setup] Process terminated with status: {:?}", status);
                    let mut child_lock = COMFYUI_CHILD_PROCESS.lock().unwrap();
                    if child_lock.is_some() { *child_lock = None; }
                }
                _ => {}
            }
        }
    });
    *COMFYUI_CHILD_PROCESS.lock().unwrap() = Some(child);

    // --- Logic adapted from perform_comfyui_health_check ---
    info!("Performing initial ComfyUI health check (setup flow)...");
    current_phase_progress = 40;
    setup::emit_setup_progress(app_handle, phase_name, "Performing ComfyUI health check...", current_phase_progress, None, None);
    
    let health_check_url = format!("http://localhost:{}/queue", COMFYUI_PORT);
    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    let max_retries = 10;
    let retry_delay = Duration::from_secs(5);

    for attempt in 1..=max_retries {
        let attempt_msg = format!("Health check attempt {}/{} to {}", attempt, max_retries, health_check_url);
        info!("{}", attempt_msg);
        // Calculate progress more safely to avoid overflow
        let base_progress = current_phase_progress as u32;
        let target_progress_cap = 90_u32;
        let remaining_progress_span = target_progress_cap.saturating_sub(base_progress);
        let progress_increment = (attempt as u32 * remaining_progress_span) / (max_retries as u32);
        let progress_for_attempt = (base_progress + progress_increment).min(100) as u8; // Ensure it doesn't exceed 100

        setup::emit_setup_progress(app_handle, phase_name, &attempt_msg, progress_for_attempt, None, None);

        match client.get(&health_check_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("ComfyUI initial health check successful (setup flow).");
                    setup::emit_setup_progress(app_handle, phase_name, "ComfyUI health check successful.", 100, None, None);
                    
                    // Start long-term health monitoring (using original monitor_comfyui_health)
                    let app_handle_for_monitor = app_handle.clone();
                    async_runtime::spawn(monitor_comfyui_health(app_handle_for_monitor));
                    return Ok(());
                } else {
                    error!("Health check (setup flow) attempt {} failed: Status {}", attempt, response.status());
                }
            }
            Err(e) => {
                error!("Health check (setup flow) attempt {} failed: Error {}", attempt, e);
            }
        }
        if attempt < max_retries { tokio::time::sleep(retry_delay).await; }
    }

    let err_msg = "ComfyUI failed initial health check after multiple attempts (setup flow).".to_string();
    error!("{}", err_msg);
    setup::emit_setup_progress(app_handle, "error", "ComfyUI Health Check Failed", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
    stop_comfyui_sidecar();
    Err(err_msg)
}

#[tauri::command]
pub async fn ensure_comfyui_running_and_healthy(app_handle: AppHandle<Wry>) -> Result<(), String> {
    log::error!("[EARLY_CALL_DEBUG] ensure_comfyui_running_and_healthy INVOKED"); // Added for debugging
    info!("[COMFYUI LIFECYCLE] ensure_comfyui_running_and_healthy called.");

    // Check if a process is already believed to be running or starting
    if COMFYUI_CHILD_PROCESS.lock().unwrap().is_some() {
        info!("[COMFYUI LIFECYCLE] ComfyUI process is already considered active or starting. Skipping new spawn attempt in ensure_comfyui_running_and_healthy.");
        // Assuming that if a child process handle exists, its startup and health are being managed.
        // If it's healthy, the frontend will eventually get the 'backend_ready' or 'comfyui-fully-healthy' event.
        // If it fails, the monitor or initial health check should handle emitting an error.
        return Ok(());
    }
    
    // We expect dependencies to be installed at this point.
    // This function will attempt to spawn and then health check.
    // It will emit 'backend-status' with 'sidecar_spawned_checking_health', then 'backend_ready' or 'backend_error'.
    // It will also emit 'comfyui-fully-healthy' on success.
    emit_backend_status(&app_handle, "starting_services", "Ensuring ComfyUI backend is running and healthy (ensure_comfyui_running_and_healthy)...".to_string(), false); // Added explicit emit
    match spawn_and_health_check_comfyui(&app_handle).await {
        Ok(_) => {
            info!("[COMFYUI LIFECYCLE] ComfyUI started and reported healthy by spawn_and_health_check_comfyui.");
            // Final success status is emitted by spawn_and_health_check_comfyui itself.
            Ok(())
        }
        Err(e) => {
            error!("[COMFYUI LIFECYCLE] Failed to ensure ComfyUI is running and healthy: {}", e);
            // Error status is emitted by spawn_and_health_check_comfyui itself.
            Err(e)
        }
    }
}

// Function to stop the sidecar
pub fn stop_comfyui_sidecar() {
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