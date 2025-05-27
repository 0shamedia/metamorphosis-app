// metamorphosis-app/src-tauri/src/setup_manager/model_events.rs

use serde::Serialize;
use tauri::{AppHandle, Wry, Emitter};
use log::error;
use std::path::PathBuf;

// --- Event Payloads ---

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgressPayload {
    pub model_id: String,
    pub model_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>, // Might not always be available from headers
    pub progress: f32, // Renamed from progress_percentage, 0.0 to 100.0
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadCompletePayload {
    pub model_id: String,
    pub model_name: String,
    pub file_path: PathBuf,
    pub size_bytes: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadFailedPayload {
    pub model_id: String,
    pub model_name: String,
    pub error_message: String,
}

// Internal struct used by model_orchestrator
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OverallModelDownloadProgressInternal {
    pub current_model_index: usize, // 0-based index of the model currently being processed or about to be
    pub total_models: usize,
    pub current_model_id: String,
    pub current_model_name: String,
    pub current_model_progress_percentage: f32, // Progress of the current model
    pub overall_progress_percentage: f32, // Overall progress across all models
}

// Struct that matches the frontend's expected payload for overall progress
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OverallModelDownloadProgressFrontendPayload {
    pub completed_models: usize, // Number of models fully completed
    pub total_models: usize,
    pub progress: f32,       // Overall progress percentage
}


// --- Event Emitter Functions ---

pub fn emit_model_download_progress(
    app_handle: &AppHandle<Wry>,
    payload: ModelDownloadProgressPayload,
) {
    if let Err(e) = app_handle.emit("model-download-progress", payload) {
        error!("Failed to emit model-download-progress event: {}", e);
    }
}

pub fn emit_model_download_complete(
    app_handle: &AppHandle<Wry>,
    payload: ModelDownloadCompletePayload,
) {
    if let Err(e) = app_handle.emit("model-download-complete", payload) {
        error!("Failed to emit model-download-complete event: {}", e);
    }
}

pub fn emit_model_download_failed(
    app_handle: &AppHandle<Wry>,
    payload: ModelDownloadFailedPayload,
) {
    if let Err(e) = app_handle.emit("model-download-failed", payload) {
        error!("Failed to emit model-download-failed event: {}", e);
    }
}

pub fn emit_overall_model_download_progress(
    app_handle: &AppHandle<Wry>,
    internal_payload: OverallModelDownloadProgressInternal, // Changed parameter name and type
) {
    let frontend_payload = OverallModelDownloadProgressFrontendPayload {
        completed_models: internal_payload.current_model_index, // Assuming current_model_index is 0-based count of completed models before current
        total_models: internal_payload.total_models,
        progress: internal_payload.overall_progress_percentage,
    };
    if let Err(e) = app_handle.emit("overall-model-download-progress", frontend_payload) {
        error!("Failed to emit overall-model-download-progress event: {}", e);
    }
}