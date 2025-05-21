// metamorphosis-app/src-tauri/src/setup_manager/event_utils.rs
use tauri::{AppHandle, Wry, Emitter}; // Import Wry, Emitter
use log::error;

use super::types::{CustomNodeCloneFailedPayload, CustomNodePayload, SetupProgressPayload}; // Import from the new types module

// Generic event emitter
pub fn emit_event<S: serde::Serialize + Clone>(
    app_handle: &AppHandle<Wry>,
    event_name: &str,
    payload: Option<S>,
) {
    if let Err(e) = app_handle.emit(event_name, payload) { // Ensure Emitter trait is in scope where this is called
        error!("Failed to emit event '{}': {}", event_name, e);
    }
}
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

// Helper functions for Custom Node Cloning Events

pub fn emit_custom_node_clone_start(app_handle: &AppHandle<Wry>, node_name: &str) {
    let payload = CustomNodePayload {
        node_name: node_name.to_string(),
    };
    if let Err(e) = app_handle.emit("CustomNodeCloneStart", payload) {
        error!("Failed to emit CustomNodeCloneStart event for {}: {}", node_name, e);
    }
}

pub fn emit_custom_node_clone_success(app_handle: &AppHandle<Wry>, node_name: &str) {
    let payload = CustomNodePayload {
        node_name: node_name.to_string(),
    };
    if let Err(e) = app_handle.emit("CustomNodeCloneSuccess", payload) {
        error!("Failed to emit CustomNodeCloneSuccess event for {}: {}", node_name, e);
    }
}

pub fn emit_custom_node_already_exists(app_handle: &AppHandle<Wry>, node_name: &str) {
    let payload = CustomNodePayload {
        node_name: node_name.to_string(),
    };
    if let Err(e) = app_handle.emit("CustomNodeAlreadyExists", payload) {
        error!("Failed to emit CustomNodeAlreadyExists event for {}: {}", node_name, e);
    }
}

pub fn emit_custom_node_clone_failed(app_handle: &AppHandle<Wry>, node_name: &str, error_message: &str) {
    let payload = CustomNodeCloneFailedPayload {
        node_name: node_name.to_string(),
        error: error_message.to_string(),
    };
    if let Err(e) = app_handle.emit("CustomNodeCloneFailed", payload) {
        error!("Failed to emit CustomNodeCloneFailed event for {}: {} - Error: {}", node_name, e, error_message);
    }
}