// metamorphosis-app/src-tauri/src/sidecar_manager/orchestration.rs

use tauri::{AppHandle, Wry, Emitter, Manager};
use log::{info, error};
use tokio::time::Duration; // For port check delay
use tauri_plugin_http::reqwest; // For port check client
// use std::path::PathBuf; // For path construction during spawn_and_health_check_comfyui - PathBuf is used by internal_spawn_comfyui_process

// Internal imports from sibling modules
use super::event_utils::{emit_backend_status, COMFYUI_PORT};
use crate::process_manager::ProcessManager;
use super::process_handler::{spawn_comfyui_process as internal_spawn_comfyui_process, IS_ATTEMPTING_SPAWN};
use super::health_checker::{perform_comfyui_health_check, monitor_comfyui_health}; // monitor_comfyui_health is started by perform_comfyui_health_check

// Crate-level imports
use crate::setup_manager::dependency_manager; // For install_python_dependencies_with_progress
use crate::setup; // For emit_setup_progress

// Tauri command to ensure dependencies are installed and sidecar is started
#[tauri::command]
pub async fn ensure_backend_ready(app_handle: AppHandle<Wry>) -> Result<(), String> {
    log::error!("[EARLY_CALL_DEBUG] ensure_backend_ready INVOKED");
    info!("Ensuring backend is ready...");
    emit_backend_status(&app_handle, "checking_dependencies", "Checking backend dependencies...".to_string(), false);
 
    match dependency_manager::install_python_dependencies_with_progress(&app_handle).await {
        Ok(_) => {
            info!("Dependency check/installation complete.");
        }
        Err(e) => {
            let err_msg = format!("Failed to install Python dependencies: {}", e);
            error!("{}", err_msg);
            emit_backend_status(&app_handle, "backend_error", err_msg.clone(), true);
            return Err(err_msg);
        }
    }

    emit_backend_status(&app_handle, "starting_sidecar", "Starting ComfyUI backend...".to_string(), false);
    match internal_spawn_comfyui_process(app_handle.clone()).await {
        Ok(_) => {
            info!("ComfyUI sidecar process spawned successfully via ensure_backend_ready. Initial health check will be initiated by internal_spawn_comfyui_process.");
            // perform_comfyui_health_check is now called internally by internal_spawn_comfyui_process (or rather, the new structure will be that internal_spawn just spawns, and this orchestrator calls health_check)
            // For now, let's assume internal_spawn_comfyui_process handles its own health check trigger.
            // Correction: internal_spawn_comfyui_process should just spawn. Health check is separate.
            // The old `spawn_comfyui_process` used to call `perform_comfyui_health_check`.
            // The new `internal_spawn_comfyui_process` in `process_handler` does NOT call it.
            // So, we must call it here.

            // After successful spawn, initiate the health check.
            if let Err(e) = perform_comfyui_health_check(app_handle.clone()).await {
                 error!("Initial ComfyUI health check failed after spawn in ensure_backend_ready: {}", e);
                 // Error status should be emitted within perform_comfyui_health_check
                 return Err(format!("ComfyUI health check failed: {}", e));
            }
            // If health check is successful, it emits "backend_ready" and starts long-term monitoring.
            Ok(())
        }
        Err(e) => {
             let err_msg = format!("Failed to start ComfyUI sidecar process (internal_spawn_comfyui_process failed): {}", e);
             error!("{}", err_msg);
             // Error status should have been emitted by internal_spawn_comfyui_process
             Err(err_msg)
        }
    }
}

