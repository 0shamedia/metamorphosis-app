// metamorphosis-app/src-tauri/src/setup_manager/dependency_manager/command_runner.rs
// metamorphosis-app/src-tauri/src/setup_manager/dependency_manager/command_runner.rs
use std::path::PathBuf;
use log::{info, error, debug, warn};
use tauri::{AppHandle, Wry, Manager};
use crate::setup_manager::event_utils::emit_setup_progress;
use crate::process_manager::ProcessManager;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use std::fs;
use std::io::Write;
use tauri_plugin_shell::ShellExt;

pub async fn run_command_for_setup_progress(
    app_handle: &AppHandle<Wry>,
    phase: &str,
    current_step_base: &str,
    progress_current_phase: u8,
    progress_weight_of_this_command: u8,
    command_path: &PathBuf,
    args: &[&str],
    current_dir: &PathBuf,
    initial_message: &str,
    error_message_prefix: &str,
) -> Result<u8, String> {
    run_command_for_setup_progress_with_env(
        app_handle,
        phase,
        current_step_base,
        progress_current_phase,
        progress_weight_of_this_command,
        command_path,
        args,
        current_dir,
        initial_message,
        error_message_prefix,
        None,
    ).await
}

pub async fn run_command_for_setup_progress_with_env(
    app_handle: &AppHandle<Wry>,
    phase: &str,
    current_step_base: &str,
    mut progress_current_phase: u8,
    progress_weight_of_this_command: u8,
    command_path: &PathBuf,
    args: &[&str],
    current_dir: &PathBuf,
    initial_message: &str,
    error_message_prefix: &str,
    env_vars: Option<std::collections::HashMap<String, String>>,
) -> Result<u8, String> {
    info!("Executing managed command for setup: {:?} {:?}", command_path, args);
    
    let step_name_initial = format!("{}: {}", current_step_base, initial_message);
    emit_setup_progress(app_handle, phase, &step_name_initial, progress_current_phase, Some(initial_message.to_string()), None);

    let mut cmd = app_handle.shell().command(command_path.to_string_lossy().as_ref()).args(args);
    if let Some(cwd_str) = current_dir.to_str() {
        cmd = cmd.current_dir(cwd_str);
    }

    if let Some(vars) = env_vars {
        cmd = cmd.envs(vars);
    }

    // The env_remove method does not exist on the command builder.
    // The conda environment activation should correctly isolate the python environment,
    // so manually clearing these is not necessary and was causing a compile error.

    let temp_log_dir = app_handle.path().app_data_dir().map_err(|e| format!("Failed to get app data dir for temp logs: {}", e))?.join("temp_command_logs");
    tokio::fs::create_dir_all(&temp_log_dir).await.map_err(|e| format!("Failed to create temp command log directory {}: {}", temp_log_dir.display(), e))?;
    let temp_log_path = temp_log_dir.join(format!("command_output_{}.log", Uuid::new_v4()));
    info!("Temporary command output being written to: {}", temp_log_path.display());

    let result = ProcessManager::spawn_and_wait_for_process(app_handle, cmd, current_step_base).await?;

    // Process stdout
    for line in &result.stdout {
        let line_to_process = line.trim_end().to_string();
        if let Ok(mut writer) = OpenOptions::new().append(true).create(true).open(&temp_log_path).await {
            let _ = writer.write_all(format!("{}\n", line_to_process).as_bytes()).await;
        }
        let lower_line = line_to_process.to_lowercase();
        let is_progress_bar_line = (lower_line.contains('[') && lower_line.contains(']') && lower_line.contains('%')) || (lower_line.contains("downloading") && lower_line.contains("of") && lower_line.contains("mb"));
        let is_spinner_line = line_to_process.chars().all(|c| c == '|' || c == '/' || c == '-' || c == '\\' || c.is_whitespace());
        let is_key_action = lower_line.contains("error:") || lower_line.contains("warning:") || lower_line.contains("fail") || lower_line.contains("failed") || lower_line.contains("nvrtc-builtins64_124.dll") || lower_line.contains("condahttp") || lower_line.contains("connection failed") || lower_line.contains("http ");
        let is_noisy_info = lower_line.contains("looking in indexes:") || lower_line.contains("satisfied constraint") || lower_line.contains("source distribution") || lower_line.contains("cache entry deserialization failed") || lower_line.starts_with("running command ") || is_spinner_line;
        if is_key_action || (!is_noisy_info && !is_progress_bar_line) {
            let dynamic_step_message = if is_key_action { format!("Error during setup: {}", line_to_process) } else { current_step_base.to_string() };
            emit_setup_progress(app_handle, phase, &dynamic_step_message, progress_current_phase, Some(line_to_process), None);
        } else {
            debug!("Filtered (stdout): {}", line_to_process);
        }
    }

    // Process stderr
    for line in &result.stderr {
        let line_to_process = line.trim_end().to_string();
        if let Ok(mut writer) = OpenOptions::new().append(true).create(true).open(&temp_log_path).await {
            let _ = writer.write_all(format!("{}\n", line_to_process).as_bytes()).await;
        }
        let lower_line = line_to_process.to_lowercase();
        let is_progress_bar_line = (lower_line.contains('[') && lower_line.contains(']') && lower_line.contains('%')) || (lower_line.contains("downloading") && lower_line.contains("of") && lower_line.contains("mb"));
        let is_spinner_line = line_to_process.chars().all(|c| c == '|' || c == '/' || c == '-' || c == '\\' || c.is_whitespace());
        let is_pure_progress_artifact = line_to_process.trim().chars().all(|c| c == '[' || c == 'A' || c.is_whitespace()) && line_to_process.len() < 50 && (line_to_process.contains('[') || line_to_process.contains('A'));
        let is_noisy_stderr_info = lower_line.contains("defaulting to user installation") || lower_line.contains("consider adding this directory to path") || (lower_line.starts_with("warning: the script ") && lower_line.contains("is installed in")) || (lower_line.contains("deprecated") && !lower_line.contains("error")) || lower_line.contains("skipping link:") || (lower_line.contains("note:") && !lower_line.contains("error")) || lower_line.contains("running build_ext") || lower_line.contains("running build_py") || lower_line.contains("running egg_info") || lower_line.contains("writing ") || lower_line.contains("copying ") || lower_line.contains("creating ") || is_progress_bar_line || is_spinner_line || is_pure_progress_artifact;
        if lower_line.contains("error:") || (lower_line.contains("warning:") && !is_noisy_stderr_info) || lower_line.contains("nvrtc-builtins64_124.dll") || lower_line.contains("condahttp") || lower_line.contains("connection failed") || lower_line.contains("http ") {
            emit_setup_progress(app_handle, phase, current_step_base, progress_current_phase, Some(line_to_process.clone()), Some(line_to_process));
        } else {
            info!("Filtered/Demoted (stderr): {}", line_to_process);
        }
    }

    let success = result.exit_code.map_or(false, |c| c == 0) && result.signal.is_none();

    if !success {
        let command_string = format!("{:?} {:?}", command_path, args);
        let error_msg = format!("{} failed with exit code: {:?}, signal: {:?}. Command: {}", error_message_prefix, result.exit_code, result.signal, command_string);
        error!("{}", error_msg);
        emit_setup_progress(app_handle, phase, error_message_prefix, progress_current_phase, Some(error_msg.clone()), Some(error_msg.clone()));
        
        if let Ok(content) = tokio::fs::read_to_string(&temp_log_path).await {
            error!("--- Full Command Output (from temp file) ---\n{}\n---------------------------------------------", content);
        }
        let direct_log_message = format!("[CRITICAL_ERROR_DIRECT_WRITE] Command failed: {}. Full output in: {}", error_msg, temp_log_path.display());
        write_to_app_log_direct(app_handle, &direct_log_message);
        return Err(error_msg);
    }
    
    if let Err(e) = tokio::fs::remove_file(&temp_log_path).await {
        warn!("Failed to delete temporary log file {}: {}", temp_log_path.display(), e);
    } else {
        info!("Successfully deleted temporary log file: {}", temp_log_path.display());
    }

    progress_current_phase += progress_weight_of_this_command;
    let success_step_name = format!("{}: Completed successfully.", current_step_base);
    info!("{}", success_step_name);
    emit_setup_progress(app_handle, phase, &success_step_name, progress_current_phase.min(100), None, None);
    Ok(progress_current_phase.min(100))
}

fn write_to_app_log_direct(app_handle: &AppHandle<Wry>, message: &str) {
    if let Ok(app_data_path) = app_handle.path().app_data_dir() {
        let log_file_path = app_data_path.join("logs").join("app.log");
        if let Some(parent_dir) = log_file_path.parent() {
            if !parent_dir.exists() {
                if let Err(e) = fs::create_dir_all(parent_dir) {
                    eprintln!("Failed to create logs directory for direct log write: {}", e);
                    return;
                }
            }
        }
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&log_file_path) {
            if let Err(e) = writeln!(file, "{}", message) {
                eprintln!("Failed to write to app.log directly: {}", e);
            }
            let _ = file.flush();
        }
    }
}