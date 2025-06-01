// metamorphosis-app/src-tauri/src/setup_manager/dependency_manager/command_runner.rs
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
use log::{info, error, debug};
use tauri::{AppHandle, Wry};
use tokio::process::Command;
use std::process::Stdio;
use tokio::task;
use crate::setup; // For emit_setup_progress
use crate::setup_manager::python_utils::{ // For python path helpers
    get_bundled_python_executable_path,
    get_venv_python_executable_path,
};

// This function executes a command and streams its stdout and stderr,
// logging each line with 'info!' for stdout and 'error!' for stderr.
// The command itself is logged before execution.
// It now emits the new `setup-progress` event.
// This function is intended for internal use within the dependency_manager module.
pub(super) async fn run_command_for_setup_progress(
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
            let line_to_process = line_buf.trim_end().to_string();
            info!("Stdout (setup): {}", line_to_process);
            let lower_line = line_to_process.to_lowercase();

            let mut dynamic_step_message = current_step_base_clone_stdout.clone();
            let mut is_major_package_action = false;

            if lower_line.contains("downloading https") || lower_line.contains("collecting ") {
                if lower_line.contains("torch") && !lower_line.contains("torchvision") && !lower_line.contains("torchaudio") {
                    dynamic_step_message = format!("Downloading PyTorch (base)... - {}", line_to_process.chars().take(60).collect::<String>());
                    is_major_package_action = true;
                } else if lower_line.contains("torchvision") {
                    dynamic_step_message = format!("Downloading Torchvision... - {}", line_to_process.chars().take(60).collect::<String>());
                    is_major_package_action = true;
                } else if lower_line.contains("torchaudio") {
                    dynamic_step_message = format!("Downloading Torchaudio... - {}", line_to_process.chars().take(60).collect::<String>());
                    is_major_package_action = true;
                } else if lower_line.contains("onnxruntime") {
                    dynamic_step_message = format!("Downloading ONNXRuntime... - {}", line_to_process.chars().take(60).collect::<String>());
                    is_major_package_action = true;
                } else if lower_line.contains("insightface") {
                    dynamic_step_message = format!("Downloading InsightFace... - {}", line_to_process.chars().take(60).collect::<String>());
                    is_major_package_action = true;
                }
            } else if lower_line.starts_with("installing collected packages:") {
                 if lower_line.contains("torch") && !lower_line.contains("torchvision") && !lower_line.contains("torchaudio") {
                    dynamic_step_message = "Installing PyTorch (base)...".to_string();
                    is_major_package_action = true;
                } else if lower_line.contains("torchvision") {
                    dynamic_step_message = "Installing Torchvision...".to_string();
                    is_major_package_action = true;
                } else if lower_line.contains("torchaudio") {
                    dynamic_step_message = "Installing Torchaudio...".to_string();
                    is_major_package_action = true;
                } else if lower_line.contains("onnxruntime") {
                    dynamic_step_message = "Installing ONNXRuntime...".to_string();
                    is_major_package_action = true;
                } else if lower_line.contains("insightface") {
                    dynamic_step_message = "Installing InsightFace...".to_string();
                    is_major_package_action = true;
                }
            } else if lower_line.starts_with("successfully installed") {
                 if lower_line.contains("torch") && !lower_line.contains("torchvision") && !lower_line.contains("torchaudio") {
                    dynamic_step_message = "PyTorch (base) installed.".to_string();
                    is_major_package_action = true;
                } else if lower_line.contains("torchvision") {
                    dynamic_step_message = "Torchvision installed.".to_string();
                    is_major_package_action = true;
                } else if lower_line.contains("torchaudio") {
                    dynamic_step_message = "Torchaudio installed.".to_string();
                    is_major_package_action = true;
                } // Can add more for onnx, insightface if needed
            }


            let is_key_action = lower_line.starts_with("collecting ") ||
                                lower_line.starts_with("downloading ") ||
                                lower_line.starts_with("installing collected packages:") ||
                                lower_line.starts_with("successfully installed");

            let is_potentially_noisy_info =
                lower_line.contains("looking in indexes:") ||
                lower_line.contains("satisfied constraint") ||
                lower_line.contains("source distribution") ||
                lower_line.contains("cache entry deserialization failed") ||
                lower_line.starts_with("running command ") ||
                (lower_line.starts_with("  ") && !is_key_action && !is_major_package_action); // Be more aggressive filtering indented lines unless key

            let should_emit_detail = is_key_action || !is_potentially_noisy_info || is_major_package_action;

            if should_emit_detail || lower_line.contains("error:") || lower_line.contains("warning:") {
                setup::emit_setup_progress(
                    &app_handle_clone_stdout,
                    &phase_clone_stdout,
                    &dynamic_step_message, // Use the dynamic or base step message
                    progress_current_phase,
                    Some(line_to_process),
                    None
                );
            } else {
                 debug!("Filtered (stdout): {}", line_to_process);
            }
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
            let line_to_process = line_buf.trim_end().to_string();
            error!("Stderr (setup): {}", line_to_process); // Log all stderr from backend as error
            let lower_line = line_to_process.to_lowercase();

            // Filter common non-error stderr messages from pip/builds
            let is_ignorable_stderr_info = lower_line.contains("defaulting to user installation") ||
                                           lower_line.contains("consider adding this directory to path") ||
                                           (lower_line.starts_with("warning: the script ") && lower_line.contains("is installed in")) ||
                                           (lower_line.starts_with("warning:") && lower_line.contains(" βρίσκεται ")) || // Greek path warning
                                           lower_line.contains("deprecated") || // Deprecation warnings unless it also says "error"
                                           lower_line.contains("skipping link:") ||
                                           (lower_line.contains("note:") && !lower_line.contains("error")) || // General notes unless error
                                           (lower_line.contains("warning:") && !lower_line.contains("error")) || // General warnings unless error
                                           lower_line.contains("running build_ext") ||
                                           lower_line.contains("running build_py") ||
                                           lower_line.contains("running egg_info") ||
                                           lower_line.contains("writing ") ||
                                           lower_line.contains("copying ") ||
                                           lower_line.contains("creating ");

            if !is_ignorable_stderr_info || lower_line.contains("error:") { // Always show lines containing "error:"
                // Emit as detail message and error string, keeping the current_step_base as the main step title
                setup::emit_setup_progress(
                    &app_handle_clone_stderr,
                    &phase_clone_stderr,
                    &current_step_base_clone_stderr, // Use the original step base
                    progress_current_phase,
                    Some(line_to_process.clone()), // Detail message
                    Some(line_to_process) // Error message
                );
            } else {
                 info!("Filtered/Demoted (stderr): {}", line_to_process); // Log ignorable stderr as info
            }
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