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
    pub progress_percentage: f32, // 0.0 to 100.0
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

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OverallModelDownloadProgress {
    pub current_model_index: usize,
    pub total_models: usize,
    pub current_model_id: String,
    pub current_model_name: String,
    pub current_model_progress_percentage: f32, // Progress of the current model
    pub overall_progress_percentage: f32, // Overall progress across all models
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
    payload: OverallModelDownloadProgress,
) {
    if let Err(e) = app_handle.emit("overall-model-download-progress", payload) {
        error!("Failed to emit overall-model-download-progress event: {}", e);
    }
}