/// Spawns the ComfyUI process and performs an initial health check.
/// Emits `setup-progress` events. This is typically called by `setup.rs`.
pub async fn spawn_and_health_check_comfyui(app_handle: &AppHandle<Wry>) -> Result<(), String> {
    log::error!("[EARLY_SPAWN_DEBUG] Orchestration spawn_and_health_check_comfyui INVOKED");
    // Initial check for existing process or ongoing spawn attempt
    // Check 1: Is another spawn attempt already in progress?
    // This block ensures the MutexGuard for IS_ATTEMPTING_SPAWN is short-lived.
    {
        let mut is_attempting_spawn_guard = IS_ATTEMPTING_SPAWN.lock().unwrap();
        if *is_attempting_spawn_guard {
            info!("[GUARD] spawn_and_health_check_comfyui: Another spawn attempt is already in progress. Skipping.");
            drop(is_attempting_spawn_guard); // Explicitly drop guard
            let err_msg = "Spawn attempt already in progress by another call.".to_string();
            emit_backend_status(app_handle, "backend_error", err_msg.clone(), true);
            setup::emit_setup_progress(app_handle, "error", "Concurrent Spawn Attempt", 0, Some(err_msg.clone()), Some(err_msg.clone()));
            return Err(err_msg);
        }
        // If no other attempt is in progress, mark this one as started.
        // The guard is released at the end of this block. IS_ATTEMPTING_SPAWN remains true.
        *is_attempting_spawn_guard = true;
        info!("[GUARD] spawn_and_health_check_comfyui: Marked IS_ATTEMPTING_SPAWN = true.");
    } // is_attempting_spawn_guard is dropped here.

    // This scopeguard will reset IS_ATTEMPTING_SPAWN to false when the function exits.
    let _reset_spawning_flag_guard = scopeguard::guard((), |_| {
        let mut guard = IS_ATTEMPTING_SPAWN.lock().unwrap();
        if *guard { // Only reset if it's still true
             *guard = false;
             info!("[GUARD_CLEANUP] spawn_and_health_check_comfyui: IS_ATTEMPTING_SPAWN flag reset to false.");
        } else {
             info!("[GUARD_CLEANUP] spawn_and_health_check_comfyui: IS_ATTEMPTING_SPAWN was already false. No change made by scopeguard.");
        }
    });

    // Check 2: Is ComfyUI process already active?
    let process_manager = app_handle.state::<ProcessManager>();
    if process_manager.is_process_running("comfyui_sidecar") {
        info!("[GUARD] spawn_and_health_check_comfyui: ComfyUI process is already active (checked after IS_ATTEMPTING_SPAWN logic). Performing health check.");
        emit_backend_status(app_handle, "already_running_quick_check", "ComfyUI appears to be already running. Verifying health...".to_string(), false);
        
        // Perform health check. No MutexGuard from IS_ATTEMPTING_SPAWN is held here.
        if let Err(e) = perform_comfyui_health_check(app_handle.clone()).await {
            error!("Health check for already running process failed: {}", e);
            // _reset_spawning_flag_guard will run here, setting IS_ATTEMPTING_SPAWN to false.
            return Err(format!("Health check for existing process failed: {}", e));
        }
        // If health check passes, _reset_spawning_flag_guard will run.
        return Ok(());
    }
    
    info!("[GUARD] Proceeding with port check and spawn logic as no existing process found and IS_ATTEMPTING_SPAWN is now true.");

    // Port availability check
    let max_port_check_retries = 3;
    let port_check_delay = Duration::from_secs(3);
    for attempt in 1..=max_port_check_retries {
        match std::net::TcpListener::bind(format!("0.0.0.0:{}", COMFYUI_PORT)) {
            Ok(listener) => {
                drop(listener);
                info!("[PORT CHECK] Attempt {}: Port {} is available.", attempt, COMFYUI_PORT);
                break;
            }
            Err(e) => {
                let port_busy_msg = format!("[PORT CHECK] Attempt {}: Port {} is already in use: {}.", attempt, COMFYUI_PORT, e);
                error!("{}", port_busy_msg);
                if attempt == max_port_check_retries {
                    emit_backend_status(app_handle, "backend_error", port_busy_msg.clone(), true);
                    setup::emit_setup_progress(app_handle, "error_port_conflict", "Port Conflict", 0, Some(port_busy_msg.clone()), Some(port_busy_msg.clone()));
                    return Err(port_busy_msg);
                }
                let retry_detail_msg = format!("Port {} busy. Retrying in {}s... (Attempt {}/{})", COMFYUI_PORT, port_check_delay.as_secs(), attempt, max_port_check_retries);
                info!("{}", retry_detail_msg);
                emit_backend_status(app_handle, "waiting_for_port", retry_detail_msg.clone(), false);
                setup::emit_setup_progress(app_handle, "port_check_retry", "Waiting for Port", 1, Some(retry_detail_msg), None);
                tokio::time::sleep(port_check_delay).await;
            }
        }
    }

    let phase_name = "starting_services";
    let mut current_phase_progress: u8 = 0;
    setup::emit_setup_progress(app_handle, phase_name, "Preparing to start ComfyUI services...", current_phase_progress, None, None);

    // Spawn the process using the internal handler
    match internal_spawn_comfyui_process(app_handle.clone()).await {
        Ok(_) => {
            info!("ComfyUI process spawned successfully by internal_spawn_comfyui_process.");
            current_phase_progress = 30;
            setup::emit_setup_progress(app_handle, phase_name, "ComfyUI process spawned.", current_phase_progress, None, None);
        },
        Err(e) => {
            let err_msg = format!("Failed to spawn ComfyUI process via internal_spawn_comfyui_process: {}", e);
            error!("{}", err_msg);
            setup::emit_setup_progress(app_handle, "error", "ComfyUI Spawn Failed", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
            return Err(err_msg);
        }
    };
    
    // Perform health check
    info!("Performing initial ComfyUI health check (setup flow)...");
    current_phase_progress = 40; // Progress before health check starts
    setup::emit_setup_progress(app_handle, phase_name, "Performing ComfyUI health check...", current_phase_progress, None, None);

    // Use a simplified health check loop here for setup progress reporting
    let health_check_url = format!("http://localhost:{}/queue", COMFYUI_PORT);
    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    let max_health_retries = 10;
    let health_retry_delay = Duration::from_secs(5);

    for attempt in 1..=max_health_retries {
        let attempt_msg = format!("Health check attempt {}/{} to {}", attempt, max_health_retries, health_check_url);
        info!("{}", attempt_msg);
        
        let base_progress = current_phase_progress as u32; // Should be 40
        let target_progress_cap = 90_u32; // Health check occupies 40-90%
        let remaining_progress_span = target_progress_cap.saturating_sub(base_progress);
        let progress_increment = (attempt as u32 * remaining_progress_span) / (max_health_retries as u32);
        let progress_for_attempt = (base_progress + progress_increment).min(100) as u8;

        setup::emit_setup_progress(app_handle, phase_name, &attempt_msg, progress_for_attempt, None, None);

        match client.get(&health_check_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("ComfyUI initial health check successful (setup flow).");
                    setup::emit_setup_progress(app_handle, phase_name, "ComfyUI health check successful.", 100, None, None);
                    if let Err(e) = app_handle.emit("comfyui-fully-healthy", ()) {
                         error!("[SPAWN_AND_HEALTH_CHECK] Failed to emit comfyui-fully-healthy event: {}", e);
                    }
                    emit_backend_status(app_handle, "backend_ready", "ComfyUI backend is fully operational (spawn_and_health_check).".to_string(), false);
                    info!("[SPAWN_AND_HEALTH_CHECK] Emitted comfyui-fully-healthy and backend_ready.");
                    
                    let app_handle_for_monitor = app_handle.clone();
                    tauri::async_runtime::spawn(monitor_comfyui_health(app_handle_for_monitor)); // monitor_comfyui_health is in health_checker
                    return Ok(());
                } else {
                    error!("Health check (setup flow) attempt {} failed: Status {}", attempt, response.status());
                }
            }
            Err(e) => {
                error!("Health check (setup flow) attempt {} failed: Error {}", attempt, e);
            }
        }
        if attempt < max_health_retries { tokio::time::sleep(health_retry_delay).await; }
    }

    let err_msg = "ComfyUI failed initial health check after multiple attempts (setup flow).".to_string();
    error!("{}", err_msg);
    setup::emit_setup_progress(app_handle, "error", "ComfyUI Health Check Failed", current_phase_progress, Some(err_msg.clone()), Some(err_msg.clone()));
    let process_manager = app_handle.state::<ProcessManager>();
    process_manager.stop_process("comfyui_sidecar");
    Err(err_msg)
}

