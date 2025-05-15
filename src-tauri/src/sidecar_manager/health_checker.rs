// metamorphosis-app/src-tauri/src/sidecar_manager/health_checker.rs

use tauri::{AppHandle, Manager, Wry, async_runtime, Emitter};
use tauri_plugin_http::reqwest;
use tokio::time::{interval, Duration};
use log::{info, error};
use std::time::Instant;
use std::pin::Pin;
use std::future::Future;
use std::error::Error as StdError; // Alias to avoid conflict

// Internal imports from sibling modules
use super::event_utils::{emit_backend_status, COMFYUI_PORT};
use super::process_handler::{
    COMFYUI_CHILD_PROCESS, RESTART_ATTEMPTS, LAST_RESTART_TIME, MAX_RESTARTS_PER_HOUR,
    stop_comfyui_sidecar, spawn_comfyui_process,
};

// Function to perform initial health check after spawning
// This function is intended for internal use by orchestration functions.
pub(super) async fn perform_comfyui_health_check(app_handle: AppHandle<Wry>) -> Result<(), String> {
    info!("Performing initial ComfyUI health check...");
    let health_check_url = format!("http://localhost:{}/queue", COMFYUI_PORT);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let max_retries = 10;
    let retry_delay = Duration::from_secs(5);

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
    stop_comfyui_sidecar(); 
    Err(err_msg)
}

// Function to monitor the health of the ComfyUI sidecar
// This function is intended for internal use, typically started by perform_comfyui_health_check.
pub async fn monitor_comfyui_health(app_handle: AppHandle<Wry>) {
    let mut interval = interval(Duration::from_secs(30)); 
    info!("Starting ComfyUI health monitoring...");
    tokio::time::sleep(Duration::from_secs(45)).await; 
    info!("Initial delay complete, starting periodic health checks.");

    loop {
        interval.tick().await; 

        let is_running = {
            let child_process_guard = COMFYUI_CHILD_PROCESS.lock().unwrap();
            child_process_guard.is_some()
        }; 

        if !is_running {
            info!("ComfyUI process is not running, attempting to restart...");
            let mut attempts_lock = RESTART_ATTEMPTS.lock().unwrap();
            let mut last_restart_lock = LAST_RESTART_TIME.lock().unwrap();
            let now = Instant::now();
            if let Some(last_time) = *last_restart_lock {
                if now.duration_since(last_time) > Duration::from_secs(3600) {
                    *attempts_lock = 0; 
                }
            }
            if *attempts_lock < MAX_RESTARTS_PER_HOUR {
                *attempts_lock += 1;
                *last_restart_lock = Some(now);
                let attempt_count = *attempts_lock;
                drop(attempts_lock);
                drop(last_restart_lock);

                info!("Restart attempt #{}", attempt_count);
                let app_handle_clone = app_handle.clone();
                let fut = async move {
                    let app_handle_for_spawn = app_handle_clone.clone(); 
                    if let Err(e) = spawn_comfyui_process(app_handle_for_spawn).await { // Assuming spawn_comfyui_process is now in process_handler
                        error!("Restart attempt failed: {}", e);
                        emit_backend_status(&app_handle_clone, "backend_error", format!("Restart attempt failed: {}", e), true);
                    } else {
                        // If spawn_comfyui_process succeeded, it would have called perform_comfyui_health_check
                        // which in turn would restart this monitor if healthy.
                        // So, no need to directly call perform_comfyui_health_check here.
                        // The new spawned process will have its own initial health check.
                        info!("Spawn attempt initiated for restart. Initial health check will follow from new process.");
                    }
                };
                async_runtime::spawn(Box::pin(fut) as Pin<Box<dyn Future<Output = ()> + Send>>);
            } else {
                error!("Maximum restart attempts reached. Not restarting ComfyUI.");
                emit_backend_status(&app_handle, "backend_error", "Maximum restart attempts reached.".to_string(), true);
            }
            continue; 
        }

        let health_url = format!("http://localhost:{}/queue", COMFYUI_PORT); 
        let client = reqwest::Client::new();
        match client.get(&health_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("ComfyUI health check successful.");
                    let mut attempts = RESTART_ATTEMPTS.lock().unwrap();
                    if *attempts > 0 {
                         info!("ComfyUI is healthy, resetting restart attempts.");
                        *attempts = 0;
                    }
                } else {
                    error!("ComfyUI health check failed: Received non-success status code: {}", response.status());
                    stop_comfyui_sidecar(); 
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
                stop_comfyui_sidecar(); 
            }
        }
    }
}