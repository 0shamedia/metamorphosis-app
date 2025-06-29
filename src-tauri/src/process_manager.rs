use log::{error, info, warn};
use tauri_plugin_shell::process::{Command, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_shell::process::CommandChild;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use uuid::Uuid;

/// Manages all active child processes spawned by the application.
/// This struct is managed by Tauri's state system, ensuring that it is accessible
/// from anywhere in the application and that its lifecycle is tied to the app itself.
pub struct ProcessManager {
    pub active_processes: Mutex<HashMap<String, CommandChild>>,
}

/// Represents the final result of a managed command execution.
pub struct CommandResult {
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}


impl ProcessManager {
    pub fn is_process_running(&self, process_name: &str) -> bool {
        self.active_processes.lock().unwrap().contains_key(process_name)
    }

    pub fn stop_process(&self, process_name: &str) {
        if let Some(child) = self.active_processes.lock().unwrap().remove(process_name) {
            info!("Attempting to kill process: {}", process_name);
            match child.kill() {
                Ok(_) => info!("Successfully sent kill signal to process: {}", process_name),
                Err(e) => error!("Failed to kill process '{}': {}", process_name, e),
            }
        }
    }
    /// Creates a new, empty ProcessManager.
    pub fn new() -> Self {
        Self {
            active_processes: Mutex::new(HashMap::new()),
        }
    }

    /// Spawns a new managed process that runs in the background.
    /// This is the standard "fire-and-forget" method for processes like the main sidecar.
    pub async fn spawn_managed_process(
        app_handle: &AppHandle<Wry>,
        process_name: String,
        command: Command,
    ) -> Result<(), String> {
        info!("Spawning background managed process: {}", process_name);
        let process_manager = app_handle.state::<ProcessManager>();
        let (mut rx, child) = command.spawn().map_err(|e| {
            let err_msg = format!("Failed to spawn managed process '{}': {}", process_name, e);
            error!("{}", err_msg);
            err_msg
        })?;

        // Add the child to the process manager
        process_manager
            .active_processes
            .lock()
            .unwrap()
            .insert(process_name.clone(), child);

        let handle = app_handle.clone();
        let name = process_name.clone();

        // Spawn a task to monitor the process
        tauri::async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Terminated(payload) => {
                        info!(
                            "Managed process '{}' terminated with code: {:?}, signal: {:?}",
                            name, payload.code, payload.signal
                        );
                        break; // Exit loop on termination
                    }
                    CommandEvent::Error(e) => {
                        error!("Error from managed process '{}': {}", name, e);
                    }
                    CommandEvent::Stdout(line) => {
                        info!("[{}_stdout] {}", name, String::from_utf8_lossy(&line));
                    }
                    CommandEvent::Stderr(line) => {
                        // Stderr should probably be at a higher level, like warn! or error!
                        // For now, let's use info! to ensure visibility, but we can refine this.
                        // Let's use warn! to make it stand out.
                        warn!("[{}_stderr] {}", name, String::from_utf8_lossy(&line));
                    }
                    _ => {}
                }
            }
            // Remove the child from the process manager once it has terminated
            let process_manager = handle.state::<ProcessManager>();
            process_manager
                .active_processes
                .lock()
                .unwrap()
                .remove(&name);
            info!("Removed managed process '{}' from tracking.", name);
        });

        Ok(())
    }

    /// Spawns a process and waits for it to complete, capturing all output.
    /// This is for setup steps or any command where the result is needed before proceeding.
    pub async fn spawn_and_wait_for_process(
        app_handle: &AppHandle<Wry>,
        command: Command,
        process_base_name: &str,
    ) -> Result<CommandResult, String> {
        let process_name = format!("{}_{}", process_base_name, Uuid::new_v4());
        info!("Spawning synchronous managed process: {}", process_name);

        let process_manager = app_handle.state::<ProcessManager>();
        let (mut rx, child) = command.spawn().map_err(|e| {
            let err_msg = format!("Failed to spawn sync process '{}': {}", process_name, e);
            error!("{}", err_msg);
            err_msg
        })?;

        // Add to tracking
        process_manager.active_processes.lock().unwrap().insert(process_name.clone(), child);

        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();
        let mut exit_code = None;
        let mut signal = None;

        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let line_str = String::from_utf8_lossy(&line).to_string();
                    info!("[{}_stdout] {}", process_name, line_str);
                    stdout_lines.push(line_str);
                }
                CommandEvent::Stderr(line) => {
                    let line_str = String::from_utf8_lossy(&line).to_string();
                    error!("[{}_stderr] {}", process_name, line_str);
                    stderr_lines.push(line_str);
                }
                CommandEvent::Terminated(payload) => {
                    info!("Sync process '{}' terminated with code: {:?}, signal: {:?}", process_name, payload.code, payload.signal);
                    exit_code = payload.code;
                    signal = payload.signal;
                    break;
                }
                CommandEvent::Error(e) => {
                    error!("Error from sync process '{}': {}", process_name, e);
                    // Remove from tracking on error
                    process_manager.active_processes.lock().unwrap().remove(&process_name);
                    return Err(format!("Error executing command '{}': {}", process_name, e));
                }
                _ => {}
            }
        }

        // Remove from tracking after completion
        process_manager.active_processes.lock().unwrap().remove(&process_name);
        info!("Removed sync process '{}' from tracking.", process_name);

        Ok(CommandResult {
            exit_code,
            signal,
            stdout: stdout_lines,
            stderr: stderr_lines,
        })
    }


    /// Shuts down all tracked processes gracefully.
    /// This should be called during the application's shutdown sequence.
    /// It sends a kill signal and waits for each process to terminate, with a timeout.
    /// Shuts down all tracked processes gracefully.
    /// This should be called during the application's shutdown sequence.
    /// It sends a kill signal and waits for each process to terminate, with a timeout.
    pub async fn shutdown_all_processes(&self, app_handle: &AppHandle<Wry>) {
        info!("Shutting down all managed processes...");
        let shell = app_handle.shell();

        let processes_to_kill: Vec<(String, u32)> = {
            let processes_map = self.active_processes.lock().unwrap();
            if processes_map.is_empty() {
                info!("No active processes to shut down.");
                return;
            }
            processes_map
                .iter()
                .map(|(name, child)| (name.clone(), child.pid()))
                .collect()
        };

        if !processes_to_kill.is_empty() {
            info!(
                "Terminating processes: {:?}",
                processes_to_kill
                    .iter()
                    .map(|(n, p)| format!("{}({})", n, p))
                    .collect::<Vec<_>>()
            );
        }

        for (name, pid) in processes_to_kill {
            info!("Attempting to kill process: {} (PID: {})", name, pid);
            let kill_command = if cfg!(windows) {
                shell.command("taskkill").args(["/F", "/PID", &pid.to_string()])
            } else {
                shell.command("kill").args(["-9", &pid.to_string()])
            };

            match kill_command.output().await {
                Ok(output) if output.status.success() => {
                    info!(
                        "Successfully sent kill signal to process: {} (PID: {})",
                        name, pid
                    );
                }
                Ok(output) => {
                    error!(
                        "Kill command for process '{}' (PID: {}) failed with status: {:?}. Stderr: {}",
                        name,
                        pid,
                        output.status,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to execute kill command for process '{}' (PID: {}): {}",
                        name, pid, e
                    );
                }
            }

            let wait_for_termination = async {
                let check_interval = Duration::from_millis(100);
                loop {
                    if !self.is_process_running(&name) {
                        info!("Process '{}' terminated successfully.", name);
                        break;
                    }
                    tokio::time::sleep(check_interval).await;
                }
            };

            if let Err(_) = tokio::time::timeout(Duration::from_secs(5), wait_for_termination).await
            {
                error!(
                    "Timeout reached while waiting for process '{}' to terminate. Force-removing from tracking.",
                    name
                );
                self.active_processes.lock().unwrap().remove(&name);
            }
        }

        info!("All tracked processes have been signaled for termination.");
    }
}