#[tauri::command]
pub async fn ensure_comfyui_running_and_healthy(app_handle: AppHandle<Wry>) -> Result<(), String> {
    log::error!("[EARLY_CALL_DEBUG] ensure_comfyui_running_and_healthy INVOKED");
    info!("[COMFYUI LIFECYCLE] ensure_comfyui_running_and_healthy called.");

    let process_manager = app_handle.state::<ProcessManager>();
    if process_manager.is_process_running("comfyui_sidecar") {
        info!("[COMFYUI LIFECYCLE] ComfyUI process is already considered active or starting. Attempting health check.");
        // If it's already running, perform a health check.
        // perform_comfyui_health_check will emit appropriate statuses.
        return perform_comfyui_health_check(app_handle.clone()).await;
    }
    
    emit_backend_status(&app_handle, "starting_services", "Ensuring ComfyUI backend is running and healthy (ensure_comfyui_running_and_healthy)...".to_string(), false);
    // This function is similar to spawn_and_health_check_comfyui but without the setup progress events.
    // It's for cases where the frontend just needs to ensure the backend is up, not during initial setup.
    // For simplicity, it can call spawn_and_health_check_comfyui, which handles the IS_ATTEMPTING_SPAWN guard.
    // The setup progress events from spawn_and_health_check_comfyui might be redundant here if not in setup phase.
    // However, spawn_and_health_check_comfyui is robust.
    match spawn_and_health_check_comfyui(&app_handle).await {
        Ok(_) => {
            info!("[COMFYUI LIFECYCLE] ComfyUI started and reported healthy by spawn_and_health_check_comfyui.");
            Ok(())
        }
        Err(e) => {
            error!("[COMFYUI LIFECYCLE] Failed to ensure ComfyUI is running and healthy: {}", e);
            Err(e)
        }
    }
}