// metamorphosis-app/src-tauri/src/sidecar_manager/event_utils.rs

use tauri::{AppHandle, Emitter, Manager, Wry};
use log::error;
use serde_json::json;

pub const COMFYUI_PORT: u16 = 8188; // TODO: Make this configurable

// Helper to emit backend status
pub fn emit_backend_status(app_handle: &AppHandle<Wry>, status: &str, message: String, is_error: bool) {
    if let Err(e) = app_handle.emit("backend-status", json!({
        "status": status,
        "message": message,
        "isError": is_error,
    })) {
        error!("Failed to emit backend status event: {}", e);
    }
}