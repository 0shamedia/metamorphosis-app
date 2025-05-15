// metamorphosis-app/src-tauri/src/setup_manager/event_utils.rs
use tauri::{AppHandle, Manager, Wry, Emitter}; // Import Manager, Wry, Emitter
use log::error;

use super::types::SetupProgressPayload; // Import from the new types module

// Helper to emit unified setup progress
pub fn emit_setup_progress(
    app_handle: &AppHandle<Wry>,
    phase: &str,
    current_step: &str,
    progress: u8,
    detail_message: Option<String>,
    error: Option<String>,
) {
    let payload = SetupProgressPayload {
        phase: phase.to_string(),
        current_step: current_step.to_string(),
        progress,
        detail_message,
        error,
    };
    if let Err(e) = app_handle.emit("setup-progress", payload) {
        error!("Failed to emit setup-progress event: {}", e);
    }
}